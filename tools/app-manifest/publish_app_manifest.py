#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import subprocess
import tempfile
import time
from pathlib import Path
from typing import Optional

try:
    import tomllib
except Exception:  # pragma: no cover
    import tomli as tomllib  # type: ignore

from app_manifest_lib import AppManifestError, build_manifest_from_app, read_b64_key

ROOT = Path(__file__).resolve().parents[2]
import sys

sys.path.insert(0, str(ROOT / "impl" / "python"))

try:
    from agentnet_py.skill import decode_skill_manifest, skill_publish_payload_to_cbor, SkillPublishPayload
    from agentnet_py.tx import TxEnvelopePayload, build_tx_envelope
except Exception as exc:  # pragma: no cover
    raise SystemExit(f"[app-manifest] missing agentnet_py deps: {exc}")


TX_SKILL_PUBLISH = 30
DEFAULT_TX_PAYLOAD_TYPE = 2000
DEFAULT_TOPIC = "agentnet/main/1.0.0"


def load_topic(config_path: Path) -> str:
    data = tomllib.loads(config_path.read_text())
    pubsub = data.get("pubsub", {}) if isinstance(data, dict) else {}
    topics = pubsub.get("topics") if isinstance(pubsub, dict) else None
    if isinstance(topics, list) and topics:
        return str(topics[0])
    return DEFAULT_TOPIC


def load_tx_payload_type(config_path: Path) -> int:
    data = tomllib.loads(config_path.read_text())
    tx = data.get("tx", {}) if isinstance(data, dict) else {}
    payload = tx.get("pubsub_payload_type") if isinstance(tx, dict) else None
    if payload is None:
        return DEFAULT_TX_PAYLOAD_TYPE
    return int(payload)


def choose_agentmesh_bin(arg: Optional[Path]) -> Path:
    if arg:
        return arg.expanduser()
    candidate = ROOT / "impl" / "rust" / "target" / "debug" / "agentmesh"
    if candidate.exists():
        return candidate
    return Path("agentmesh")


def main() -> None:
    parser = argparse.ArgumentParser(description="Publish a signed APP manifest via AgentNet tx.")
    parser.add_argument("--config", type=Path, required=True, help="agentmesh.toml")
    parser.add_argument("--manifest", type=Path, default=None, help="Signed manifest cbor")
    parser.add_argument("--app", type=Path, default=None, help="APP.md path (compile before publish)")
    parser.add_argument("--agent-key", type=Path, required=True, help="Agent ed25519 key (base64)")
    parser.add_argument("--agent-did", default=None)
    parser.add_argument("--artifact-root", type=Path, default=None)
    parser.add_argument("--out-dir", type=Path, default=None)
    parser.add_argument("--topic", default=None)
    parser.add_argument("--tx-payload-type", type=int, default=None)
    parser.add_argument("--voucher-hex", default=None)
    parser.add_argument("--voucher-file", type=Path, default=None)
    parser.add_argument("--agentmesh-bin", type=Path, default=None)
    parser.add_argument("--preconnect-seconds", type=int, default=30)
    parser.add_argument("--settle-seconds", type=int, default=20)
    parser.add_argument("--print-tx", action="store_true")
    args = parser.parse_args()

    config_path = args.config.expanduser().resolve()
    if not config_path.exists():
        raise SystemExit(f"[app-manifest] config not found: {config_path}")

    manifest_path: Optional[Path] = args.manifest.expanduser().resolve() if args.manifest else None
    summary = None
    if manifest_path is None:
        if not args.app:
            raise SystemExit("[app-manifest] provide --manifest or --app")
        app_path = args.app.expanduser().resolve()
        out_dir = args.out_dir.expanduser().resolve() if args.out_dir else app_path.parent / "dist" / "agentnet-apps"
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
        summary = result["summary"]
        manifest_path = result["manifest_path"]

    manifest_bytes = manifest_path.read_bytes()
    manifest_obj = decode_skill_manifest(manifest_bytes)
    author = manifest_obj.payload.author
    if args.agent_did and author != args.agent_did:
        raise SystemExit("[app-manifest] manifest author does not match agent DID")

    ts = int(time.time())
    publish_payload = SkillPublishPayload(manifest=manifest_bytes, ts=ts)
    publish_cbor = skill_publish_payload_to_cbor(publish_payload)

    nonce = int(time.time() * 1000)
    tx_payload = TxEnvelopePayload(
        tx_type=TX_SKILL_PUBLISH,
        sender=author,
        nonce=nonce,
        fee=0,
        payload=publish_cbor,
    )

    secret_key = read_b64_key(args.agent_key.expanduser())
    tx_cbor = build_tx_envelope(tx_payload, secret_key)

    topic = args.topic or load_topic(config_path)
    payload_type = args.tx_payload_type or load_tx_payload_type(config_path)

    if args.print_tx:
        print(json.dumps(
            {
                "topic": topic,
                "payload_type": payload_type,
                "sender": author,
                "nonce": nonce,
                "ts": ts,
                "manifest_path": str(manifest_path),
            },
            indent=2,
        ))

    voucher_hex = None
    if args.voucher_file:
        voucher_hex = args.voucher_file.read_text().strip() or None
    if args.voucher_hex:
        voucher_hex = args.voucher_hex

    agentmesh_bin = choose_agentmesh_bin(args.agentmesh_bin)

    with tempfile.NamedTemporaryFile(delete=False) as tmp:
        tmp.write(tx_cbor)
        tmp_path = Path(tmp.name)

    try:
        cmd = [
            str(agentmesh_bin),
            "publish",
            "--config",
            str(config_path),
            "--topic",
            topic,
            "--payload-type",
            str(payload_type),
            "--payload-cbor",
            str(tmp_path),
            "--preconnect-seconds",
            str(args.preconnect_seconds),
            "--settle-seconds",
            str(args.settle_seconds),
        ]
        if voucher_hex:
            cmd.extend(["--proof-voucher-hex", voucher_hex])
        subprocess.run(cmd, check=True)
    finally:
        try:
            os.unlink(tmp_path)
        except Exception:
            pass


if __name__ == "__main__":
    main()
