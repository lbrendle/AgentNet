#!/usr/bin/env python3
from __future__ import annotations

import argparse
import base64
import hashlib
import json
import os
import secrets
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path
from typing import Optional

try:
    import cbor2  # type: ignore
except Exception as exc:  # pragma: no cover
    print(f"[agent-onboard] missing dependency: {exc}", file=sys.stderr)
    print("[agent-onboard] install with: python -m pip install cbor2", file=sys.stderr)
    sys.exit(1)

try:
    from cryptography.hazmat.primitives.asymmetric import ed25519, x25519
    from cryptography.hazmat.primitives.serialization import (
        Encoding,
        PrivateFormat,
        PublicFormat,
        NoEncryption,
    )
except Exception as exc:  # pragma: no cover
    print(f"[agent-onboard] cryptography unavailable: {exc}", file=sys.stderr)
    sys.exit(1)


def canonical_cbor(value: object) -> bytes:
    return cbor2.dumps(value, canonical=True)


def b64_encode(data: bytes) -> str:
    return base64.b64encode(data).decode("ascii")


def b64_decode(text: str) -> bytes:
    return base64.b64decode(text)


def sha256(data: bytes) -> bytes:
    return hashlib.sha256(data).digest()


def read_base64_key(path: Path) -> bytes:
    data = path.read_text().strip()
    if not data:
        raise ValueError(f"key file empty: {path}")
    raw = b64_decode(data)
    if len(raw) != 32:
        raise ValueError(f"key must be 32 bytes: {path}")
    return raw


def write_key(path: Path, raw: bytes) -> None:
    path.write_text(b64_encode(raw) + "\n")
    try:
        os.chmod(path, 0o600)
    except Exception:
        pass


def write_bytes(path: Path, raw: bytes) -> None:
    path.write_bytes(raw)


def write_text(path: Path, text: str) -> None:
    path.write_text(text + "\n")


def build_did(pubkey: bytes) -> str:
    return f"did:anet:agent:{b64_encode(pubkey)}"


def build_agentmesh_config(
    *,
    chain_id: str,
    agent_did: str,
    key_path: Path,
    state_dir: Path,
    bootstrap: str,
    enable_dht: bool,
    capabilities: list[str],
    agent_record_ttl: int,
    publish_interval: int,
    enable_agentmail: bool,
) -> str:
    lines = [
        f'chain_id = "{chain_id}"',
        f'agent_did = "{agent_did}"',
        f'key_path = "{key_path}"',
        f'state_dir = "{state_dir}"',
        'listen_addrs = ["/ip4/0.0.0.0/tcp/0/ws"]',
        f'bootstrap = ["{bootstrap}"]',
        'protocols = ["agentnet/handshake/1.0.0", "agentnet/dht/1.0.0", "agentnet/pubsub/1.0.0"]',
        'transports = ["ws"]',
        'roles = ["agent"]',
        "",
        "[pubsub]",
        'topics = ["agentnet/main/1.0.0"]',
        "",
    ]
    if enable_dht:
        lines.extend(
            [
                "[dht]",
                "enabled = true",
                f"publish_interval_sec = {publish_interval}",
                "",
                "[dht.agent_record]",
                f'record_key = "agentnet/agent/{agent_did}"',
                f"expires_sec = {agent_record_ttl}",
                f"capabilities = [{', '.join(f'\"{cap}\"' for cap in capabilities)}]",
                "",
            ]
        )
    if enable_agentmail:
        lines.extend(
            [
                "[agentmail]",
                "enabled = true",
                "",
            ]
        )
    return "\n".join(lines)


def fetch_mesh_info(index_url: str) -> Optional[dict]:
    try:
        with urllib.request.urlopen(index_url.rstrip("/") + "/mesh/info") as resp:
            if resp.status != 200:
                return None
            return json.loads(resp.read().decode("utf-8"))
    except Exception:
        return None


def request_json(method: str, url: str, payload: Optional[dict] = None, api_key: Optional[str] = None) -> dict:
    data = None
    headers = {"Content-Type": "application/json"}
    if api_key:
        headers["Authorization"] = f"Bearer {api_key}"
    if payload is not None:
        data = json.dumps(payload).encode("utf-8")
    req = urllib.request.Request(url, data=data, headers=headers, method=method)
    try:
        with urllib.request.urlopen(req) as resp:
            body = resp.read()
            if not body:
                return {}
            return json.loads(body.decode("utf-8"))
    except urllib.error.HTTPError as exc:
        body = exc.read().decode("utf-8", errors="replace")
        raise SystemExit(f"[agent-onboard] {method} {url} failed: {exc.code} {body}")
    except urllib.error.URLError as exc:
        raise SystemExit(f"[agent-onboard] {method} {url} failed: {exc}")


