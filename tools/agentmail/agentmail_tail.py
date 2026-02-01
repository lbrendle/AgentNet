#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
import sys
import time
from pathlib import Path
from typing import Optional

from agentnet_py.agentmail import decode_agentmail_message, verify_agentmail_message


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Tail AgentMail inbox log and emit JSONL.")
    parser.add_argument("--inbox", type=Path, default=None)
    parser.add_argument("--state-dir", type=Path, default=None)
    parser.add_argument("--cursor-file", type=Path, default=None)
    parser.add_argument("--follow", action="store_true")
    parser.add_argument("--poll-interval-sec", type=float, default=0.5)
    parser.add_argument("--verify-pubkey-hex", default=None)
    parser.add_argument("--include-raw", action="store_true")
    return parser.parse_args()


def resolve_inbox(state_dir: Optional[Path], inbox: Optional[Path]) -> Path:
    if inbox is not None:
        return inbox.expanduser()
    if state_dir is not None:
        return state_dir.expanduser() / "agentmail" / "inbox.log"
    default = Path.home() / ".agentnet-secrets" / "agents" / "personal" / "state" / "agentmail" / "inbox.log"
    return default


def resolve_cursor(inbox: Path, cursor: Optional[Path]) -> Path:
    if cursor is not None:
        return cursor.expanduser()
    return inbox.with_suffix(inbox.suffix + ".offset")


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


def decode_record(data: bytes, verify_pubkey_hex: Optional[str]) -> dict:
    if verify_pubkey_hex:
        pubkey = bytes.fromhex(verify_pubkey_hex)
        payload = verify_agentmail_message(data, pubkey)
        msg = decode_agentmail_message(data)
    else:
        msg = decode_agentmail_message(data)
        payload = msg.payload

    attachments = None
    if payload.attachments is not None:
        attachments = [
            {
                "content_hash_hex": att.content_hash.hex(),
                "size_bytes": att.size_bytes,
                "mime": att.mime,
                "retrieval": att.retrieval,
            }
            for att in payload.attachments
        ]

    return {
        "message_id": payload.message_id,
        "sender": payload.sender,
        "recipients": payload.recipients,
        "thread_id": payload.thread_id,
        "reply_to": payload.reply_to,
        "subject": payload.subject,
        "markdown": payload.markdown,
        "attachments": attachments,
        "intent_hashes_hex": None if payload.intent_hashes is None else [h.hex() for h in payload.intent_hashes],
        "receipt_hashes_hex": None if payload.receipt_hashes is None else [h.hex() for h in payload.receipt_hashes],
        "metadata": payload.metadata,
        "ts": payload.ts,
        "expires": payload.expires,
        "signature_hex": msg.signature.hex(),
    }


def main() -> None:
    args = parse_args()
    inbox = resolve_inbox(args.state_dir, args.inbox)
    cursor_path = resolve_cursor(inbox, args.cursor_file)

    inbox.parent.mkdir(parents=True, exist_ok=True)
    if not inbox.exists():
        inbox.touch()

    offset = load_cursor(cursor_path)
    poll = max(0.05, float(args.poll_interval_sec))

    with inbox.open("rb") as handle:
        if offset > 0:
            handle.seek(offset)

        while True:
            start = handle.tell()
            header = handle.read(4)
            if len(header) < 4:
                if not args.follow:
                    break
                handle.seek(start)
                time.sleep(poll)
                continue
            length = int.from_bytes(header, "big")
            data = handle.read(length)
            if len(data) < length:
                handle.seek(start)
                if not args.follow:
                    break
                time.sleep(poll)
                continue

            try:
                record = decode_record(data, args.verify_pubkey_hex)
                if args.include_raw:
                    record["raw_cbor_hex"] = data.hex()
                print(json.dumps(record, ensure_ascii=True))
            except Exception as exc:
                error = {
                    "error": "decode_failed",
                    "message": str(exc),
                    "offset": start,
                    "length": length,
                }
                print(json.dumps(error, ensure_ascii=True), file=sys.stderr)

            offset = handle.tell()
            write_cursor(cursor_path, offset)


if __name__ == "__main__":
    main()
