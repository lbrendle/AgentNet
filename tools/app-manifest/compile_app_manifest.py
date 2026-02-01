#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from pathlib import Path

from app_manifest_lib import AppManifestError, build_manifest_from_app


def default_out_dir(app_path: Path) -> Path:
    safe = "".join(ch if ch.isalnum() or ch in "._-" else "_" for ch in app_path.stem)
    return app_path.parent / "dist" / "agentnet-apps" / safe


def main() -> None:
    parser = argparse.ArgumentParser(description="Compile APP.md into a signed AgentNet app manifest.")
    parser.add_argument("--app", type=Path, required=True, help="Path to APP.md")
    parser.add_argument("--agent-key", type=Path, required=True, help="Agent ed25519 key (base64)")
    parser.add_argument("--agent-did", default=None, help="Agent DID (optional, used for validation)")
    parser.add_argument("--artifact-root", type=Path, default=None, help="Root for artifact paths")
    parser.add_argument("--out-dir", type=Path, default=None, help="Output directory")
    args = parser.parse_args()

    app_path = args.app.expanduser().resolve()
    if not app_path.exists():
        raise SystemExit(f"[app-manifest] APP.md not found: {app_path}")

    out_dir = args.out_dir.expanduser().resolve() if args.out_dir else default_out_dir(app_path)
    artifact_root = args.artifact_root.expanduser().resolve() if args.artifact_root else app_path.parent

    try:
        result = build_manifest_from_app(
            app_path=app_path,
            agent_key=args.agent_key.expanduser(),
            agent_did=args.agent_did,
            out_dir=out_dir,
            artifact_root=artifact_root,
        )
    except AppManifestError as exc:
        raise SystemExit(f"[app-manifest] {exc}") from exc

    print(json.dumps(result["summary"], indent=2))


if __name__ == "__main__":
    main()
