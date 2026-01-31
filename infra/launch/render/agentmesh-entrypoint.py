#!/usr/bin/env python3
from __future__ import annotations

import base64
import json
import os
import subprocess
import sys
import time
from pathlib import Path


def fail(message: str) -> None:
    print(f"[agentmesh-entrypoint] {message}", file=sys.stderr)
    sys.exit(1)


def env_required(name: str) -> str:
    value = os.getenv(name)
    if value is None or value.strip() == "":
        fail(f"missing required env: {name}")
    return expand_port(value.strip())


def env_optional(name: str, default: str | None = None) -> str | None:
    value = os.getenv(name)
    if value is None:
        return default
    value = expand_port(value.strip())
    return value if value else default


def env_bool(name: str) -> bool:
    value = env_required(name).lower()
    if value in ("true", "1", "yes"):
        return True
    if value in ("false", "0", "no"):
        return False
    fail(f"invalid boolean for {name}: {value}")
    return False


def env_bool_optional(name: str, default: bool = False) -> bool:
    raw = env_optional(name)
    if raw is None:
        return default
    value = raw.lower()
    if value in ("true", "1", "yes"):
        return True
    if value in ("false", "0", "no"):
        return False
    fail(f"invalid boolean for {name}: {value}")
    return default


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
    return [expand_port(item) for item in items]


def env_list_optional(name: str) -> list[str]:
    raw = env_optional(name)
    if raw is None:
        return []
    items = [item.strip() for item in raw.split(",") if item.strip()]
    return [expand_port(item) for item in items]


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
    scalars: list[tuple[str, object]] = []
    tables: list[tuple[str, dict]] = []
    arrays: list[tuple[str, list[dict]]] = []

    for key, value in data.items():
        if value is None:
            continue
        if isinstance(value, dict):
            tables.append((key, value))
        elif isinstance(value, list) and value and all(isinstance(v, dict) for v in value):
            arrays.append((key, value))
        else:
            scalars.append((key, value))

    for key, value in scalars:
        lines.append(f"{key} = {toml_value(value)}")

    for key, value in tables:
        lines.append(f"[{prefix}{key}]")
        emit_section(f"{prefix}{key}.", value, lines)

    # Emit array-of-table entries last so we don't accidentally nest later scalars under them.
    for key, value in arrays:
        for item in value:
            lines.append(f"[[{prefix}{key}]]")
            emit_section(f"{prefix}{key}.", item, lines)


def dump_toml(config: dict) -> str:
    lines: list[str] = []
    emit_section("", config, lines)
    return "\n".join(lines) + "\n"


def expand_port(value: str) -> str:
    if "$PORT" not in value and "${PORT}" not in value:
        return value
    port = os.getenv("PORT")
    if port is None or port.strip() == "":
        fail("PORT must be set when using $PORT placeholders")
    return value.replace("${PORT}", port.strip()).replace("$PORT", port.strip())


def ensure_key(path: Path) -> None:
    if path.exists():
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    print(f"[agentmesh-entrypoint] generating key at {path}")
    subprocess.run(
        ["/usr/local/bin/agentmesh", "keygen", "--out", str(path)],
        check=True,
    )


def read_pubkey_hex(key_path: Path) -> str:
    result = subprocess.run(
        ["/usr/local/bin/agentmesh", "pubkey", "--key", str(key_path)],
        check=True,
        capture_output=True,
        text=True,
    )
    pubkey_hex = result.stdout.strip()
    if not pubkey_hex:
        fail("agentmesh pubkey returned empty output")
    return pubkey_hex


def read_peer_id(key_path: Path) -> str:
    result = subprocess.run(
        ["/usr/local/bin/agentmesh", "peer-id", "--key", str(key_path)],
        check=True,
        capture_output=True,
        text=True,
    )
    peer_id = result.stdout.strip()
    if not peer_id:
        fail("agentmesh peer-id returned empty output")
    return peer_id


def compute_agent_did(pubkey_hex: str) -> str:
    try:
        raw = bytes.fromhex(pubkey_hex)
    except ValueError:
        fail("agentmesh pubkey hex invalid")
        return ""
    did_suffix = base64.b64encode(raw).decode("ascii")
    return f"did:anet:agent:{did_suffix}"


