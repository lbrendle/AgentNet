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
from pathlib import Path
from typing import Optional

try:
    import tomllib
except Exception:  # pragma: no cover
    import tomli as tomllib  # type: ignore

from cryptography.hazmat.primitives.asymmetric import ed25519
from cryptography.hazmat.primitives.serialization import Encoding, PublicFormat

from agentnet_py.agentmail import AgentMailMessagePayload, build_agentmail_message, decode_agentmail_message

DEFAULT_TOPIC = "agentnet/mail/1.0.0"
DEFAULT_PAYLOAD_TYPE = 1000


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Bridge AgentNet AgentMail to OpenClaw agent turns.")
    parser.add_argument("--state-dir", type=Path, required=True, help="AgentNet agent state dir")
    parser.add_argument("--agent-key", type=Path, required=True, help="ed25519 key (base64)")
    parser.add_argument("--agent-did", default=None, help="Agent DID (optional)")
    parser.add_argument("--agentmesh-config", type=Path, required=True, help="agentmesh.toml path")
    parser.add_argument("--agentmesh-bin", type=Path, default=None, help="agentmesh binary path")
    parser.add_argument("--openclaw-agent", default="main", help="OpenClaw agent id")
    parser.add_argument("--openclaw-bin", default="openclaw", help="OpenClaw CLI binary")
    parser.add_argument("--poll-interval-sec", type=float, default=0.5)
    parser.add_argument("--cursor-file", type=Path, default=None)
    parser.add_argument("--once", action="store_true", help="Process current inbox and exit")
    parser.add_argument("--reply-prefix", default="Re:", help="Subject prefix for replies")
    parser.add_argument("--timeout-sec", type=int, default=120)
    return parser.parse_args()


def read_b64_key(path: Path) -> bytes:
    data = path.read_text().strip()
    if not data:
        raise SystemExit(f"[bridge] key file empty: {path}")
    raw = base64.b64decode(data)
    if len(raw) != 32:
        raise SystemExit(f"[bridge] key must be 32 bytes: {path}")
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


def resolve_inbox(state_dir: Path) -> Path:
    return state_dir.expanduser() / "agentmail" / "inbox.log"


def resolve_cursor(inbox: Path, cursor: Optional[Path]) -> Path:
    if cursor is not None:
        return cursor.expanduser()
    return inbox.with_suffix(inbox.suffix + ".bridge.offset")


def load_cursor(path: Path) -> int:
    if not path.exists():
        return 0
    raw = path.read_text().strip()
    if not raw:
        return 0
    try:
        return int(raw)
    except ValueError:
        return 0


def write_cursor(path: Path, offset: int) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(str(offset))


def extract_reply(payload: object) -> Optional[str]:
    if isinstance(payload, str):
        return payload
    if isinstance(payload, dict):
        for key in ("reply", "message", "response", "output", "text", "content", "assistant"):
            value = payload.get(key)
            if isinstance(value, str) and value.strip():
                return value.strip()
        for nested_key in ("result", "data", "body"):
            nested = payload.get(nested_key)
            if isinstance(nested, dict):
                for key in ("reply", "message", "response", "output", "text", "content"):
                    value = nested.get(key)
                    if isinstance(value, str) and value.strip():
                        return value.strip()
    return None


def run_openclaw(openclaw_bin: str, agent_id: str, message: str, timeout_sec: int) -> str:
    cmd = [openclaw_bin, "agent", "--agent", agent_id, "--message", message, "--json", "--timeout", str(timeout_sec)]
    proc = subprocess.run(cmd, capture_output=True, text=True)
    if proc.returncode != 0:
        raise RuntimeError(proc.stderr.strip() or proc.stdout.strip() or "OpenClaw agent failed")
    stdout = proc.stdout.strip()
    if not stdout:
        raise RuntimeError("OpenClaw agent returned empty response")
    # Ignore any leading warnings (e.g., node deprecation) before JSON
    start = stdout.find("{")
    if start == -1:
        return stdout
    try:
        data = json.loads(stdout[start:])
        reply = extract_reply(data)
        return reply or stdout[start:]
    except json.JSONDecodeError:
        return stdout


