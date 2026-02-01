#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
import urllib.error
import urllib.request
from pathlib import Path
from typing import Optional


class ClaimError(RuntimeError):
    pass


def request_json(method: str, url: str, payload: Optional[dict] = None, api_key: Optional[str] = None) -> dict:
    data = None
    headers = {"Content-Type": "application/json"}
    if api_key:
        headers["Authorization"] = f"Bearer {api_key}"
    if payload is not None:
        data = json.dumps(payload).encode("utf-8")
    req = urllib.request.Request(url, data=data, headers=headers, method=method)
    try:
        with urllib.request.urlopen(req) as resp:
            body = resp.read()
            if not body:
                return {}
            return json.loads(body.decode("utf-8"))
    except urllib.error.HTTPError as exc:
        body = exc.read().decode("utf-8", errors="replace")
        raise ClaimError(f"{method} {url} failed: {exc.code} {body}")
    except urllib.error.URLError as exc:
        raise ClaimError(f"{method} {url} failed: {exc}")


def load_api_key(args: argparse.Namespace) -> Optional[str]:
    if args.claim_api_key and args.claim_api_key_file:
        raise ClaimError("use --claim-api-key or --claim-api-key-file, not both")
    if args.claim_api_key_file:
        key = args.claim_api_key_file.read_text().strip()
        return key or None
    if args.claim_api_key:
        return args.claim_api_key
    return (
        os.getenv("AGENTNET_CLAIM_API_KEY")
        or os.getenv("ANET_CLAIM_API_KEY")
        or None
    )


def load_claim_service_url(args: argparse.Namespace, agent_dir: Optional[Path]) -> str:
    if args.claim_service_url:
        return args.claim_service_url
    env_url = os.getenv("AGENTNET_CLAIM_URL") or os.getenv("ANET_CLAIM_URL")
    if env_url:
        return env_url
    if agent_dir:
        onboard_path = agent_dir / "onboard.json"
        if onboard_path.exists():
            data = json.loads(onboard_path.read_text())
            if isinstance(data, dict):
                url = data.get("claim_service_url")
                if url:
                    return url
    return "https://agentclaim-mainnet.onrender.com"


def load_claim_id(args: argparse.Namespace, agent_dir: Optional[Path]) -> str:
    if args.claim_id:
        return args.claim_id
    if args.claim_file:
        data = json.loads(args.claim_file.read_text())
        claim_id = data.get("claim_id")
        if claim_id:
            return str(claim_id)
    if agent_dir:
        claim_path = agent_dir / "claim.json"
        if claim_path.exists():
            data = json.loads(claim_path.read_text())
            claim_id = data.get("claim_id")
            if claim_id:
                return str(claim_id)
    raise ClaimError("claim id not found; provide --claim-id or --claim-file")


def main() -> None:
    parser = argparse.ArgumentParser(description="Revoke a pairing claim for an agent.")
    parser.add_argument("--claim-service-url", default=None)
    parser.add_argument("--claim-id", default=None)
    parser.add_argument("--claim-file", type=Path, default=None)
    parser.add_argument("--agent-dir", type=Path, default=None)
    parser.add_argument("--claim-api-key", default=None)
    parser.add_argument("--claim-api-key-file", type=Path, default=None)
    parser.add_argument("--out-dir", type=Path, default=None)
    args = parser.parse_args()

    agent_dir = args.agent_dir.expanduser().resolve() if args.agent_dir else None
    api_key = load_api_key(args)
    claim_url = load_claim_service_url(args, agent_dir)
    claim_id = load_claim_id(args, agent_dir)

    base = claim_url.rstrip("/")
    payload = request_json("POST", f"{base}/v1/claims/{claim_id}/revoke", api_key=api_key)

    out_dir = args.out_dir.expanduser().resolve() if args.out_dir else agent_dir
    if out_dir:
        out_dir.mkdir(parents=True, exist_ok=True)
        (out_dir / "claim-revoked.json").write_text(json.dumps(payload, indent=2) + "\n")

    print(json.dumps(payload, indent=2))


if __name__ == "__main__":
    main()
