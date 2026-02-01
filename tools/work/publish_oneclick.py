#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
import secrets
import sys
import tempfile
import time
from pathlib import Path
from typing import Optional
from urllib.request import urlopen

ROOT = Path(__file__).resolve().parents[2]

sys.path.insert(0, str(ROOT / "impl" / "python"))

try:
    from agentnet_py.work import (
        WorkOfferPayload,
        WorkOfferPublishPayload,
        build_work_offer,
        work_offer_publish_payload_to_cbor,
    )
    from agentnet_py.tx import TxEnvelopePayload, build_tx_envelope
except Exception as exc:  # pragma: no cover
    raise SystemExit(f"[work-offer] missing agentnet_py deps: {exc}")

DEFAULT_INDEX_URL = "https://agentindex-mainnet.onrender.com"
DEFAULT_TOPIC = "agentnet/main/1.0.0"
DEFAULT_CHAIN_ID = "agentnet-mainnet-1"
DEFAULT_TX_PAYLOAD_TYPE = 2000
TX_WORK_OFFER_PUBLISH = 50


def fail(message: str) -> None:
    print(f"[work-offer] {message}", file=sys.stderr)
    sys.exit(1)


def read_text(path: Path) -> str:
    data = path.read_text().strip()
    if not data:
        fail(f"empty file: {path}")
    return data


def read_b64_key(path: Path) -> bytes:
    data = read_text(path)
    import base64

    try:
        decoded = base64.b64decode(data)
    except Exception as exc:
        raise SystemExit(f"[work-offer] invalid base64 key: {exc}")
    if len(decoded) != 32:
        raise SystemExit("[work-offer] key must be 32 bytes")
    return decoded


def fetch_mesh_info(index_url: str) -> dict:
    url = f"{index_url.rstrip('/')}/mesh/info"
    try:
        with urlopen(url) as resp:
            if resp.status != 200:
                fail(f"mesh info returned {resp.status}")
            return json.loads(resp.read().decode("utf-8"))
    except Exception as exc:
        fail(f"mesh info fetch failed: {exc}")
    return {}


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


def choose_agentmesh_bin(arg: Optional[Path]) -> Path:
    if arg:
        return arg.expanduser()
    candidate = ROOT / "impl" / "rust" / "target" / "debug" / "agentmesh"
    if candidate.exists():
        return candidate
    return Path("agentmesh")


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


def sanitize_pocket(pocket: str) -> str:
    slug = pocket.strip().lower()
    if not slug:
        return ""
    for ch in slug:
        if not (ch.isalnum() or ch == "-"):
            fail("pocket slug must contain only letters, numbers, or dashes")
    return slug