def send_agentmail(
    agentmesh_bin: Path,
    config: Path,
    secret_key: bytes,
    agent_did: str,
    to: str,
    subject: Optional[str],
    thread_id: Optional[str],
    reply_to: Optional[str],
    markdown: str,
) -> None:
    ts = int(time.time())
    payload = AgentMailMessagePayload(
        version=1,
        message_id=str(ts) + "-" + os.urandom(6).hex(),
        sender=agent_did,
        recipients=[to],
        thread_id=thread_id,
        reply_to=reply_to,
        subject=subject,
        markdown=markdown,
        attachments=None,
        intent_hashes=None,
        receipt_hashes=None,
        metadata=None,
        ts=ts,
        expires=None,
    )
    message_bytes = build_agentmail_message(payload, secret_key)
    topic, payload_type = load_agentmail_config(config)

    with tempfile.NamedTemporaryFile(delete=False) as tmp:
        tmp.write(message_bytes)
        tmp_path = Path(tmp.name)

    try:
        cmd = [
            str(agentmesh_bin),
            "publish",
            "--config",
            str(config),
            "--topic",
            topic,
            "--payload-type",
            str(payload_type),
            "--payload-cbor",
            str(tmp_path),
            "--preconnect-seconds",
            "30",
            "--settle-seconds",
            "20",
        ]
        subprocess.run(cmd, check=True)
    finally:
        try:
            os.unlink(tmp_path)
        except Exception:
            pass


def main() -> None:
    args = parse_args()
    state_dir = args.state_dir.expanduser()
    inbox = resolve_inbox(state_dir)
    cursor_path = resolve_cursor(inbox, args.cursor_file)
    inbox.parent.mkdir(parents=True, exist_ok=True)
    if not inbox.exists():
        inbox.touch()

    secret_key = read_b64_key(args.agent_key.expanduser())
    agent_did = args.agent_did or build_agent_did(secret_key)
    agentmesh_bin = args.agentmesh_bin.expanduser() if args.agentmesh_bin else Path("agentmesh")

    offset = load_cursor(cursor_path)
    poll = max(0.05, float(args.poll_interval_sec))

    with inbox.open("rb") as handle:
        if offset > 0:
            handle.seek(offset)

        while True:
            start = handle.tell()
            header = handle.read(4)
            if len(header) < 4:
                if args.once:
                    break
                handle.seek(start)
                time.sleep(poll)
                continue
            length = int.from_bytes(header, "big")
            data = handle.read(length)
            if len(data) < length:
                handle.seek(start)
                if args.once:
                    break
                time.sleep(poll)
                continue

            try:
                msg = decode_agentmail_message(data)
                payload = msg.payload
                if payload.sender == agent_did:
                    offset = handle.tell()
                    write_cursor(cursor_path, offset)
                    continue
                if agent_did not in payload.recipients:
                    offset = handle.tell()
                    write_cursor(cursor_path, offset)
                    continue

                subject = payload.subject or ""
                prompt = payload.markdown.strip()
                if subject:
                    prompt = f"{subject}\n\n{prompt}"

                reply_text = run_openclaw(args.openclaw_bin, args.openclaw_agent, prompt, args.timeout_sec)
                reply_subject = None
                if payload.subject:
                    reply_subject = f"{args.reply_prefix} {payload.subject}".strip()

                send_agentmail(
                    agentmesh_bin=agentmesh_bin,
                    config=args.agentmesh_config.expanduser(),
                    secret_key=secret_key,
                    agent_did=agent_did,
                    to=payload.sender,
                    subject=reply_subject,
                    thread_id=payload.thread_id,
                    reply_to=payload.message_id,
                    markdown=reply_text,
                )

                print(json.dumps({"status": "replied", "message_id": payload.message_id}, ensure_ascii=True))
            except Exception as exc:
                print(
                    json.dumps({"status": "error", "message": str(exc), "offset": start}, ensure_ascii=True),
                    file=sys.stderr,
                )

            offset = handle.tell()
            write_cursor(cursor_path, offset)


if __name__ == "__main__":
    main()