def main() -> None:
    key_path = Path(env_optional("AGENTMESH_KEY_PATH", "/var/lib/agentnet/keys/agentmesh.key"))
    state_dir = Path(env_optional("AGENTMESH_STATE_DIR", "/var/lib/agentnet/state"))
    state_dir.mkdir(parents=True, exist_ok=True)

    ensure_key(key_path)

    pubkey_hex = read_pubkey_hex(key_path)
    peer_id = read_peer_id(key_path)
    agent_did_value = env_optional("AGENTMESH_AGENT_DID")
    if agent_did_value is None or agent_did_value.lower() == "auto":
        agent_did_value = compute_agent_did(pubkey_hex)

    pubsub_econ_cmd = env_json("AGENTMESH_PUBSUB_ECON_CMD")
    if not isinstance(pubsub_econ_cmd, list) or not pubsub_econ_cmd:
        fail("AGENTMESH_PUBSUB_ECON_CMD must be a JSON array command")

    tx_sender_keys = env_json("AGENTMESH_TX_SENDER_PUBKEYS_JSON")
    if not isinstance(tx_sender_keys, list):
        fail("AGENTMESH_TX_SENDER_PUBKEYS_JSON must be a JSON array")
    if not tx_sender_keys:
        tx_sender_keys = []
    if not any(isinstance(entry, dict) and entry.get("did") == agent_did_value for entry in tx_sender_keys):
        tx_sender_keys.append({"did": agent_did_value, "pubkey_hex": pubkey_hex})

    budget_caps = env_json("AGENTMESH_TX_BUDGET_CAPS_JSON")
    if not isinstance(budget_caps, list) or not budget_caps:
        fail("AGENTMESH_TX_BUDGET_CAPS_JSON must be a non-empty JSON array")

    agentmail_sender_keys = env_json("AGENTMESH_AGENTMAIL_SENDER_PUBKEYS_JSON")
    if not isinstance(agentmail_sender_keys, list):
        fail("AGENTMESH_AGENTMAIL_SENDER_PUBKEYS_JSON must be a JSON array")
    if not agentmail_sender_keys:
        agentmail_sender_keys = []
    if not any(isinstance(entry, dict) and entry.get("did") == agent_did_value for entry in agentmail_sender_keys):
        agentmail_sender_keys.append({"did": agent_did_value, "pubkey_hex": pubkey_hex})

    dht_service_records = env_json("AGENTMESH_DHT_SERVICE_RECORDS_JSON")
    if not isinstance(dht_service_records, list):
        fail("AGENTMESH_DHT_SERVICE_RECORDS_JSON must be a JSON array")

    dht_community_record = None
    dht_community_raw = env_optional("AGENTMESH_DHT_COMMUNITY_RECORD_JSON")
    if dht_community_raw:
        try:
            dht_community_record = json.loads(dht_community_raw)
        except json.JSONDecodeError as exc:
            fail(f"invalid JSON for AGENTMESH_DHT_COMMUNITY_RECORD_JSON: {exc}")
        if not isinstance(dht_community_record, dict):
            fail("AGENTMESH_DHT_COMMUNITY_RECORD_JSON must be a JSON object")

    voucher_issuers = env_json("ANET_ECON_VOUCHER_ISSUERS_JSON")
    if not isinstance(voucher_issuers, list) or not voucher_issuers:
        fail("ANET_ECON_VOUCHER_ISSUERS_JSON must be a non-empty JSON array")

    onchain_enabled = env_bool_optional("ANET_ECON_ONCHAIN_ENABLED", False)

    econ_config = {
        "voucher": {
            "issuers": voucher_issuers,
            "nonce_state_path": env_required("ANET_ECON_VOUCHER_NONCE_STATE_PATH"),
            "max_clock_skew_sec": env_int("ANET_ECON_VOUCHER_MAX_CLOCK_SKEW_SEC"),
            "require_topic_match": env_bool("ANET_ECON_VOUCHER_REQUIRE_TOPIC_MATCH"),
            "allowed_purposes": env_list("ANET_ECON_VOUCHER_ALLOWED_PURPOSES"),
        },
    }
    if onchain_enabled:
        econ_config["onchain"] = {
            "enabled": True,
            "chain_id": env_required("ANET_ECON_ONCHAIN_CHAIN_ID"),
            "rpc_url": env_required("ANET_ECON_ONCHAIN_RPC_URL"),
            "min_confirmations": env_int("ANET_ECON_ONCHAIN_MIN_CONFIRMATIONS"),
            "require_success": env_bool("ANET_ECON_ONCHAIN_REQUIRE_SUCCESS"),
            "max_tx_age_sec": env_int("ANET_ECON_ONCHAIN_MAX_TX_AGE_SEC"),
            "required_to": env_required("ANET_ECON_ONCHAIN_REQUIRED_TO"),
            "required_from": env_required("ANET_ECON_ONCHAIN_REQUIRED_FROM"),
        }

    escrow_arbitrators = env_list_optional("AGENTMESH_TX_ESCROW_ARBITRATORS")
    if not escrow_arbitrators:
        escrow_arbitrators = [agent_did_value]

    agent_record_key = env_required("AGENTMESH_DHT_AGENT_RECORD_KEY")
    if agent_record_key.lower() == "auto":
        agent_record_key = f"agentnet/agent/{agent_did_value}"

    listen_addrs = env_list("AGENTMESH_LISTEN_ADDRS")
    public_ws = env_optional("AGENTMESH_PUBLIC_WS")

    config = {
        "chain_id": env_required("AGENTMESH_CHAIN_ID"),
        "agent_did": agent_did_value,
        "key_path": str(key_path),
        "node_id": env_optional("AGENTMESH_NODE_ID"),
        "state_dir": str(state_dir),
        "listen_addrs": listen_addrs,
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
                "allow_register": env_bool("AGENTMESH_TX_IDENTITY_ALLOW_REGISTER"),
                "allow_rotate": env_bool("AGENTMESH_TX_IDENTITY_ALLOW_ROTATE"),
                "allow_revoke": env_bool("AGENTMESH_TX_IDENTITY_ALLOW_REVOKE"),
                "max_clock_skew_sec": env_int("AGENTMESH_TX_IDENTITY_MAX_CLOCK_SKEW_SEC"),
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
                "allow_publish": env_bool("AGENTMESH_TX_SKILL_ALLOW_PUBLISH"),
                "allow_update": env_bool("AGENTMESH_TX_SKILL_ALLOW_UPDATE"),
                "allow_revoke": env_bool("AGENTMESH_TX_SKILL_ALLOW_REVOKE"),
                "max_clock_skew_sec": env_int("AGENTMESH_TX_SKILL_MAX_CLOCK_SKEW_SEC"),
            },
            "work_registry": {
                "enabled": env_bool("AGENTMESH_TX_WORK_ENABLED"),
                "state_path": env_required("AGENTMESH_TX_WORK_STATE_PATH"),
                "allow_offer_publish": env_bool("AGENTMESH_TX_WORK_ALLOW_OFFER_PUBLISH"),
                "allow_agreement_publish": env_bool("AGENTMESH_TX_WORK_ALLOW_AGREEMENT_PUBLISH"),
                "allow_agreement_update": env_bool("AGENTMESH_TX_WORK_ALLOW_AGREEMENT_UPDATE"),
                "allow_agreement_close": env_bool("AGENTMESH_TX_WORK_ALLOW_AGREEMENT_CLOSE"),
                "max_clock_skew_sec": env_int("AGENTMESH_TX_WORK_MAX_CLOCK_SKEW_SEC"),
            },
            "escrow": {
                "enabled": env_bool("AGENTMESH_TX_ESCROW_ENABLED"),
                "state_path": env_required("AGENTMESH_TX_ESCROW_STATE_PATH"),
                "log_path": env_required("AGENTMESH_TX_ESCROW_LOG_PATH"),
                "arbitrators": escrow_arbitrators,
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
                "record_key": agent_record_key,
                "agent_pubkeys_hex": env_list_optional("AGENTMESH_DHT_AGENT_PUBKEYS_HEX"),
                "capabilities": env_list("AGENTMESH_DHT_AGENT_CAPABILITIES"),
                "expires_sec": env_int("AGENTMESH_DHT_AGENT_EXPIRES_SEC"),
                "signing_key_path": env_optional("AGENTMESH_DHT_AGENT_SIGNING_KEY_PATH"),
            },
            "service_records": dht_service_records,
            "community_record": dht_community_record,
        },
    }

    mesh_info = {
        "agent_did": agent_did_value,
        "peer_id": peer_id,
        "listen_addrs": listen_addrs,
        "public_ws": public_ws,
        "updated_at": int(time.time()),
    }
    mesh_info_path = state_dir / "mesh_info.json"
    mesh_info_path.write_text(json.dumps(mesh_info))
    print(f"[agentmesh-entrypoint] wrote mesh info to {mesh_info_path}")

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
