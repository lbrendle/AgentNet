#!/usr/bin/env python3
from __future__ import annotations

import argparse
import base64
import hashlib
import json
import os
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

try:
    import cbor2  # type: ignore
except Exception as exc:  # pragma: no cover
    print(f"[agent-profile] missing dependency: {exc}", file=sys.stderr)
    print("[agent-profile] install with: python -m pip install cbor2", file=sys.stderr)
    sys.exit(1)

try:
    from cryptography.hazmat.primitives.asymmetric import ed25519
    from cryptography.hazmat.primitives.serialization import (
        Encoding,
        PrivateFormat,
        PublicFormat,
        NoEncryption,
    )
except Exception as exc:  # pragma: no cover
    print(f"[agent-profile] cryptography unavailable: {exc}", file=sys.stderr)
    sys.exit(1)


def canonical_cbor(value: object) -> bytes:
    return cbor2.dumps(value, canonical=True)


def sha256(data: bytes) -> bytes:
    return hashlib.sha256(data).digest()


def b64_decode(text: str) -> bytes:
    return base64.b64decode(text)


def read_base64_key(path: Path) -> bytes:
    data = path.read_text().strip()
    if not data:
        raise ValueError(f"key file empty: {path}")
    raw = b64_decode(data)
    if len(raw) != 32:
        raise ValueError(f"key must be 32 bytes: {path}")
    return raw


def write_bytes(path: Path, raw: bytes) -> None:
    path.write_bytes(raw)


def write_text(path: Path, text: str) -> None:
    path.write_text(text + "\n")


def request_json(method: str, url: str, payload: dict) -> dict:
    data = json.dumps(payload).encode("utf-8")
    req = urllib.request.Request(url, data=data, method=method)
    req.add_header("Content-Type", "application/json")
    try:
        with urllib.request.urlopen(req) as resp:
            body = resp.read()
            if not body:
                return {}
            return json.loads(body.decode("utf-8"))
    except urllib.error.HTTPError as exc:
        body = exc.read().decode("utf-8", errors="replace")
        raise SystemExit(f"[agent-profile] {method} {url} failed: {exc.code} {body}")
    except urllib.error.URLError as exc:
        raise SystemExit(f"[agent-profile] {method} {url} failed: {exc}")


def main() -> None:
    parser = argparse.ArgumentParser(description="Publish an AgentNet directory profile.")
    parser.add_argument("--agent-key", type=Path, required=True)
    parser.add_argument("--agent-did", required=True)
    parser.add_argument("--display-name", required=True)
    parser.add_argument("--summary", required=True)
    parser.add_argument("--tag", action="append", default=[])
    parser.add_argument("--capability", action="append", default=[])
    parser.add_argument("--link", action="append", default=[])
    parser.add_argument("--visibility", choices=["private", "public"], default="private")
    parser.add_argument("--expires-sec", type=int, default=604800)
    parser.add_argument("--out-dir", type=Path, required=True)
    parser.add_argument(
        "--index-url",
        default="https://agentindex-mainnet.onrender.com",
    )
    parser.add_argument("--publish", action="store_true")
    args = parser.parse_args()

    if not args.display_name.strip():
        raise SystemExit("[agent-profile] display-name required")
    if not args.summary.strip():
        raise SystemExit("[agent-profile] summary required")

    out_dir: Path = args.out_dir.expanduser().resolve()
    out_dir.mkdir(parents=True, exist_ok=True)

    secret = read_base64_key(args.agent_key)
    private = ed25519.Ed25519PrivateKey.from_private_bytes(secret)

    now = int(time.time())
    exp = now + int(args.expires_sec)
    visibility = 1 if args.visibility == "public" else 0

    payload = {
        0: args.agent_did,
        1: args.display_name.strip(),
        2: args.summary.strip(),
        3: args.tag,
        4: args.capability,
        6: visibility,
        7: exp,
    }
    if args.link:
        payload[5] = args.link

    payload_cbor = canonical_cbor(payload)
    payload_hash = sha256(payload_cbor)
    signature = private.sign(payload_hash)
    record = dict(payload)
    record[8] = signature
    record_cbor = canonical_cbor(record)

    write_bytes(out_dir / "agent-profile.cbor", record_cbor)
    write_text(out_dir / "agent-profile.hex", record_cbor.hex())

    summary = {
        "agent_did": args.agent_did,
        "display_name": args.display_name.strip(),
        "summary": args.summary.strip(),
        "visibility": args.visibility,
        "expires": exp,
        "agent_profile_cbor": str(out_dir / "agent-profile.cbor"),
        "agent_profile_hex": str(out_dir / "agent-profile.hex"),
    }
    write_text(out_dir / "agent-profile.json", json.dumps(summary, indent=2))

    if args.publish:
        payload = {
            "cbor_hex": record_cbor.hex(),
        }
        request_json(
            "POST",
            args.index_url.rstrip("/") + "/ingest/agent_profile",
            payload,
        )
        print(json.dumps({"status": "published", **summary}, indent=2))
        return

    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
