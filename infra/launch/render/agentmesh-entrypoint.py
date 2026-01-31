#!/usr/bin/env python3
from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path


def fail(message: str) -> None:
    print(f"[agentmesh-entrypoint] {message}", file=sys.stderr)
    sys.exit(1)


def env_required(name: str) -> str:
    value = os.getenv(name)
    if value is None or value.strip() == "":
        fail(f"missing required env: {name}")
    return value.strip()


def env_optional(name: str, default: str | None = None) -> str | None:
    value = os.getenv(name)
    if value is None:
        return default
    value = value.strip()
    return value if value else default


def env_bool(name: str) -> bool:
    value = env_required(name).lower()
    if value in ("true", "1", "yes"):
        return True
    if value in ("false", "0", "no"):
        return False
    fail(f"invalid boolean for {name}: {value}")
    return False


def env_int(name: str) -> int:
    value = env_required(name)
    try:
        return int(value)
    except ValueError:
        fail(f"invalid integer for {name}: {value}")
        return 0


def env_list(name: str) -> list[str]:
    raw = env_required(name)
    items = [item.strip() for item in raw.split(",") if item.strip()]
    if not items:
        fail(f"{name} must contain at least one item")
    return items


def env_list_optional(name: str) -> list[str]:
    raw = env_optional(name)
    if raw is None:
        return []
    items = [item.strip() for item in raw.split(",") if item.strip()]
    return items


def env_json(name: str) -> object:
    raw = env_required(name)
    try:
        return json.loads(raw)
    except json.JSONDecodeError as exc:
        fail(f"invalid JSON for {name}: {exc}")
        return {}


def toml_escape(value: str) -> str:
    escaped = value.replace("\\", "\\\\").replace('"', '\\"')
    return f'"{escaped}"'


def toml_value(value: object) -> str:
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, int):
        return str(value)
    if isinstance(value, str):
        return toml_escape(value)
    if value is None:
        return "null"
    if isinstance(value, list):
        return "[" + ", ".join(toml_value(v) for v in value) + "]"
    raise TypeError(f"unsupported value type for toml: {type(value)}")


def emit_section(prefix: str, data: dict, lines: list[str]) -> None:
    for key, value in data.items():
        if isinstance(value, dict):
            lines.append(f"[{prefix}{key}]")
            emit_section(f"{prefix}{key}.", value, lines)
        elif isinstance(value, list) and value and all(isinstance(v, dict) for v in value):
            for item in value:
                lines.append(f"[[{prefix}{key}]]")
                emit_section(f"{prefix}{key}.", item, lines)
        else:
            lines.append(f"{key} = {toml_value(value)}")


def dump_toml(config: dict) -> str:
    lines: list[str] = []
    emit_section("", config, lines)
    return "\n".join(lines) + "\n"


def ensure_key(path: Path) -> None:
    if path.exists():
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    print(f"[agentmesh-entrypoint] generating key at {path}")
    subprocess.run(
        ["/usr/local/bin/agentmesh", "keygen", "--out", str(path)],
        check=True,
    )