def load_claim_api_key(args: argparse.Namespace) -> Optional[str]:
    if args.claim_api_key and args.claim_api_key_file:
        raise SystemExit("[agent-onboard] use --claim-api-key or --claim-api-key-file, not both")
    if args.claim_api_key_file:
        key = args.claim_api_key_file.read_text().strip()
        return key or None
    return args.claim_api_key


def claim_voucher(args: argparse.Namespace, agent_did: str) -> tuple[Optional[str], dict]:
    if not args.claim_service_url:
        return None, {}
    api_key = load_claim_api_key(args)
    base = args.claim_service_url.rstrip("/")
    payload = {"agent_did": agent_did}
    if args.x_handle:
        payload["x_handle"] = args.x_handle
    claim = request_json("POST", f"{base}/v1/claims", payload, api_key=api_key)
    claim_id = claim.get("claim_id")
    if not claim_id:
        raise SystemExit("[agent-onboard] claim service did not return claim_id")
    required_post = claim.get("required_post")
    if required_post:
        print(f"[agent-onboard] post on X: {required_post}", file=sys.stderr)
    claim_url = f"{base}/v1/claims/{claim_id}"
    claim["claim_url"] = claim_url
    if args.claim_wait_sec <= 0:
        return None, claim
    deadline = time.time() + args.claim_wait_sec
    while time.time() < deadline:
        status = request_json("GET", claim_url, api_key=api_key)
        if status.get("status") == "issued" and status.get("voucher_hex"):
            return status["voucher_hex"], status
        time.sleep(max(1, int(args.claim_poll_sec)))
    raise SystemExit("[agent-onboard] claim voucher not issued within wait window")


def build_bootstrap(mesh_info: dict) -> Optional[str]:
    if not isinstance(mesh_info, dict):
        return None
    peer_id = mesh_info.get("peer_id")
    public_ws = mesh_info.get("public_ws")
    if not peer_id or not public_ws:
        return None
    if public_ws.startswith("wss://"):
        host = public_ws[len("wss://") :]
        return f"/dns4/{host}/tcp/443/wss/p2p/{peer_id}"
    if public_ws.startswith("ws://"):
        host = public_ws[len("ws://") :]
        return f"/dns4/{host}/tcp/80/ws/p2p/{peer_id}"
    return None


