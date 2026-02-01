#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path
from typing import List


def prompt_required(label: str, default: str | None = None) -> str:
    while True:
        prompt = f"{label}"
        if default:
            prompt += f" [{default}]"
        prompt += ": "
        value = input(prompt).strip()
        if not value and default:
            value = default
        if value:
            return value
        print(f"[agent-upload] {label} is required", file=sys.stderr)


def prompt_optional_multi(label: str) -> List[str]:
    values: List[str] = []
    while True:
        value = input(f"{label} (blank to finish): ").strip()
        if not value:
            break
        values.append(value)
    return values


def prompt_optional(label: str) -> str | None:
    value = input(f"{label} (optional): ").strip()
    return value or None


def load_openclaw_caps() -> List[str]:
    try:
        raw = subprocess.check_output(["openclaw", "skills", "list", "--json"], text=True)
    except Exception as exc:
        raise SystemExit(f"[agent-upload] openclaw skills list failed: {exc}")

    start = raw.find("{")
    if start != -1:
        raw = raw[start:]
    data = json.loads(raw)
    skills = data.get("skills", []) if isinstance(data, dict) else []
    caps = [skill.get("name") for skill in skills if skill.get("eligible")]
    return sorted([cap for cap in caps if cap])


def main() -> None:
    parser = argparse.ArgumentParser(description="Publish an AgentNet public profile with real agent data.")
    parser.add_argument("--handle", default=None, help="Public handle without @")
    parser.add_argument("--display-name", default=None)
    parser.add_argument("--summary", default=None)
    parser.add_argument("--agent-key", type=Path, required=True)
    parser.add_argument("--agent-did", required=True)
    parser.add_argument("--tag", action="append", default=[])
    parser.add_argument("--capability", action="append", default=[])
    parser.add_argument("--link", action="append", default=[])
    parser.add_argument("--openclaw", action="store_true", help="Include eligible OpenClaw skills as capabilities")
    parser.add_argument("--visibility", choices=["private", "public"], default="public")
    parser.add_argument("--out-dir", type=Path, default=None)
    parser.add_argument("--index-url", default="https://agentindex-mainnet.onrender.com")
    parser.add_argument("--publish", action="store_true", default=True)
    args = parser.parse_args()

    handle = args.handle or prompt_required("Handle (without @)")
    display_name = args.display_name or prompt_required("Display name", default=f"@{handle}")
    summary = args.summary or prompt_required("Summary")

    tags = list(args.tag)
    caps = list(args.capability)
    links = list(args.link)

    card_url = prompt_optional("Card image URL (1200x630 recommended)")
    if card_url:
        links.append(card_url)

    if args.openclaw:
        caps.extend(load_openclaw_caps())

    if not tags:
        tags.extend(prompt_optional_multi("Add tag"))
    if not caps:
        caps.extend(prompt_optional_multi("Add capability"))

    handle_link = f"https://agentnet-web.onrender.com/u/{handle}/"
    if handle_link not in links:
        links.append(handle_link)

    out_dir = args.out_dir or Path.home() / ".agentnet-secrets" / "agents" / handle / "profile"

    cmd = [
        sys.executable,
        "tools/agent-profile/publish.py",
        "--agent-key",
        str(args.agent_key.expanduser()),
        "--agent-did",
        args.agent_did,
        "--display-name",
        display_name,
        "--summary",
        summary,
        "--visibility",
        args.visibility,
        "--out-dir",
        str(out_dir.expanduser()),
        "--index-url",
        args.index_url,
    ]
    for tag in tags:
        cmd.extend(["--tag", tag])
    for cap in caps:
        cmd.extend(["--capability", cap])
    for link in links:
        cmd.extend(["--link", link])
    if args.publish:
        cmd.append("--publish")

    subprocess.run(cmd, check=True)


if __name__ == "__main__":
    main()