def main() -> None:
    key_path = Path(env_optional("AGENTMESH_KEY_PATH", "/var/lib/agentnet/keys/agentmesh.key"))
    state_dir = Path(env_optional("AGENTMESH_STATE_DIR", "/var/lib/agentnet/state"))
    state_dir.mkdir(parents=True, exist_ok=True)

    ensure_key(key_path)

    pubsub_econ_cmd = env_json("AGENTMESH_PUBSUB_ECON_CMD")
    if not isinstance(pubsub_econ_cmd, list) or not pubsub_econ_cmd:
        fail("AGENTMESH_PUBSUB_ECON_CMD must be a JSON array command")

    tx_sender_keys = env_json("AGENTMESH_TX_SENDER_PUBKEYS_JSON")
    if not isinstance(tx_sender_keys, list):
        fail("AGENTMESH_TX_SENDER_PUBKEYS_JSON must be a JSON array")

    budget_caps = env_json("AGENTMESH_TX_BUDGET_CAPS_JSON")
    if not isinstance(budget_caps, list) or not budget_caps:
        fail("AGENTMESH_TX_BUDGET_CAPS_JSON must be a non-empty JSON array")

    agentmail_sender_keys = env_json("AGENTMESH_AGENTMAIL_SENDER_PUBKEYS_JSON")
    if not isinstance(agentmail_sender_keys, list):
        fail("AGENTMESH_AGENTMAIL_SENDER_PUBKEYS_JSON must be a JSON array")

    dht_service_records = env_json("AGENTMESH_DHT_SERVICE_RECORDS_JSON")
    if not isinstance(dht_service_records, list):
        fail("AGENTMESH_DHT_SERVICE_RECORDS_JSON must be a JSON array")

    dht_community_record = env_json("AGENTMESH_DHT_COMMUNITY_RECORD_JSON")
    if not isinstance(dht_community_record, dict):
        fail("AGENTMESH_DHT_COMMUNITY_RECORD_JSON must be a JSON object")

    voucher_issuers = env_json("ANET_ECON_VOUCHER_ISSUERS_JSON")
    if not isinstance(voucher_issuers, list) or not voucher_issuers:
        fail("ANET_ECON_VOUCHER_ISSUERS_JSON must be a non-empty JSON array")

    econ_config = {
        "voucher": {
            "issuers": voucher_issuers,
            "nonce_state_path": env_required("ANET_ECON_VOUCHER_NONCE_STATE_PATH"),
            "max_clock_skew_sec": env_int("ANET_ECON_VOUCHER_MAX_CLOCK_SKEW_SEC"),
            "require_topic_match": env_bool("ANET_ECON_VOUCHER_REQUIRE_TOPIC_MATCH"),
            "allowed_purposes": env_list("ANET_ECON_VOUCHER_ALLOWED_PURPOSES"),
        },
        "onchain": {
            "enabled": env_bool("ANET_ECON_ONCHAIN_ENABLED"),
            "chain_id": env_required("ANET_ECON_ONCHAIN_CHAIN_ID"),
            "rpc_url": env_required("ANET_ECON_ONCHAIN_RPC_URL"),
            "min_confirmations": env_int("ANET_ECON_ONCHAIN_MIN_CONFIRMATIONS"),
            "require_success": env_bool("ANET_ECON_ONCHAIN_REQUIRE_SUCCESS"),
            "max_tx_age_sec": env_int("ANET_ECON_ONCHAIN_MAX_TX_AGE_SEC"),
            "required_to": env_required("ANET_ECON_ONCHAIN_REQUIRED_TO"),
            "required_from": env_required("ANET_ECON_ONCHAIN_REQUIRED_FROM"),
        },
    }

    config = {
        "chain_id": env_required("AGENTMESH_CHAIN_ID"),
        "agent_did": env_required("AGENTMESH_AGENT_DID"),
        "key_path": str(key_path),
        "node_id": env_optional("AGENTMESH_NODE_ID"),
        "state_dir": str(state_dir),
        "listen_addrs": env_list("AGENTMESH_LISTEN_ADDRS"),
        "bootstrap": env_list_optional("AGENTMESH_BOOTSTRAP_ADDRS"),
        "protocols": env_list("AGENTMESH_PROTOCOLS"),
        "transports": env_list("AGENTMESH_TRANSPORTS"),
        "roles": env_list("AGENTMESH_ROLES"),
        "features": {
            "encodings": env_list("AGENTMESH_FEATURE_ENCODINGS"),
            "max_msg_bytes": env_int("AGENTMESH_FEATURE_MAX_MSG_BYTES"),
            "supports_receipt_anchoring": env_bool("AGENTMESH_FEATURE_RECEIPT_ANCHORING"),
            "time_sync": env_required("AGENTMESH_FEATURE_TIME_SYNC"),
        },
        "pubsub": {
            "topics": env_list("AGENTMESH_PUBSUB_TOPICS"),
            "require_economic_proof": env_bool("AGENTMESH_PUBSUB_REQUIRE_ECON"),
            "verify_signatures": env_bool("AGENTMESH_PUBSUB_VERIFY_SIGS"),
            "economic_proof_validator_cmd": pubsub_econ_cmd,
            "economic_proof_validator_timeout_ms": env_int("AGENTMESH_PUBSUB_ECON_TIMEOUT_MS"),
            "economic_proof_fail_open": env_bool("AGENTMESH_PUBSUB_ECON_FAIL_OPEN"),
        },
        "handshake": {
            "max_clock_skew_sec": env_int("AGENTMESH_HANDSHAKE_MAX_CLOCK_SKEW_SEC"),
            "require_peer_id_match": env_bool("AGENTMESH_HANDSHAKE_REQUIRE_PEER_ID_MATCH"),
        },
        "kill_switch": {
            "enabled": env_bool("AGENTMESH_KILL_ENABLED"),
            "topic": env_required("AGENTMESH_KILL_TOPIC"),
            "payload_type": env_int("AGENTMESH_KILL_PAYLOAD_TYPE"),
            "pubkey_hex": env_required("AGENTMESH_KILL_PUBKEY_HEX"),
            "max_clock_skew_sec": env_int("AGENTMESH_KILL_MAX_CLOCK_SKEW_SEC"),
            "replay_window": env_int("AGENTMESH_KILL_REPLAY_WINDOW"),
            "allow_release": env_bool("AGENTMESH_KILL_ALLOW_RELEASE"),
        },
        "receipts": {
            "enabled": env_bool("AGENTMESH_RECEIPTS_ENABLED"),
            "path": env_required("AGENTMESH_RECEIPTS_PATH"),
            "emit_policy_accepts": env_bool("AGENTMESH_RECEIPTS_EMIT_ACCEPTS"),
            "emit_policy_denies": env_bool("AGENTMESH_RECEIPTS_EMIT_DENIES"),
            "emit_kill_switch": env_bool("AGENTMESH_RECEIPTS_EMIT_KILL_SWITCH"),
        },
        "tx": {
            "enabled": env_bool("AGENTMESH_TX_ENABLED"),
            "pubsub_payload_type": env_int("AGENTMESH_TX_PUBSUB_PAYLOAD_TYPE"),
            "sender_pubkeys": tx_sender_keys,
            "identity": {
                "enabled": env_bool("AGENTMESH_TX_IDENTITY_ENABLED"),
                "state_path": env_required("AGENTMESH_TX_IDENTITY_STATE_PATH"),
            },
            "budget": {
                "enabled": env_bool("AGENTMESH_TX_BUDGET_ENABLED"),
                "state_path": env_required("AGENTMESH_TX_BUDGET_STATE_PATH"),
                "window_sec": env_int("AGENTMESH_TX_BUDGET_WINDOW_SEC"),
                "caps": budget_caps,
            },
            "skill_registry": {
                "enabled": env_bool("AGENTMESH_TX_SKILL_ENABLED"),
                "state_path": env_required("AGENTMESH_TX_SKILL_STATE_PATH"),
            },
            "work_registry": {
                "enabled": env_bool("AGENTMESH_TX_WORK_ENABLED"),
                "state_path": env_required("AGENTMESH_TX_WORK_STATE_PATH"),
            },
            "escrow": {
                "enabled": env_bool("AGENTMESH_TX_ESCROW_ENABLED"),
                "state_path": env_required("AGENTMESH_TX_ESCROW_STATE_PATH"),
                "log_path": env_required("AGENTMESH_TX_ESCROW_LOG_PATH"),
            },
        },
        "agentmail": {
            "enabled": env_bool("AGENTMESH_AGENTMAIL_ENABLED"),
            "topic": env_required("AGENTMESH_AGENTMAIL_TOPIC"),
            "payload_type": env_int("AGENTMESH_AGENTMAIL_PAYLOAD_TYPE"),
            "require_recipient": env_bool("AGENTMESH_AGENTMAIL_REQUIRE_RECIPIENT"),
            "enforce_sender_match": env_bool("AGENTMESH_AGENTMAIL_ENFORCE_SENDER_MATCH"),
            "require_postage_for_unknown": env_bool("AGENTMESH_AGENTMAIL_REQUIRE_POSTAGE_UNKNOWN"),
            "max_clock_skew_sec": env_int("AGENTMESH_AGENTMAIL_MAX_CLOCK_SKEW_SEC"),
            "max_markdown_bytes": env_int("AGENTMESH_AGENTMAIL_MAX_MARKDOWN_BYTES"),
            "max_attachments": env_int("AGENTMESH_AGENTMAIL_MAX_ATTACHMENTS"),
            "max_attachment_bytes": env_int("AGENTMESH_AGENTMAIL_MAX_ATTACHMENT_BYTES"),
            "max_total_attachment_bytes": env_int("AGENTMESH_AGENTMAIL_MAX_TOTAL_ATTACHMENT_BYTES"),
            "allow_senders": env_list_optional("AGENTMESH_AGENTMAIL_ALLOW_SENDERS"),
            "deny_senders": env_list_optional("AGENTMESH_AGENTMAIL_DENY_SENDERS"),
            "sender_pubkeys": agentmail_sender_keys,
            "inbox_path": env_required("AGENTMESH_AGENTMAIL_INBOX_PATH"),
            "seen_path": env_required("AGENTMESH_AGENTMAIL_SEEN_PATH"),
            "retention_sec": env_int("AGENTMESH_AGENTMAIL_RETENTION_SEC"),
            "max_seen_entries": env_int("AGENTMESH_AGENTMAIL_MAX_SEEN_ENTRIES"),
        },
        "dht": {
            "enabled": env_bool("AGENTMESH_DHT_ENABLED"),
            "publish_interval_sec": env_int("AGENTMESH_DHT_PUBLISH_INTERVAL_SEC"),
            "agent_record": {
                "record_key": env_required("AGENTMESH_DHT_AGENT_RECORD_KEY"),
                "agent_pubkeys_hex": env_list("AGENTMESH_DHT_AGENT_PUBKEYS_HEX"),
                "capabilities": env_list("AGENTMESH_DHT_AGENT_CAPABILITIES"),
                "expires_sec": env_int("AGENTMESH_DHT_AGENT_EXPIRES_SEC"),
                "signing_key_path": env_optional("AGENTMESH_DHT_AGENT_SIGNING_KEY_PATH"),
            },
            "service_records": dht_service_records,
            "community_record": dht_community_record,
        },
    }

    config_path = Path(env_optional("AGENTMESH_CONFIG_PATH", "/var/lib/agentnet/config/agentmesh.toml"))
    config_path.parent.mkdir(parents=True, exist_ok=True)
    config_path.write_text(dump_toml(config))

    econ_path = Path(env_required("AGENTMESH_ECON_CONFIG_PATH"))
    econ_path.parent.mkdir(parents=True, exist_ok=True)
    econ_path.write_text(dump_toml(econ_config))

    subprocess.run(
        [
            "python3",
            "/opt/agentnet/validate-config.py",
            "--agentmesh",
            str(config_path),
            "--econ",
            str(econ_path),
        ],
        check=True,
    )

    sync_cmd = [
        "python3",
        "/opt/agentnet/index-sync.py",
    ]
    sync_env = os.environ.copy()
    sync_env["AGENTMESH_STATE_DIR"] = str(state_dir)

    sync_proc = subprocess.Popen(sync_cmd, env=sync_env)
    try:
        subprocess.run(
            ["/usr/local/bin/agentmesh", "run", "--config", str(config_path)],
            check=True,
        )
    finally:
        sync_proc.terminate()
        sync_proc.wait(timeout=10)


if __name__ == "__main__":
    main()
