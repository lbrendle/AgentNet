#!/usr/bin/env python3
from __future__ import annotations

import argparse
import base64
import json
import os
import subprocess
import sys
import tempfile
import time
import uuid
from pathlib import Path
from typing import Optional

try:
    import tomllib
except Exception:  # pragma: no cover
    import tomli as tomllib  # type: ignore

from cryptography.hazmat.primitives.asymmetric import ed25519
from cryptography.hazmat.primitives.serialization import Encoding, PublicFormat

from agentnet_py.agentmail import AgentMailMessagePayload, build_agentmail_message


DEFAULT_TOPIC = "agentnet/mail/1.0.0"
DEFAULT_PAYLOAD_TYPE = 1000


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Send AgentMail via agentmesh publish.")
    parser.add_argument("--config", type=Path, required=True, help="agentmesh.toml path")
    parser.add_argument("--agent-key", type=Path, required=True, help="ed25519 key (base64)")
    parser.add_argument("--agent-did", default=None)
    parser.add_argument("--to", action="append", required=True, help="recipient DID (repeatable)")
    parser.add_argument("--subject", default=None)
    parser.add_argument("--thread-id", default=None)
    parser.add_argument("--reply-to", default=None)
    parser.add_argument("--markdown", default=None)
    parser.add_argument("--markdown-file", type=Path, default=None)
    parser.add_argument("--expires-sec", type=int, default=None)
    parser.add_argument("--voucher-hex", default=None)
    parser.add_argument("--voucher-file", type=Path, default=None)
    parser.add_argument("--agentmesh-bin", type=Path, default=None)
    parser.add_argument("--preconnect-seconds", type=int, default=30)
    parser.add_argument("--settle-seconds", type=int, default=20)
    parser.add_argument("--print-payload", action="store_true")
    return parser.parse_args()


def read_b64_key(path: Path) -> bytes:
    data = path.read_text().strip()
    if not data:
        raise SystemExit(f"[agentmail-send] key file empty: {path}")
    raw = base64.b64decode(data)
    if len(raw) != 32:
        raise SystemExit(f"[agentmail-send] key must be 32 bytes: {path}")
    return raw


def build_agent_did(secret_key: bytes) -> str:
    priv = ed25519.Ed25519PrivateKey.from_private_bytes(secret_key)
    pub = priv.public_key().public_bytes(Encoding.Raw, PublicFormat.Raw)
    return "did:anet:agent:" + base64.b64encode(pub).decode("ascii")


def load_agentmail_config(config_path: Path) -> tuple[str, int]:
    data = tomllib.loads(config_path.read_text())
    agentmail = data.get("agentmail", {}) if isinstance(data, dict) else {}
    topic = agentmail.get("topic", DEFAULT_TOPIC)
    payload_type = agentmail.get("payload_type", DEFAULT_PAYLOAD_TYPE)
    return str(topic), int(payload_type)


def resolve_markdown(args: argparse.Namespace) -> str:
    if args.markdown is not None and args.markdown_file is not None:
        raise SystemExit("[agentmail-send] use --markdown or --markdown-file, not both")
    if args.markdown_file is not None:
        return args.markdown_file.read_text()
    if args.markdown is None:
        raise SystemExit("[agentmail-send] markdown content required")
    return args.markdown


def resolve_voucher(args: argparse.Namespace) -> Optional[str]:
    if args.voucher_hex and args.voucher_file:
        raise SystemExit("[agentmail-send] use --voucher-hex or --voucher-file, not both")
    if args.voucher_file:
        return args.voucher_file.read_text().strip() or None
    return args.voucher_hex


def main() -> None:
    args = parse_args()
    secret_key = read_b64_key(args.agent_key.expanduser())
    agent_did = args.agent_did or build_agent_did(secret_key)

    markdown = resolve_markdown(args)
    ts = int(time.time())
    expires = ts + args.expires_sec if args.expires_sec else None

    payload = AgentMailMessagePayload(
        version=1,
        message_id=str(uuid.uuid4()),
        sender=agent_did,
        recipients=list(args.to),
        thread_id=args.thread_id,
        reply_to=args.reply_to,
        subject=args.subject,
        markdown=markdown,
        attachments=None,
        intent_hashes=None,
        receipt_hashes=None,
        metadata=None,
        ts=ts,
        expires=expires,
    )
    message_bytes = build_agentmail_message(payload, secret_key)

    topic, payload_type = load_agentmail_config(args.config.expanduser())

    if args.print_payload:
        print(json.dumps(
            {
                "topic": topic,
                "payload_type": payload_type,
                "sender": agent_did,
                "recipients": list(args.to),
                "subject": args.subject,
                "ts": ts,
                "expires": expires,
            },
            ensure_ascii=True,
        ))

    voucher_hex = resolve_voucher(args)
    agentmesh_bin = args.agentmesh_bin.expanduser() if args.agentmesh_bin else Path("agentmesh")

    with tempfile.NamedTemporaryFile(delete=False) as tmp:
        tmp.write(message_bytes)
        tmp_path = Path(tmp.name)

    try:
        cmd = [
            str(agentmesh_bin),
            "publish",
            "--config",
            str(args.config.expanduser()),
            "--topic",
            topic,
            "--payload-type",
            str(payload_type),
            "--payload-cbor",
            str(tmp_path),
            "--preconnect-seconds",
            str(args.preconnect_seconds),
            "--settle-seconds",
            str(args.settle_seconds),
        ]
        if voucher_hex:
            cmd.extend(["--proof-voucher-hex", voucher_hex])

        subprocess.run(cmd, check=True)
    finally:
        try:
            os.unlink(tmp_path)
        except Exception:
            pass


if __name__ == "__main__":
    main()