def main() -> None:
    parser = argparse.ArgumentParser(description="Onboard a personal AgentNet agent.")
    parser.add_argument("--out-dir", type=Path, required=True)
    parser.add_argument("--issuer-key", type=Path)
    parser.add_argument("--issuer-did")
    parser.add_argument("--claim-service-url", default=None)
    parser.add_argument("--x-handle", default=None)
    parser.add_argument("--claim-api-key", default=None)
    parser.add_argument("--claim-api-key-file", type=Path, default=None)
    parser.add_argument("--claim-wait-sec", type=int, default=600)
    parser.add_argument("--claim-poll-sec", type=int, default=10)
    parser.add_argument("--chain-id", default="agentnet-mainnet-1")
    parser.add_argument("--index-url", default="https://agentindex-mainnet.onrender.com")
    parser.add_argument("--bootstrap", default=None)
    parser.add_argument("--currency", default="ANET")
    parser.add_argument("--amount", type=int, default=1)
    parser.add_argument("--purpose", default="agentnet/main/1.0.0")
    parser.add_argument("--voucher-ttl-sec", type=int, default=3600)
    parser.add_argument("--fee", type=int, default=0)
    parser.add_argument("--nonce", type=int, default=None)
    parser.add_argument("--enable-dht", action="store_true")
    parser.add_argument("--capability", action="append", default=[])
    parser.add_argument("--agent-record-ttl-sec", type=int, default=86400)
    parser.add_argument("--publish-interval-sec", type=int, default=600)
    parser.add_argument("--enable-agentmail", action="store_true")
    args = parser.parse_args()

    if not args.claim_service_url:
        if not args.issuer_key or not args.issuer_did:
            raise SystemExit("[agent-onboard] --issuer-key and --issuer-did required without claim service")

    out_dir: Path = args.out_dir.expanduser().resolve()
    out_dir.mkdir(parents=True, exist_ok=True)
    state_dir = out_dir / "state"
    state_dir.mkdir(parents=True, exist_ok=True)

    ed_key_path = out_dir / "agent.ed25519.key"
    x_key_path = out_dir / "agent.x25519.key"

    if ed_key_path.exists():
        ed_secret = read_base64_key(ed_key_path)
        ed_private = ed25519.Ed25519PrivateKey.from_private_bytes(ed_secret)
    else:
        ed_private = ed25519.Ed25519PrivateKey.generate()
        ed_secret = ed_private.private_bytes(
            encoding=Encoding.Raw,
            format=PrivateFormat.Raw,
            encryption_algorithm=NoEncryption(),
        )
        write_key(ed_key_path, ed_secret)

    ed_public = ed_private.public_key().public_bytes(
        encoding=Encoding.Raw,
        format=PublicFormat.Raw,
    )

    if x_key_path.exists():
        x_secret = read_base64_key(x_key_path)
        x_private = x25519.X25519PrivateKey.from_private_bytes(x_secret)
    else:
        x_private = x25519.X25519PrivateKey.generate()
        x_secret = x_private.private_bytes(
            encoding=Encoding.Raw,
            format=PrivateFormat.Raw,
            encryption_algorithm=NoEncryption(),
        )
        write_key(x_key_path, x_secret)

    x_public = x_private.public_key().public_bytes(
        encoding=Encoding.Raw,
        format=PublicFormat.Raw,
    )

    agent_did = build_did(ed_public)
    write_text(out_dir / "agent.did", agent_did)
    write_text(out_dir / "agent.ed25519.pub.hex", ed_public.hex())
    write_text(out_dir / "agent.x25519.pub.hex", x_public.hex())

    created = int(time.time())
    identity_payload = {
        0: agent_did,
        1: ed_public,
        2: x_public,
        3: created,
    }
    identity_cbor = canonical_cbor(identity_payload)
    write_bytes(out_dir / "identity-register.cbor", identity_cbor)

    tx_nonce = args.nonce if args.nonce is not None else secrets.randbits(63)
    tx_payload = {
        0: 10,
        1: agent_did,
        2: tx_nonce,
        3: args.fee,
        4: identity_payload,
    }
    tx_payload_cbor = canonical_cbor(tx_payload)
    tx_hash = sha256(tx_payload_cbor)
    tx_sig = ed_private.sign(tx_hash)
    tx_map = dict(tx_payload)
    tx_map[5] = tx_sig
    tx_cbor = canonical_cbor(tx_map)
    write_bytes(out_dir / "identity-register-tx.cbor", tx_cbor)
    write_text(out_dir / "identity-register-tx.hex", tx_cbor.hex())

    voucher_hex = None
    claim_info: dict = {}
    if args.claim_service_url:
        voucher_hex, claim_info = claim_voucher(args, agent_did)
        if claim_info:
            write_text(out_dir / "claim.json", json.dumps(claim_info, indent=2))
        if voucher_hex:
            voucher_bytes = bytes.fromhex(voucher_hex)
            write_bytes(out_dir / "voucher.cbor", voucher_bytes)
            write_text(out_dir / "voucher.hex", voucher_hex)
    else:
        issuer_secret = read_base64_key(args.issuer_key)
        issuer_private = ed25519.Ed25519PrivateKey.from_private_bytes(issuer_secret)
        now = int(time.time())
        exp = now + int(args.voucher_ttl_sec)
        nonce = secrets.token_bytes(16)
        voucher_payload = {
            0: args.issuer_did,
            1: agent_did,
            2: args.amount,
            3: args.currency,
            4: args.purpose,
            5: now,
            6: exp,
            7: nonce,
        }
        voucher_payload_cbor = canonical_cbor(voucher_payload)
        voucher_hash = sha256(voucher_payload_cbor)
        voucher_sig = issuer_private.sign(voucher_hash)
        voucher_map = dict(voucher_payload)
        voucher_map[8] = voucher_sig
        voucher_cbor = canonical_cbor(voucher_map)
        voucher_hex = voucher_cbor.hex()
        write_bytes(out_dir / "voucher.cbor", voucher_cbor)
        write_text(out_dir / "voucher.hex", voucher_hex)

    bootstrap = args.bootstrap
    if not bootstrap:
        mesh_info = fetch_mesh_info(args.index_url)
        bootstrap = build_bootstrap(mesh_info) if mesh_info else None
    if not bootstrap:
        raise SystemExit("[agent-onboard] bootstrap multiaddr not available")

    if args.enable_dht and not args.capability:
        raise SystemExit("[agent-onboard] --enable-dht requires at least one --capability")

    config_text = build_agentmesh_config(
        chain_id=args.chain_id,
        agent_did=agent_did,
        key_path=ed_key_path,
        state_dir=state_dir,
        bootstrap=bootstrap,
        enable_dht=args.enable_dht,
        capabilities=args.capability,
        agent_record_ttl=args.agent_record_ttl_sec,
        publish_interval=args.publish_interval_sec,
        enable_agentmail=args.enable_agentmail,
    )
    config_path = out_dir / "agentmesh.toml"
    write_text(config_path, config_text)

    summary = {
        "agent_did": agent_did,
        "ed25519_key": str(ed_key_path),
        "x25519_key": str(x_key_path),
        "identity_tx_cbor": str(out_dir / "identity-register-tx.cbor"),
        "voucher_hex": str(out_dir / "voucher.hex") if voucher_hex else None,
        "claim_info": str(out_dir / "claim.json") if claim_info else None,
        "claim_service_url": args.claim_service_url,
        "agentmesh_config": str(config_path),
        "bootstrap": bootstrap,
    }
    write_text(out_dir / "onboard.json", json.dumps(summary, indent=2))

    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
