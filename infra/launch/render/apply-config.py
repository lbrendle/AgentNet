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
    lines = [line.strip() for line in path.read_text().splitlines() if line.strip()]
    if not lines:
        fail(f"api key file empty: {path}")
    if len(lines) > 1:
        print(f"[render-apply] warning: extra lines found in {path}; using first line", file=sys.stderr)
    return lines[0]


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


def find_service_optional(services: list[dict], name: str) -> dict | None:
    normalized = []
    for item in services:
        if isinstance(item, dict) and "service" in item and isinstance(item["service"], dict):
            normalized.append(item["service"])
        elif isinstance(item, dict):
            normalized.append(item)
    matches = [svc for svc in normalized if svc.get("name") == name]
    if not matches:
        return None
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


def fetch_mesh_info(index_url: str) -> dict | None:
    try:
        with urlopen(f"{index_url.rstrip('/')}/mesh/info") as resp:
            if resp.status != 200:
                return None
            return json.loads(resp.read().decode("utf-8"))
    except Exception:
        return None


def build_bootstrap_addr(mesh_info: dict) -> str | None:
    if not isinstance(mesh_info, dict):
        return None
    peer_id = mesh_info.get("peer_id")
    public_ws = mesh_info.get("public_ws")
    if not peer_id or not public_ws:
        return None
    if public_ws.startswith("wss://"):
        host = public_ws[len("wss://") :]
        return f"/dns4/{host}/tcp/443/wss/p2p/{peer_id}"
    if public_ws.startswith("ws://"):
        host = public_ws[len("ws://") :]
        return f"/dns4/{host}/tcp/80/ws/p2p/{peer_id}"
    return None


def main() -> None:
    parser = argparse.ArgumentParser(description="Apply Render config for AgentNet services.")
    parser.add_argument("--api-key-file", type=Path, default=Path("ref/renderkey.txt"))
    parser.add_argument("--agentmesh-name", default="agentmesh-mainnet")
    parser.add_argument("--agentindex-name", default="agentindex-mainnet")
    parser.add_argument("--agentclaim-name", default="agentclaim-mainnet")
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
    parser.add_argument(
        "--agentclaim-env",
        type=Path,
        default=Path.home() / ".agentnet-secrets/render-env-agentclaim.json",
    )
    parser.add_argument("--mesh-disk-size", type=int, default=20)
    parser.add_argument("--index-disk-size", type=int, default=10)
    parser.add_argument("--claim-disk-size", type=int, default=5)
    parser.add_argument("--mount-path", default="/var/lib/agentnet")
    parser.add_argument("--skip-claim", action="store_true")
    parser.add_argument(
        "--extra-mesh-name",
        action="append",
        default=[],
        help="additional mesh service name (repeatable)",
    )
    parser.add_argument(
        "--bootstrap-multiaddr",
        default=None,
        help="override bootstrap multiaddr for extra mesh services",
    )
    args = parser.parse_args()

    token = read_key(args.api_key_file)
    services = request("GET", "/services", token)
    if not isinstance(services, list):
        fail("unexpected services response")

    mesh = find_service(services, args.agentmesh_name)
    index = find_service(services, args.agentindex_name)
    claim = None
    if not args.skip_claim and args.agentclaim_name:
        claim = find_service_optional(services, args.agentclaim_name)
        if claim is None:
            print(
                f"[render-apply] claim service {args.agentclaim_name} not found; skipping",
                file=sys.stderr,
            )

    mesh_id = mesh.get("id")
    index_id = index.get("id")
    if not mesh_id or not index_id:
        fail("missing service ids")
    claim_id = None
    if claim is not None:
        claim_id = claim.get("id")
        if not claim_id:
            fail("missing claim service id")

    ensure_disk(token, mesh_id, f"{args.agentmesh_name}-data", args.mesh_disk_size, args.mount_path)
    ensure_disk(token, index_id, f"{args.agentindex_name}-data", args.index_disk_size, args.mount_path)
    if claim_id:
        ensure_disk(token, claim_id, f"{args.agentclaim_name}-data", args.claim_disk_size, args.mount_path)

    index_details = index.get("serviceDetails", {})
    index_url = index_details.get("url")
    if not index_url:
        fail("agentindex url not available yet")

    mesh_details = mesh.get("serviceDetails", {})
    mesh_url = mesh_details.get("url")
    mesh_public_ws = None
    if mesh_url:
        if mesh_url.startswith("https://"):
            mesh_public_ws = "wss://" + mesh_url[len("https://") :]
        elif mesh_url.startswith("http://"):
            mesh_public_ws = "ws://" + mesh_url[len("http://") :]

    mesh_env = load_env(args.agentmesh_env)
    mesh_env["AGENTINDEX_URL"] = index_url
    if mesh_public_ws:
        mesh_env["AGENTMESH_PUBLIC_WS"] = mesh_public_ws

    index_env = load_env(args.agentindex_env)

    update_env(token, mesh_id, mesh_env)
    update_env(token, index_id, index_env)
    if claim_id:
        claim_env = load_env(args.agentclaim_env)
        update_env(token, claim_id, claim_env)

    bootstrap_addr = args.bootstrap_multiaddr
    if args.extra_mesh_name and not bootstrap_addr:
        mesh_info = fetch_mesh_info(index_url)
        bootstrap_addr = build_bootstrap_addr(mesh_info) if mesh_info else None
        if not bootstrap_addr:
            fail("bootstrap multiaddr not available; pass --bootstrap-multiaddr")

    for extra_name in args.extra_mesh_name:
        extra = find_service(services, extra_name)
        extra_id = extra.get("id")
        if not extra_id:
            fail(f"missing service id for {extra_name}")
        ensure_disk(
            token,
            extra_id,
            f"{extra_name}-data",
            args.mesh_disk_size,
            args.mount_path,
        )
        extra_details = extra.get("serviceDetails", {})
        extra_url = extra_details.get("url")
        extra_public_ws = None
        if extra_url:
            if extra_url.startswith("https://"):
                extra_public_ws = "wss://" + extra_url[len("https://") :]
            elif extra_url.startswith("http://"):
                extra_public_ws = "ws://" + extra_url[len("http://") :]
        extra_env = load_env(args.agentmesh_env)
        extra_env["AGENTINDEX_URL"] = index_url
        if bootstrap_addr:
            extra_env["AGENTMESH_BOOTSTRAP_ADDRS"] = bootstrap_addr
        if extra_public_ws:
            extra_env["AGENTMESH_PUBLIC_WS"] = extra_public_ws
        update_env(token, extra_id, extra_env)
        trigger_deploy(token, extra_id)

    trigger_deploy(token, mesh_id)
    trigger_deploy(token, index_id)
    if claim_id:
        trigger_deploy(token, claim_id)

    print("Render config applied and deploys triggered.")


if __name__ == "__main__":
    main()