def main() -> None:
    parser = argparse.ArgumentParser(description="Publish a signed work offer to AgentNet (one-click).")
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

    parser.add_argument("--pocket", default=None, help="pocket slug (letters, numbers, dashes)")
    parser.add_argument("--offer-id", default=None)
    parser.add_argument("--title", required=True)
    parser.add_argument("--summary", required=True)
    parser.add_argument("--scope", required=True)
    parser.add_argument("--budget-amount", type=int, required=True)
    parser.add_argument("--budget-currency", required=True)
    parser.add_argument("--duration-sec", type=int)
    parser.add_argument("--duration-days", type=int)
    parser.add_argument("--deliverable", action="append", default=[])
    parser.add_argument("--requirement", action="append", default=[])
    parser.add_argument("--ttl-sec", type=int, default=60 * 60 * 24 * 7)
    parser.add_argument("--fee", type=int, default=0)
    parser.add_argument("--print-tx", action="store_true")
    args = parser.parse_args()

    if not args.deliverable:
        fail("at least one --deliverable is required")

    if args.duration_sec is None and args.duration_days is None:
        fail("provide --duration-sec or --duration-days")
    if args.duration_sec is not None and args.duration_days is not None:
        fail("use --duration-sec or --duration-days, not both")

    duration_sec = args.duration_sec if args.duration_sec is not None else args.duration_days * 86400

    agent_dir = args.agent_dir.expanduser().resolve() if args.agent_dir else None
    agent_did, agent_key_path = resolve_agent(agent_dir, args.agent_key, args.agent_did)

    pocket_slug = sanitize_pocket(args.pocket) if args.pocket else ""
    scope = args.scope.strip()
    if pocket_slug:
        scope = f"pocket:{pocket_slug} :: {scope}"

    offer_id = args.offer_id
    if not offer_id:
        offer_id = f"offer:{agent_did}:{int(time.time())}:{secrets.token_hex(4)}"

    voucher_hex = resolve_voucher(agent_dir, args.voucher_file, args.voucher_hex)
    if voucher_hex is None:
        print("[work-offer] voucher missing; publish will likely be rejected on mainnet.", file=sys.stderr)

    mesh_info = fetch_mesh_info(args.index_url)
    bootstrap = build_bootstrap(mesh_info)

    config_text = build_agentmesh_toml(
        agent_did=agent_did,
        key_path=agent_key_path.expanduser().resolve(),
        bootstrap=bootstrap,
        topic=args.topic,
        chain_id=args.chain_id,
    )

    now = int(time.time())
    exp = now + int(args.ttl_sec)

    offer_payload = WorkOfferPayload(
        offer_id=offer_id,
        issuer=agent_did,
        title=args.title.strip(),
        summary=args.summary.strip(),
        scope=scope,
        budget_amount=args.budget_amount,
        budget_currency=args.budget_currency.strip(),
        duration_sec=duration_sec,
        deliverables=[d.strip() for d in args.deliverable if d.strip()],
        requirements=[r.strip() for r in args.requirement if r.strip()] or None,
        ts=now,
        exp=exp,
    )

    secret_key = read_b64_key(agent_key_path.expanduser())
    offer_bytes = build_work_offer(offer_payload, secret_key)
    publish_payload = WorkOfferPublishPayload(offer=offer_bytes, ts=now)
    publish_cbor = work_offer_publish_payload_to_cbor(publish_payload)

    nonce = int(time.time() * 1000)
    tx_payload = TxEnvelopePayload(
        tx_type=TX_WORK_OFFER_PUBLISH,
        sender=agent_did,
        nonce=nonce,
        fee=args.fee,
        payload=publish_cbor,
    )
    tx_cbor = build_tx_envelope(tx_payload, secret_key)

    if args.print_tx:
        print(json.dumps({
            "offer_id": offer_id,
            "topic": args.topic,
            "payload_type": DEFAULT_TX_PAYLOAD_TYPE,
            "sender": agent_did,
            "nonce": nonce,
            "ts": now,
        }, indent=2))

    with tempfile.NamedTemporaryFile(delete=False) as tmp:
        tmp.write(tx_cbor)
        tmp_path = Path(tmp.name)

    try:
        cmd = [
            str(choose_agentmesh_bin(args.agentmesh_bin)),
            "publish",
            "--config",
            str(tmp_path.with_suffix(".toml")),
        ]
        tmp_cfg = tmp_path.with_suffix(".toml")
        tmp_cfg.write_text(config_text)

        cmd.extend([
            "--topic",
            args.topic,
            "--payload-type",
            str(DEFAULT_TX_PAYLOAD_TYPE),
            "--payload-cbor",
            str(tmp_path),
            "--preconnect-seconds",
            str(args.preconnect_seconds),
            "--settle-seconds",
            str(args.settle_seconds),
        ])
        if voucher_hex:
            cmd.extend(["--proof-voucher-hex", voucher_hex])
        import subprocess

        subprocess.run(cmd, check=True)
    finally:
        try:
            os.unlink(tmp_path)
        except Exception:
            pass
        try:
            os.unlink(str(tmp_path.with_suffix(".toml")))
        except Exception:
            pass


if __name__ == "__main__":
    main()
