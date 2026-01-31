#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

try:
    import tomllib  # Python 3.11+
except ModuleNotFoundError:  # pragma: no cover
    print("Python 3.11+ required for tomllib", file=sys.stderr)
    sys.exit(2)


def load_toml(path: Path) -> dict:
    data = path.read_bytes()
    return tomllib.loads(data.decode("utf-8"))


def require(obj: dict, key: str, label: str) -> object:
    if key not in obj:
        fail(f"missing required {label}: {key}")
    value = obj[key]
    if value is None or (isinstance(value, str) and value.strip() == ""):
        fail(f"{label} is empty: {key}")
    return value


def require_list(obj: dict, key: str, label: str) -> list:
    value = require(obj, key, label)
    if not isinstance(value, list) or not value:
        fail(f"{label} must be a non-empty list: {key}")
    return value


def require_bool(obj: dict, key: str, label: str) -> bool:
    value = require(obj, key, label)
    if not isinstance(value, bool):
        fail(f"{label} must be boolean: {key}")
    return value


def require_int(obj: dict, key: str, label: str) -> int:
    value = require(obj, key, label)
    if not isinstance(value, int):
        fail(f"{label} must be integer: {key}")
    return value


def fail(message: str) -> None:
    print(f"CONFIG INVALID: {message}", file=sys.stderr)
    sys.exit(1)


def validate_agentmesh(path: Path) -> None:
    cfg = load_toml(path)

    require(cfg, "chain_id", "agentmesh")
    require(cfg, "agent_did", "agentmesh")
    require(cfg, "key_path", "agentmesh")
    require(cfg, "state_dir", "agentmesh")
    require_list(cfg, "listen_addrs", "agentmesh")
    require_list(cfg, "protocols", "agentmesh")
    require_list(cfg, "transports", "agentmesh")
    require_list(cfg, "roles", "agentmesh")

    features = require(cfg, "features", "agentmesh")
    if not isinstance(features, dict):
        fail("features must be a table")
    require_list(features, "encodings", "features")
    require_int(features, "max_msg_bytes", "features")
    require_bool(features, "supports_receipt_anchoring", "features")
    require(features, "time_sync", "features")

    pubsub = require(cfg, "pubsub", "agentmesh")
    if not isinstance(pubsub, dict):
        fail("pubsub must be a table")
    require_list(pubsub, "topics", "pubsub")
    require_bool(pubsub, "require_economic_proof", "pubsub")
    require_bool(pubsub, "verify_signatures", "pubsub")
    require_list(pubsub, "economic_proof_validator_cmd", "pubsub")
    require_int(pubsub, "economic_proof_validator_timeout_ms", "pubsub")
    require_bool(pubsub, "economic_proof_fail_open", "pubsub")

    handshake = require(cfg, "handshake", "agentmesh")
    if not isinstance(handshake, dict):
        fail("handshake must be a table")
    require_int(handshake, "max_clock_skew_sec", "handshake")
    require_bool(handshake, "require_peer_id_match", "handshake")

    kill_switch = require(cfg, "kill_switch", "agentmesh")
    if not isinstance(kill_switch, dict):
        fail("kill_switch must be a table")
    require_bool(kill_switch, "enabled", "kill_switch")
    require(kill_switch, "topic", "kill_switch")
    require_int(kill_switch, "payload_type", "kill_switch")
    require(kill_switch, "pubkey_hex", "kill_switch")
    require_int(kill_switch, "max_clock_skew_sec", "kill_switch")
    require_int(kill_switch, "replay_window", "kill_switch")
    require_bool(kill_switch, "allow_release", "kill_switch")

    receipts = require(cfg, "receipts", "agentmesh")
    if not isinstance(receipts, dict):
        fail("receipts must be a table")
    require_bool(receipts, "enabled", "receipts")
    require(receipts, "path", "receipts")
    require_bool(receipts, "emit_policy_accepts", "receipts")
    require_bool(receipts, "emit_policy_denies", "receipts")
    require_bool(receipts, "emit_kill_switch", "receipts")

    tx = require(cfg, "tx", "agentmesh")
    if not isinstance(tx, dict):
        fail("tx must be a table")
    require_bool(tx, "enabled", "tx")
    require_int(tx, "pubsub_payload_type", "tx")

    for table in ("identity", "budget", "skill_registry", "work_registry", "escrow"):
        section = require(tx, table, f"tx.{table}")
        if not isinstance(section, dict):
            fail(f"{table} must be a table")
        require_bool(section, "enabled", f"tx.{table}")
        require(section, "state_path", f"tx.{table}")

    budget = tx.get("budget", {})
    require_int(budget, "window_sec", "tx.budget")
    require_list(budget, "caps", "tx.budget")

    agentmail = require(cfg, "agentmail", "agentmesh")
    if not isinstance(agentmail, dict):
        fail("agentmail must be a table")
    require_bool(agentmail, "enabled", "agentmail")
    require(agentmail, "topic", "agentmail")
    require_int(agentmail, "payload_type", "agentmail")
    require_bool(agentmail, "require_recipient", "agentmail")
    require_bool(agentmail, "enforce_sender_match", "agentmail")
    require_bool(agentmail, "require_postage_for_unknown", "agentmail")
    require_int(agentmail, "max_clock_skew_sec", "agentmail")
    require_int(agentmail, "max_markdown_bytes", "agentmail")
    require_int(agentmail, "max_attachments", "agentmail")
    require_int(agentmail, "max_attachment_bytes", "agentmail")
    require_int(agentmail, "max_total_attachment_bytes", "agentmail")
    require(agentmail, "inbox_path", "agentmail")
    require(agentmail, "seen_path", "agentmail")
    require_int(agentmail, "retention_sec", "agentmail")
    require_int(agentmail, "max_seen_entries", "agentmail")

    print(f"OK: {path}")


