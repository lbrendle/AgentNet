#!/usr/bin/env python3
from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path


def fail(message: str) -> None:
    print(f"[agentindex-entrypoint] {message}", file=sys.stderr)
    sys.exit(1)


def env_required(name: str) -> str:
    value = os.getenv(name)
    if value is None or value.strip() == "":
        fail(f"missing required env: {name}")
    return expand_port(value.strip())


def env_optional(name: str) -> str | None:
    value = os.getenv(name)
    if value is None:
        return None
    value = expand_port(value.strip())
    return value if value else None


def main() -> None:
    bind = env_required("AGENTINDEX_BIND")
    db_path = Path(env_required("AGENTINDEX_DB"))
    db_path.parent.mkdir(parents=True, exist_ok=True)

    args = ["/usr/local/bin/agentindex", "--bind", bind, "--db", str(db_path)]

    identity_state = env_optional("AGENTINDEX_IDENTITY_STATE")
    skill_state = env_optional("AGENTINDEX_SKILL_STATE")
    work_state = env_optional("AGENTINDEX_WORK_STATE")
    if identity_state:
        args.extend(["--identity_state", identity_state])
    if skill_state:
        args.extend(["--skill_registry_state", skill_state])
    if work_state:
        args.extend(["--work_registry_state", work_state])

    subprocess.run(args, check=True)


def expand_port(value: str) -> str:
    if "$PORT" not in value and "${PORT}" not in value:
        return value
    port = os.getenv("PORT")
    if port is None or port.strip() == "":
        fail("PORT must be set when using $PORT placeholders")
    return value.replace("${PORT}", port.strip()).replace("$PORT", port.strip())


if __name__ == "__main__":
    main()
