#!/usr/bin/env python3
from __future__ import annotations

import os
import sys
from pathlib import Path

import uvicorn


def ensure_parent(path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)


def main() -> None:
    db_path = os.getenv("ANET_CLAIM_DB_PATH", "/var/lib/agentnet/agentclaim/claims.sqlite")
    os.environ["ANET_CLAIM_DB_PATH"] = db_path
    ensure_parent(Path(db_path))

    key_b64 = os.getenv("ANET_VOUCHER_ISSUER_KEY_B64")
    key_path = os.getenv("ANET_VOUCHER_ISSUER_KEY_PATH")
    if key_b64:
        if not key_path:
            key_path = "/var/lib/agentnet/agentclaim/issuer.key"
            os.environ["ANET_VOUCHER_ISSUER_KEY_PATH"] = key_path
        key_file = Path(key_path)
        ensure_parent(key_file)
        key_file.write_text(key_b64.strip() + "\n")
        try:
            os.chmod(key_file, 0o600)
        except Exception:
            pass

    port = int(os.getenv("PORT", "8080"))
    uvicorn.run("agentclaim_app:app", host="0.0.0.0", port=port)


if __name__ == "__main__":
    try:
        main()
    except Exception as exc:
        print(f"[agentclaim] startup failure: {exc}", file=sys.stderr)
        sys.exit(1)
