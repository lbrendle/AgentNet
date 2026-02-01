#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import sys
import time
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
    return args.claim_api_key


def write_json(path: Path, payload: dict) -> None:
    path.write_text(json.dumps(payload, indent=2) + "\n")


def main() -> None:
    parser = argparse.ArgumentParser(description="Initiate an X.com claim pairing for an agent.")
    parser.add_argument("--claim-service-url", required=True)
    parser.add_argument("--agent-did", required=True)
    parser.add_argument("--x-handle", default=None)
    parser.add_argument("--claim-api-key", default=None)
    parser.add_argument("--claim-api-key-file", type=Path, default=None)
    parser.add_argument("--wait-sec", type=int, default=600)
    parser.add_argument("--poll-sec", type=int, default=10)
    parser.add_argument("--out-dir", type=Path, default=None)
    args = parser.parse_args()

    api_key = load_api_key(args)
    base = args.claim_service_url.rstrip("/")
    payload = {"agent_did": args.agent_did}
    if args.x_handle:
        payload["x_handle"] = args.x_handle

    claim = request_json("POST", f"{base}/v1/claims", payload, api_key=api_key)
    claim_id = claim.get("claim_id")
    if not claim_id:
        raise SystemExit("[x-pair] claim service did not return claim_id")

    claim_url = f"{base}/v1/claims/{claim_id}"
    claim["claim_url"] = claim_url

    required_post = claim.get("required_post")
    if required_post:
        print(f"[x-pair] required X post: {required_post}", file=sys.stderr)

    out_dir = args.out_dir.expanduser().resolve() if args.out_dir else None
    if out_dir:
        out_dir.mkdir(parents=True, exist_ok=True)
        write_json(out_dir / "claim.json", claim)

    if args.wait_sec <= 0:
        print(json.dumps(claim, indent=2))
        return

    deadline = time.time() + args.wait_sec
    while time.time() < deadline:
        status = request_json("GET", claim_url, api_key=api_key)
        if status.get("status") == "issued" and status.get("voucher_hex"):
            if out_dir:
                (out_dir / "voucher.hex").write_text(status["voucher_hex"] + "\n")
                write_json(out_dir / "claim-issued.json", status)
            print(json.dumps(status, indent=2))
            return
        time.sleep(max(1, int(args.poll_sec)))

    raise SystemExit("[x-pair] claim voucher not issued within wait window")


if __name__ == "__main__":
    main()
