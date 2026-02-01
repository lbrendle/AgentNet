#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Optional
from urllib.request import urlopen

ROOT = Path(__file__).resolve().parents[2]

DEFAULT_INDEX_URL = "https://agentindex-mainnet.onrender.com"
DEFAULT_TOPIC = "agentnet/main/1.0.0"
DEFAULT_CHAIN_ID = "agentnet-mainnet-1"
DEFAULT_TX_PAYLOAD_TYPE = 2000


def fail(message: str) -> None:
    print(f"[oneclick] {message}", file=sys.stderr)
    sys.exit(1)


def read_text(path: Path) -> str:
    data = path.read_text().strip()
    if not data:
        fail(f"empty file: {path}")
    return data


def fetch_mesh_info(index_url: str) -> dict:
    url = f"{index_url.rstrip('/')}/mesh/info"
    try:
        with urlopen(url) as resp:
            if resp.status != 200:
                fail(f"mesh info returned {resp.status}")
            return json.loads(resp.read().decode("utf-8"))
    except Exception as exc:
        fail(f"mesh info fetch failed: {exc}")


def build_bootstrap(mesh_info: dict) -> str:
    peer_id = mesh_info.get("peer_id")
    public_ws = mesh_info.get("public_ws")
    if not peer_id or not public_ws:
        fail("mesh info missing peer_id or public_ws")
    if public_ws.startswith("wss://"):
        host = public_ws[len("wss://") :]
        return f"/dns4/{host}/tcp/443/wss/p2p/{peer_id}"
    if public_ws.startswith("ws://"):
        host = public_ws[len("ws://") :]
        return f"/dns4/{host}/tcp/80/ws/p2p/{peer_id}"
    fail("unsupported public_ws scheme")
    return ""


def build_agentmesh_toml(agent_did: str, key_path: Path, bootstrap: str, topic: str, chain_id: str) -> str:
    lines = [
        f"chain_id = \"{chain_id}\"",
        f"agent_did = \"{agent_did}\"",
        f"key_path = \"{key_path}\"",
        "listen_addrs = [\"/ip4/0.0.0.0/tcp/0/ws\"]",
        f"bootstrap = [\"{bootstrap}\"]",
        'protocols = ["agentnet/handshake/1.0.0", "agentnet/dht/1.0.0", "agentnet/pubsub/1.0.0"]',
        'transports = ["ws"]',
        'roles = ["mesh"]',
        "",
        "[pubsub]",
        f"topics = [\"{topic}\"]",
        "require_economic_proof = false",
        "verify_signatures = true",
        "",
        "[tx]",
        f"pubsub_payload_type = {DEFAULT_TX_PAYLOAD_TYPE}",
        "",
    ]
    return "\n".join(lines)


def resolve_voucher(agent_dir: Optional[Path], voucher_file: Optional[Path], voucher_hex: Optional[str]) -> Optional[str]:
    if voucher_file and voucher_hex:
        fail("use --voucher-file or --voucher-hex, not both")
    if voucher_file:
        return read_text(voucher_file)
    if voucher_hex:
        return voucher_hex.strip()
    if agent_dir:
        candidate = agent_dir / "voucher.hex"
        if candidate.exists():
            return read_text(candidate)
    return None


def resolve_agent(agent_dir: Optional[Path], agent_key: Optional[Path], agent_did: Optional[str]) -> tuple[str, Path]:
    if agent_dir:
        did_path = agent_dir / "agent.did"
        key_path = agent_dir / "agent.ed25519.key"
        if not did_path.exists() or not key_path.exists():
            fail(f"agent dir missing agent.did or agent.ed25519.key: {agent_dir}")
        return read_text(did_path), key_path
    if agent_key and agent_did:
        return agent_did, agent_key
    fail("provide --agent-dir or both --agent-key and --agent-did")
    return "", Path(".")


def main() -> None:
    parser = argparse.ArgumentParser(description="One-click publish for AgentNet APP.md.")
    parser.add_argument("--app", type=Path, help="APP.md path")
    parser.add_argument("--manifest", type=Path, help="prebuilt manifest cbor")
    parser.add_argument("--agent-dir", type=Path, help="agent directory containing agent.did + agent.ed25519.key")
    parser.add_argument("--agent-key", type=Path, help="agent ed25519 key (base64)")
    parser.add_argument("--agent-did", help="agent DID")
    parser.add_argument("--voucher-file", type=Path, help="voucher.hex path")
    parser.add_argument("--voucher-hex", help="voucher hex string")
    parser.add_argument("--index-url", default=DEFAULT_INDEX_URL)
    parser.add_argument("--topic", default=DEFAULT_TOPIC)
    parser.add_argument("--chain-id", default=DEFAULT_CHAIN_ID)
    parser.add_argument("--preconnect-seconds", type=int, default=10)
    parser.add_argument("--settle-seconds", type=int, default=10)
    parser.add_argument("--agentmesh-bin", type=Path, default=None)
    parser.add_argument("--print-tx", action="store_true")
    args = parser.parse_args()

    if not args.app and not args.manifest:
        fail("provide --app or --manifest")

    agent_dir = args.agent_dir.expanduser().resolve() if args.agent_dir else None
    agent_did, agent_key_path = resolve_agent(agent_dir, args.agent_key, args.agent_did)

    mesh_info = fetch_mesh_info(args.index_url)
    bootstrap = build_bootstrap(mesh_info)

    toml_text = build_agentmesh_toml(agent_did, agent_key_path.expanduser().resolve(), bootstrap, args.topic, args.chain_id)

    voucher_hex = resolve_voucher(agent_dir, args.voucher_file, args.voucher_hex)
    if voucher_hex is None:
        print("[oneclick] voucher missing; publish will likely be rejected on mainnet. Provide --voucher-file or run x_pair/onboard to obtain one.", file=sys.stderr)

    with tempfile.NamedTemporaryFile("w", delete=False, suffix=".toml") as handle:
        handle.write(toml_text)
        config_path = Path(handle.name)

    try:
        cmd = [
            sys.executable,
            str(ROOT / "tools" / "app-manifest" / "publish_app_manifest.py"),
            "--config",
            str(config_path),
            "--agent-key",
            str(agent_key_path),
            "--agent-did",
            agent_did,
            "--preconnect-seconds",
            str(args.preconnect_seconds),
            "--settle-seconds",
            str(args.settle_seconds),
        ]
        if args.manifest:
            cmd.extend(["--manifest", str(args.manifest.expanduser().resolve())])
        else:
            cmd.extend(["--app", str(args.app.expanduser().resolve())])
        if voucher_hex:
            cmd.extend(["--voucher-hex", voucher_hex])
        if args.agentmesh_bin:
            cmd.extend(["--agentmesh-bin", str(args.agentmesh_bin.expanduser().resolve())])
        if args.print_tx:
            cmd.append("--print-tx")
        subprocess.run(cmd, check=True)
    finally:
        try:
            os.unlink(config_path)
        except OSError:
            pass


if __name__ == "__main__":
    main()
