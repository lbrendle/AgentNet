#!/usr/bin/env python3
from __future__ import annotations

import argparse
import base64
import json
import os
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path
from typing import Optional

try:
    import tomllib
except Exception:  # pragma: no cover
    import tomli as tomllib  # type: ignore

try:
    from cryptography.hazmat.primitives.asymmetric import ed25519
    from cryptography.hazmat.primitives.serialization import Encoding, PublicFormat
except Exception:  # pragma: no cover
    ed25519 = None
    Encoding = None
    PublicFormat = None


class ClaimError(RuntimeError):
    pass


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
        raise ClaimError(f"{method} {url} failed: {exc.code} {body}")
    except urllib.error.URLError as exc:
        raise ClaimError(f"{method} {url} failed: {exc}")


def load_api_key(args: argparse.Namespace) -> Optional[str]:
    if args.claim_api_key and args.claim_api_key_file:
        raise ClaimError("use --claim-api-key or --claim-api-key-file, not both")
    if args.claim_api_key_file:
        key = args.claim_api_key_file.read_text().strip()
        return key or None
    if args.claim_api_key:
        return args.claim_api_key
    return (
        os.getenv("AGENTNET_CLAIM_API_KEY")
        or os.getenv("ANET_CLAIM_API_KEY")
        or None
    )


def load_claim_service_url(args: argparse.Namespace, agent_dir: Optional[Path]) -> str:
    if args.claim_service_url:
        return args.claim_service_url
    env_url = os.getenv("AGENTNET_CLAIM_URL") or os.getenv("ANET_CLAIM_URL")
    if env_url:
        return env_url
    if agent_dir:
        onboard_path = agent_dir / "onboard.json"
        if onboard_path.exists():
            data = json.loads(onboard_path.read_text())
            if isinstance(data, dict):
                url = data.get("claim_service_url")
                if url:
                    return url
    return "https://agentclaim-mainnet.onrender.com"


def build_did(pubkey: bytes) -> str:
    return "did:anet:agent:" + base64.b64encode(pubkey).decode("ascii")


def did_from_secret_key(secret: bytes) -> str:
    if ed25519 is None or Encoding is None or PublicFormat is None:
        raise ClaimError("cryptography required to derive DID from agent key")
    priv = ed25519.Ed25519PrivateKey.from_private_bytes(secret)
    pub = priv.public_key().public_bytes(Encoding.Raw, PublicFormat.Raw)
    return build_did(pub)


def load_agent_did(args: argparse.Namespace, agent_dir: Optional[Path]) -> str:
    if args.agent_did:
        return args.agent_did
    if agent_dir:
        did_path = agent_dir / "agent.did"
        if did_path.exists():
            return did_path.read_text().strip()
        mesh_path = agent_dir / "agentmesh.toml"
        if mesh_path.exists():
            data = tomllib.loads(mesh_path.read_text())
            agent_did = data.get("agent_did") if isinstance(data, dict) else None
            if agent_did:
                return str(agent_did)
    if args.agent_key:
        raw = base64.b64decode(args.agent_key.read_text().strip())
        if len(raw) != 32:
            raise ClaimError("agent key must be 32 bytes")
        return did_from_secret_key(raw)
    raise ClaimError("agent DID not provided; use --agent-did, --agent-dir, or --agent-key")


def write_json(path: Path, payload: dict) -> None:
    path.write_text(json.dumps(payload, indent=2) + "\n")


def main() -> None:
    parser = argparse.ArgumentParser(description="Initiate an X.com claim pairing for an agent.")
    parser.add_argument("--claim-service-url", default=None)
    parser.add_argument("--agent-did", default=None)
    parser.add_argument("--agent-dir", type=Path, default=None)
    parser.add_argument("--agent-key", type=Path, default=None)
    parser.add_argument("--x-handle", default=None)
    parser.add_argument("--claim-api-key", default=None)
    parser.add_argument("--claim-api-key-file", type=Path, default=None)
    parser.add_argument("--wait-sec", type=int, default=600)
    parser.add_argument("--poll-sec", type=int, default=10)
    parser.add_argument("--out-dir", type=Path, default=None)
    args = parser.parse_args()

    agent_dir = args.agent_dir.expanduser().resolve() if args.agent_dir else None
    api_key = load_api_key(args)
    claim_url = load_claim_service_url(args, agent_dir)
    agent_did = load_agent_did(args, agent_dir)

    base = claim_url.rstrip("/")
    payload = {"agent_did": agent_did}
    if args.x_handle:
        payload["x_handle"] = args.x_handle

    claim = request_json("POST", f"{base}/v1/claims", payload, api_key=api_key)
    claim_id = claim.get("claim_id")
    if not claim_id:
        raise SystemExit("[x-pair] claim service did not return claim_id")

    claim_url = f"{base}/v1/claims/{claim_id}"
    claim["claim_url"] = claim_url

    required_post = claim.get("required_post")
    if required_post:
        print(f"[x-pair] required X post: {required_post}", file=sys.stderr)

    out_dir = args.out_dir.expanduser().resolve() if args.out_dir else (agent_dir if agent_dir else None)
    if out_dir:
        out_dir.mkdir(parents=True, exist_ok=True)
        write_json(out_dir / "claim.json", claim)

    if args.wait_sec <= 0:
        print(json.dumps(claim, indent=2))
        return

    deadline = time.time() + args.wait_sec
    while time.time() < deadline:
        status = request_json("GET", claim_url, api_key=api_key)
        if status.get("status") == "issued" and status.get("voucher_hex"):
            if out_dir:
                (out_dir / "voucher.hex").write_text(status["voucher_hex"] + "\n")
                write_json(out_dir / "claim-issued.json", status)
            print(json.dumps(status, indent=2))
            return
        time.sleep(max(1, int(args.poll_sec)))

    raise SystemExit("[x-pair] claim voucher not issued within wait window")


if __name__ == "__main__":
    main()