def validate_econ(path: Path) -> None:
    cfg = load_toml(path)
    voucher = cfg.get("voucher")
    onchain = cfg.get("onchain")
    if voucher is None and onchain is None:
        fail("econ config must enable voucher and/or onchain")
    if voucher is not None:
        if not isinstance(voucher, dict):
            fail("voucher must be a table")
        require_list(voucher, "issuers", "voucher")
        require(voucher, "nonce_state_path", "voucher")
        require_int(voucher, "max_clock_skew_sec", "voucher")
        require_bool(voucher, "require_topic_match", "voucher")
        require_list(voucher, "allowed_purposes", "voucher")
        issuers = voucher.get("issuers", [])
        for entry in issuers:
            if not isinstance(entry, dict):
                fail("voucher issuers must be tables")
            require(entry, "did", "voucher.issuers")
            require(entry, "pubkey_hex", "voucher.issuers")
    if onchain is not None:
        if not isinstance(onchain, dict):
            fail("onchain must be a table")
        require_bool(onchain, "enabled", "onchain")
        if onchain.get("enabled", False):
            require(onchain, "chain_id", "onchain")
            require(onchain, "rpc_url", "onchain")
            require_int(onchain, "min_confirmations", "onchain")
            require_bool(onchain, "require_success", "onchain")
            require_int(onchain, "max_tx_age_sec", "onchain")
            require(onchain, "required_to", "onchain")
            require(onchain, "required_from", "onchain")

    print(f"OK: {path}")


def main() -> None:
    parser = argparse.ArgumentParser(description="Validate AgentNet production configs.")
    parser.add_argument("--agentmesh", type=Path, required=True, help="Path to agentmesh config TOML")
    parser.add_argument("--econ", type=Path, required=True, help="Path to anet-econ-verify config TOML")
    args = parser.parse_args()

    validate_agentmesh(args.agentmesh)
    validate_econ(args.econ)


if __name__ == "__main__":
    main()
