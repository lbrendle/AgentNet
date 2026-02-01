#!/usr/bin/env python3
from __future__ import annotations

import argparse
import base64
import json
import sys
import urllib.error
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools" / "app-manifest"))

from app_manifest_lib import AppManifestError, build_manifest_from_app  # type: ignore


def read_base64_key(path: Path) -> bytes:
    data = path.read_text().strip()
    if not data:
        raise SystemExit(f"[experience] key file empty: {path}")
    raw = base64.b64decode(data)
    if len(raw) != 32:
        raise SystemExit(f"[experience] key must be 32 bytes: {path}")
    return raw


def derive_public_key_hex(secret: bytes) -> str:
    try:
        from cryptography.hazmat.primitives.asymmetric import ed25519
        from cryptography.hazmat.primitives.serialization import Encoding, PublicFormat
    except Exception as exc:  # pragma: no cover
        raise SystemExit(f"[experience] cryptography unavailable: {exc}")
    private = ed25519.Ed25519PrivateKey.from_private_bytes(secret)
    public_key = private.public_key().public_bytes(Encoding.Raw, PublicFormat.Raw)
    return public_key.hex()


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
        raise SystemExit(f"[experience] {method} {url} failed: {exc.code} {body}")
    except urllib.error.URLError as exc:
        raise SystemExit(f"[experience] {method} {url} failed: {exc}")


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Compile APP.md into a signed Skill Manifest and publish as an AgentNet experience."
    )
    parser.add_argument("--app", type=Path, required=True, help="APP.md path")
    parser.add_argument("--agent-key", type=Path, required=True, help="Agent ed25519 key (base64)")
    parser.add_argument("--agent-did", required=True, help="Agent DID")
    parser.add_argument("--artifact-root", type=Path, default=None)
    parser.add_argument("--out-dir", type=Path, default=None)
    parser.add_argument(
        "--index-url",
        default="https://agentindex-mainnet.onrender.com",
    )
    parser.add_argument("--publish", action="store_true", default=True)
    args = parser.parse_args()

    app_path = args.app.expanduser().resolve()
    if not app_path.exists():
        raise SystemExit(f"[experience] APP.md not found: {app_path}")

    out_dir = args.out_dir.expanduser().resolve() if args.out_dir else app_path.parent / "dist" / "agentnet-apps"
    artifact_root = args.artifact_root.expanduser().resolve() if args.artifact_root else app_path.parent

    try:
        result = build_manifest_from_app(
            app_path=app_path,
            agent_key=args.agent_key.expanduser(),
            agent_did=args.agent_did,
            out_dir=out_dir,
            artifact_root=artifact_root,
        )
    except AppManifestError as exc:
        raise SystemExit(f"[experience] {exc}") from exc

    summary = result["summary"]
    manifest_bytes = result["manifest_bytes"]

    if args.publish:
        secret = read_base64_key(args.agent_key.expanduser())
        public_key_hex = derive_public_key_hex(secret)
        payload = {"cbor_hex": manifest_bytes.hex(), "public_key_hex": public_key_hex}
        request_json(
            "POST",
            args.index_url.rstrip("/") + "/ingest/experience_manifest",
            payload,
        )
        print(json.dumps({"status": "published", **summary}, indent=2))
        return

    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
