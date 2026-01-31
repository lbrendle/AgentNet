#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import os
import sys
import time
from pathlib import Path
from urllib import request


def fail(message: str) -> None:
    print(f"[index-sync] {message}", file=sys.stderr)
    sys.exit(1)


def env_required(name: str) -> str:
    value = os.getenv(name)
    if value is None or value.strip() == "":
        fail(f"missing required env: {name}")
    return value.strip()


def env_int(name: str, default: int) -> int:
    value = os.getenv(name)
    if value is None or value.strip() == "":
        return default
    try:
        return int(value)
    except ValueError:
        fail(f"invalid integer for {name}: {value}")
        return default


def read_file(path: Path) -> bytes:
    return path.read_bytes()


def hash_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def post_json(url: str, payload: dict) -> None:
    data = json.dumps(payload).encode("utf-8")
    req = request.Request(url, data=data, headers={"Content-Type": "application/json"})
    with request.urlopen(req, timeout=15) as resp:
        if resp.status < 200 or resp.status >= 300:
            raise RuntimeError(f"unexpected status {resp.status}")


def sync_loop(base_url: str, state_dir: Path, interval: int, retry_sec: int, max_backoff: int) -> None:
    identity_path = state_dir / "identity_registry.json"
    skill_path = state_dir / "skill_registry.json"
    work_path = state_dir / "work_registry.json"
    mesh_info_path = state_dir / "mesh_info.json"

    last_hashes: dict[str, str] = {}
    backoff = retry_sec

    while True:
        try:
            for name, path, endpoint in (
                ("identity", identity_path, "/ingest/identity_state"),
                ("skill", skill_path, "/ingest/skill_registry_state"),
                ("work", work_path, "/ingest/work_registry_state"),
            ):
                if not path.exists():
                    continue
                data = read_file(path)
                digest = hash_bytes(data)
                if last_hashes.get(name) == digest:
                    continue
                payload = {"json": data.decode("utf-8")}
                post_json(f"{base_url}{endpoint}", payload)
                last_hashes[name] = digest
                print(f"[index-sync] synced {name} state")
            if mesh_info_path.exists():
                data = read_file(mesh_info_path)
                digest = hash_bytes(data)
                if last_hashes.get("mesh_info") != digest:
                    try:
                        payload = json.loads(data.decode("utf-8"))
                    except json.JSONDecodeError as exc:
                        raise RuntimeError(f"invalid mesh_info.json: {exc}") from exc
                    post_json(f\"{base_url}/ingest/mesh_info\", payload)
                    last_hashes[\"mesh_info\"] = digest
                    print(\"[index-sync] synced mesh info\")
            backoff = retry_sec
            time.sleep(interval)
        except Exception as exc:  # pylint: disable=broad-except
            print(f"[index-sync] sync error: {exc}")
            time.sleep(backoff)
            backoff = min(max_backoff, backoff * 2)


def main() -> None:
    base_url = env_required("AGENTINDEX_URL").rstrip("/")
    state_dir = Path(env_required("AGENTMESH_STATE_DIR"))
    interval = env_int("AGENTINDEX_SYNC_INTERVAL_SEC", 10)
    retry_sec = env_int("AGENTINDEX_SYNC_RETRY_SEC", 5)
    max_backoff = env_int("AGENTINDEX_SYNC_MAX_BACKOFF_SEC", 60)

    if not state_dir.exists():
        fail(f"state dir not found: {state_dir}")

    sync_loop(base_url, state_dir, interval, retry_sec, max_backoff)


if __name__ == "__main__":
    main()
