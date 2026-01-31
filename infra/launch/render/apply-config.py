#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path
from urllib.error import HTTPError
from urllib.request import Request, urlopen

BASE_URL = "https://api.render.com/v1"


def fail(message: str) -> None:
    print(f"[render-apply] {message}", file=sys.stderr)
    sys.exit(1)


def read_key(path: Path) -> str:
    if not path.exists():
        fail(f"api key file not found: {path}")
    key = path.read_text().strip()
    if not key:
        fail(f"api key file empty: {path}")
    return key


def request(
    method: str, path: str, token: str, payload: object | None = None
) -> object | None:
    url = f"{BASE_URL}{path}"
    data = None
    headers = {
        "Authorization": f"Bearer {token}",
        "Content-Type": "application/json",
    }
    if payload is not None:
        data = json.dumps(payload).encode("utf-8")
    req = Request(url, data=data, method=method, headers=headers)
    try:
        with urlopen(req) as resp:
            body = resp.read()
            if not body:
                return None
            return json.loads(body.decode("utf-8"))
    except HTTPError as exc:
        body = exc.read().decode("utf-8", errors="replace")
        fail(f"{method} {path} failed: {exc.code} {body}")
    return None


def request_allow_disk_exists(
    method: str, path: str, token: str, payload: object | None = None
) -> object | None:
    url = f"{BASE_URL}{path}"
    data = None
    headers = {
        "Authorization": f"Bearer {token}",
        "Content-Type": "application/json",
    }
    if payload is not None:
        data = json.dumps(payload).encode("utf-8")
    req = Request(url, data=data, method=method, headers=headers)
    try:
        with urlopen(req) as resp:
            body = resp.read()
            if not body:
                return None
            return json.loads(body.decode("utf-8"))
    except HTTPError as exc:
        body = exc.read().decode("utf-8", errors="replace")
        if exc.code == 400 and "disk already exists" in body.lower():
            return None
        fail(f"{method} {path} failed: {exc.code} {body}")
    return None


def find_service(services: list[dict], name: str) -> dict:
    normalized = []
    for item in services:
        if isinstance(item, dict) and "service" in item and isinstance(item["service"], dict):
            normalized.append(item["service"])
        elif isinstance(item, dict):
            normalized.append(item)
    matches = [svc for svc in normalized if svc.get("name") == name]
    if not matches:
        names = sorted({svc.get("name") for svc in normalized})
        fail(f"service {name} not found; available: {names}")
    if len(matches) > 1:
        fail(f"multiple services named {name}")
    return matches[0]


def ensure_disk(
    token: str, service_id: str, name: str, size_gb: int, mount_path: str
) -> None:
    disks = request("GET", "/disks", token)
    if not isinstance(disks, list):
        fail("unexpected disks response")
    for disk in disks:
        if disk.get("serviceId") == service_id:
            return
    payload = {
        "name": name,
        "sizeGB": size_gb,
        "mountPath": mount_path,
        "serviceId": service_id,
    }
    request_allow_disk_exists("POST", "/disks", token, payload)


def load_env(path: Path) -> dict[str, str]:
    if not path.exists():
        fail(f"env file not found: {path}")
    data = json.loads(path.read_text())
    if not isinstance(data, dict):
        fail(f"env file must be a json object: {path}")
    env = {}
    for key, value in data.items():
        if value is None:
            continue
        if not isinstance(value, str):
            env[key] = json.dumps(value)
        else:
            env[key] = value
    return env


def update_env(token: str, service_id: str, env: dict[str, str]) -> None:
    env_list = []
    for key, value in env.items():
        if value == "":
            continue
        env_list.append({"key": key, "value": value})
    request("PUT", f"/services/{service_id}/env-vars", token, env_list)


def trigger_deploy(token: str, service_id: str) -> None:
    request("POST", f"/services/{service_id}/deploys", token, {})


def main() -> None:
    parser = argparse.ArgumentParser(description="Apply Render config for AgentNet services.")
    parser.add_argument("--api-key-file", type=Path, default=Path("ref/renderkey.txt"))
    parser.add_argument("--agentmesh-name", default="agentmesh-mainnet")
    parser.add_argument("--agentindex-name", default="agentindex-mainnet")
    parser.add_argument(
        "--agentmesh-env",
        type=Path,
        default=Path.home() / ".agentnet-secrets/render-env-agentmesh.json",
    )
    parser.add_argument(
        "--agentindex-env",
        type=Path,
        default=Path.home() / ".agentnet-secrets/render-env-agentindex.json",
    )
    parser.add_argument("--mesh-disk-size", type=int, default=20)
    parser.add_argument("--index-disk-size", type=int, default=10)
    parser.add_argument("--mount-path", default="/var/lib/agentnet")
    args = parser.parse_args()

    token = read_key(args.api_key_file)
    services = request("GET", "/services", token)
    if not isinstance(services, list):
        fail("unexpected services response")

    mesh = find_service(services, args.agentmesh_name)
    index = find_service(services, args.agentindex_name)

    mesh_id = mesh.get("id")
    index_id = index.get("id")
    if not mesh_id or not index_id:
        fail("missing service ids")

    ensure_disk(token, mesh_id, f"{args.agentmesh_name}-data", args.mesh_disk_size, args.mount_path)
    ensure_disk(token, index_id, f"{args.agentindex_name}-data", args.index_disk_size, args.mount_path)

    index_details = index.get("serviceDetails", {})
    index_url = index_details.get("url")
    if not index_url:
        fail("agentindex url not available yet")

    mesh_env = load_env(args.agentmesh_env)
    mesh_env["AGENTINDEX_URL"] = index_url

    index_env = load_env(args.agentindex_env)

    update_env(token, mesh_id, mesh_env)
    update_env(token, index_id, index_env)

    trigger_deploy(token, mesh_id)
    trigger_deploy(token, index_id)

    print("Render config applied and deploys triggered.")


if __name__ == "__main__":
    main()
