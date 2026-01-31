import json
import sys
from pathlib import Path

from .cbor import decode_canonical, encode_canonical, CborMap
from .crypto import sha256, verify_ed25519_hash
from .skill import (
    parse_skill_manifest_payload,
    parse_skill_publish_payload,
    parse_skill_update_payload,
    parse_skill_revoke_payload,
    verify_skill_manifest,
)
from .work import (
    parse_work_offer_payload,
    parse_work_agreement_payload,
    parse_work_offer_publish_payload,
    parse_work_agreement_publish_payload,
    parse_work_agreement_update_payload,
    parse_work_agreement_close_payload,
    verify_work_offer,
    verify_work_agreement,
)
from .agentmail import (
    parse_agentmail_payload,
    parse_agentmail_message,
)


class VectorError(ValueError):
    pass


def _decode_hex(value, field):
    if value is None:
        raise VectorError(f"missing {field}")
    return bytes.fromhex(value)


def _ensure_roundtrip(label, cbor_bytes):
    value = decode_canonical(cbor_bytes)
    encoded = encode_canonical(value)
    if encoded != cbor_bytes:
        raise VectorError(f"{label} canonical roundtrip mismatch")


def _verify_hash_and_sig(label, public_key, cbor_bytes, expected_hash, signature):
    digest = sha256(cbor_bytes)
    if digest != expected_hash:
        raise VectorError(f"{label} hash mismatch")
    verify_ed25519_hash(public_key, digest, signature)


def _is_int(value):
    return isinstance(value, int) and not isinstance(value, bool)


def _parse_kill_switch_map(value):
    if not isinstance(value, CborMap):
        raise VectorError("kill switch map must be cbor map")
    action = None
    reason = None
    ts = None
    nonce = None
    signature = None
    for key, val in value.entries:
        if not _is_int(key):
            continue
        if key == 0 and _is_int(val) and 0 <= val <= 255:
            action = val
        elif key == 1 and isinstance(val, str):
            reason = val
        elif key == 2 and _is_int(val) and val >= 0:
            ts = val
        elif key == 3 and isinstance(val, (bytes, bytearray)):
            nonce = bytes(val)
        elif key == 4 and isinstance(val, (bytes, bytearray)):
            signature = bytes(val)
    if action is None:
        raise VectorError("kill switch action missing")
    if reason is None:
        raise VectorError("kill switch reason missing")
    if ts is None:
        raise VectorError("kill switch ts missing")
    if nonce is None:
        raise VectorError("kill switch nonce missing")
    return {
        "action": action,
        "reason": reason,
        "ts": ts,
        "nonce": nonce,
        "signature": signature,
    }


def _ensure_kill_switch_parts(label, parts, require_signature):
    if parts["action"] not in (0, 1):
        raise VectorError(f"{label} invalid action")
    if len(parts["nonce"]) != 16:
        raise VectorError(f"{label} nonce length invalid")
    if require_signature:
        sig = parts.get("signature")
        if sig is None:
            raise VectorError(f"{label} signature missing")
        if len(sig) != 64:
            raise VectorError(f"{label} signature length invalid")


def main():
    if len(sys.argv) != 2:
        raise SystemExit("usage: agentnet-vectors <path-to-vectors.json>")

    path = Path(sys.argv[1])
    data = json.loads(path.read_text())

    public_key = bytes.fromhex(data["ed25519_public_key_hex"])
    action_intent_hash = None

    for entry in data["vectors"]:
        vector_id = entry["id"]
        if vector_id == "TV1_ActionIntent":
            cbor_bytes = _decode_hex(entry.get("object_cbor_hex"), "object_cbor_hex")
            expected_hash = _decode_hex(entry.get("sha256_hex"), "sha256_hex")
            signature = _decode_hex(entry.get("signature_hex"), "signature_hex")
            _verify_hash_and_sig(vector_id, public_key, cbor_bytes, expected_hash, signature)
            _ensure_roundtrip(vector_id, cbor_bytes)
            action_intent_hash = expected_hash
        elif vector_id == "TV2_Approval":
            payload = _decode_hex(entry.get("approval_payload_cbor_hex"), "approval_payload_cbor_hex")
            expected_hash = _decode_hex(entry.get("approval_payload_sha256_hex"), "approval_payload_sha256_hex")
            signature = _decode_hex(entry.get("approval_signature_hex"), "approval_signature_hex")
            intent_hash = _decode_hex(entry.get("intent_hash_hex"), "intent_hash_hex")
            if entry.get("approval_full_object_cbor_hex"):
                full = bytes.fromhex(entry["approval_full_object_cbor_hex"])
                _ensure_roundtrip("TV2_Approval full object", full)
            _verify_hash_and_sig(vector_id, public_key, payload, expected_hash, signature)
            _ensure_roundtrip(vector_id, payload)
            if action_intent_hash is not None and intent_hash != action_intent_hash:
                raise VectorError("TV2_Approval intent hash mismatch")
        elif vector_id == "TV3_Grant":
            payload = _decode_hex(entry.get("grant_payload_cbor_hex"), "grant_payload_cbor_hex")
            expected_hash = _decode_hex(entry.get("grant_payload_sha256_hex"), "grant_payload_sha256_hex")
            signature = _decode_hex(entry.get("grant_signature_hex"), "grant_signature_hex")
            if entry.get("grant_full_object_cbor_hex"):
                full = bytes.fromhex(entry["grant_full_object_cbor_hex"])
                _ensure_roundtrip("TV3_Grant full object", full)
            _verify_hash_and_sig(vector_id, public_key, payload, expected_hash, signature)
            _ensure_roundtrip(vector_id, payload)
        elif vector_id == "TV4_NodeHello":
            payload = _decode_hex(entry.get("nodehello_payload_cbor_hex"), "nodehello_payload_cbor_hex")
            expected_hash = _decode_hex(entry.get("nodehello_payload_sha256_hex"), "nodehello_payload_sha256_hex")
            signature = _decode_hex(entry.get("nodehello_signature_hex"), "nodehello_signature_hex")
            _verify_hash_and_sig(vector_id, public_key, payload, expected_hash, signature)
            _ensure_roundtrip(vector_id, payload)
        elif vector_id == "TV5_ReceiptChain":
            receipt1 = _decode_hex(entry.get("receipt1_payload_cbor_hex"), "receipt1_payload_cbor_hex")
            receipt1_hash = _decode_hex(entry.get("receipt1_hash_hex"), "receipt1_hash_hex")
            receipt1_sig = _decode_hex(entry.get("receipt1_sig_hex"), "receipt1_sig_hex")
            receipt2 = _decode_hex(entry.get("receipt2_payload_cbor_hex"), "receipt2_payload_cbor_hex")
            receipt2_hash = _decode_hex(entry.get("receipt2_hash_hex"), "receipt2_hash_hex")
            receipt2_sig = _decode_hex(entry.get("receipt2_sig_hex"), "receipt2_sig_hex")
            receipt2_prev = _decode_hex(entry.get("receipt2_prev_hash_hex"), "receipt2_prev_hash_hex")
            _verify_hash_and_sig("TV5_ReceiptChain receipt1", public_key, receipt1, receipt1_hash, receipt1_sig)
            _verify_hash_and_sig("TV5_ReceiptChain receipt2", public_key, receipt2, receipt2_hash, receipt2_sig)
            if receipt2_prev != receipt1_hash:
                raise VectorError("TV5_ReceiptChain prev hash mismatch")
            _ensure_roundtrip("TV5_ReceiptChain receipt1", receipt1)
            _ensure_roundtrip("TV5_ReceiptChain receipt2", receipt2)
        elif vector_id == "TV6_EscrowLockTx":
            payload = _decode_hex(entry.get("tx_envelope_payload_cbor_hex"), "tx_envelope_payload_cbor_hex")
            expected_hash = _decode_hex(entry.get("tx_envelope_payload_sha256_hex"), "tx_envelope_payload_sha256_hex")
            signature = _decode_hex(entry.get("tx_signature_hex"), "tx_signature_hex")
            _verify_hash_and_sig(vector_id, public_key, payload, expected_hash, signature)
            _ensure_roundtrip(vector_id, payload)
        elif vector_id == "TV7_KillSwitch":
            payload = _decode_hex(entry.get("kill_switch_payload_cbor_hex"), "kill_switch_payload_cbor_hex")
            expected_hash = _decode_hex(entry.get("kill_switch_payload_sha256_hex"), "kill_switch_payload_sha256_hex")
            signature = _decode_hex(entry.get("kill_switch_signature_hex"), "kill_switch_signature_hex")
            full = _decode_hex(entry.get("kill_switch_full_object_cbor_hex"), "kill_switch_full_object_cbor_hex")
            _verify_hash_and_sig(vector_id, public_key, payload, expected_hash, signature)
            _ensure_roundtrip("TV7_KillSwitch payload", payload)
            _ensure_roundtrip("TV7_KillSwitch full object", full)
            payload_value = decode_canonical(payload)
            payload_parts = _parse_kill_switch_map(payload_value)
            if payload_parts.get("signature") is not None:
                raise VectorError("TV7_KillSwitch payload must not include signature")
            _ensure_kill_switch_parts("TV7_KillSwitch payload", payload_parts, False)

            full_value = decode_canonical(full)
            full_parts = _parse_kill_switch_map(full_value)
            _ensure_kill_switch_parts("TV7_KillSwitch full object", full_parts, True)
            if full_parts.get("signature") != signature:
                raise VectorError("TV7_KillSwitch signature mismatch")
            if (
                payload_parts["action"] != full_parts["action"]
                or payload_parts["reason"] != full_parts["reason"]
                or payload_parts["ts"] != full_parts["ts"]
                or payload_parts["nonce"] != full_parts["nonce"]
            ):
                raise VectorError("TV7_KillSwitch full object fields mismatch")
            reconstructed = encode_canonical(
                CborMap([
                    (0, payload_parts["action"]),
                    (1, payload_parts["reason"]),
                    (2, payload_parts["ts"]),
                    (3, payload_parts["nonce"]),
                ])
            )
            if reconstructed != payload:
                raise VectorError("TV7_KillSwitch payload reconstruction mismatch")
        elif vector_id == "TV18_AgentMailMessage":
            payload = _decode_hex(entry.get("agentmail_payload_cbor_hex"), "agentmail_payload_cbor_hex")
            expected_hash = _decode_hex(entry.get("agentmail_payload_sha256_hex"), "agentmail_payload_sha256_hex")
            signature = _decode_hex(entry.get("agentmail_signature_hex"), "agentmail_signature_hex")
            _verify_hash_and_sig(vector_id, public_key, payload, expected_hash, signature)
            _ensure_roundtrip("TV18_AgentMailMessage payload", payload)
            parse_agentmail_payload(decode_canonical(payload))
            if entry.get("agentmail_full_object_cbor_hex"):
                full = bytes.fromhex(entry["agentmail_full_object_cbor_hex"])
                _ensure_roundtrip("TV18_AgentMailMessage full object", full)
                message = parse_agentmail_message(decode_canonical(full))
                if message.signature != signature:
                    raise VectorError("TV18_AgentMailMessage signature mismatch")
        elif vector_id == "TV8_SkillManifest":
            payload = _decode_hex(entry.get("object_cbor_hex"), "object_cbor_hex")
            expected_hash = _decode_hex(entry.get("sha256_hex"), "sha256_hex")
            signature = _decode_hex(entry.get("signature_hex"), "signature_hex")
            _verify_hash_and_sig(vector_id, public_key, payload, expected_hash, signature)
            _ensure_roundtrip(vector_id, payload)
            parse_skill_manifest_payload(decode_canonical(payload))
            if entry.get("skill_manifest_full_object_cbor_hex"):
                full = bytes.fromhex(entry["skill_manifest_full_object_cbor_hex"])
                _ensure_roundtrip("TV8_SkillManifest full object", full)
                verify_skill_manifest(full, public_key)
        elif vector_id == "TV9_WorkOffer":
            payload = _decode_hex(entry.get("work_offer_payload_cbor_hex"), "work_offer_payload_cbor_hex")
            expected_hash = _decode_hex(entry.get("work_offer_payload_sha256_hex"), "work_offer_payload_sha256_hex")
            signature = _decode_hex(entry.get("work_offer_signature_hex"), "work_offer_signature_hex")
            _verify_hash_and_sig(vector_id, public_key, payload, expected_hash, signature)
            _ensure_roundtrip(vector_id, payload)
            parse_work_offer_payload(decode_canonical(payload))
            if entry.get("work_offer_full_object_cbor_hex"):
                full = bytes.fromhex(entry["work_offer_full_object_cbor_hex"])
                _ensure_roundtrip("TV9_WorkOffer full object", full)
                verify_work_offer(full, public_key)
        elif vector_id == "TV10_WorkAgreement":
            payload = _decode_hex(entry.get("work_agreement_payload_cbor_hex"), "work_agreement_payload_cbor_hex")
            expected_hash = _decode_hex(entry.get("work_agreement_payload_sha256_hex"), "work_agreement_payload_sha256_hex")
            signature = _decode_hex(entry.get("work_agreement_signature_hex"), "work_agreement_signature_hex")
            _verify_hash_and_sig(vector_id, public_key, payload, expected_hash, signature)
            _ensure_roundtrip(vector_id, payload)
            parse_work_agreement_payload(decode_canonical(payload))
            if entry.get("work_agreement_full_object_cbor_hex"):
                full = bytes.fromhex(entry["work_agreement_full_object_cbor_hex"])
                _ensure_roundtrip("TV10_WorkAgreement full object", full)
                verify_work_agreement(full, public_key)
        elif vector_id == "TV11_SkillPublishPayload":
            payload = _decode_hex(entry.get("skill_publish_payload_cbor_hex"), "skill_publish_payload_cbor_hex")
            expected_hash = _decode_hex(entry.get("skill_publish_payload_sha256_hex"), "skill_publish_payload_sha256_hex")
            digest = sha256(payload)
            if digest != expected_hash:
                raise VectorError("TV11_SkillPublishPayload hash mismatch")
            _ensure_roundtrip(vector_id, payload)
            parse_skill_publish_payload(decode_canonical(payload))
        elif vector_id == "TV12_SkillUpdatePayload":
            payload = _decode_hex(entry.get("skill_update_payload_cbor_hex"), "skill_update_payload_cbor_hex")
            expected_hash = _decode_hex(entry.get("skill_update_payload_sha256_hex"), "skill_update_payload_sha256_hex")
            digest = sha256(payload)
            if digest != expected_hash:
                raise VectorError("TV12_SkillUpdatePayload hash mismatch")
            _ensure_roundtrip(vector_id, payload)
            parse_skill_update_payload(decode_canonical(payload))
        elif vector_id == "TV13_SkillRevokePayload":
            payload = _decode_hex(entry.get("skill_revoke_payload_cbor_hex"), "skill_revoke_payload_cbor_hex")
            expected_hash = _decode_hex(entry.get("skill_revoke_payload_sha256_hex"), "skill_revoke_payload_sha256_hex")
            digest = sha256(payload)
            if digest != expected_hash:
                raise VectorError("TV13_SkillRevokePayload hash mismatch")
            _ensure_roundtrip(vector_id, payload)
            parse_skill_revoke_payload(decode_canonical(payload))
        elif vector_id == "TV14_WorkOfferPublishPayload":
            payload = _decode_hex(entry.get("work_offer_publish_payload_cbor_hex"), "work_offer_publish_payload_cbor_hex")
            expected_hash = _decode_hex(entry.get("work_offer_publish_payload_sha256_hex"), "work_offer_publish_payload_sha256_hex")
            digest = sha256(payload)
            if digest != expected_hash:
                raise VectorError("TV14_WorkOfferPublishPayload hash mismatch")
            _ensure_roundtrip(vector_id, payload)
            parse_work_offer_publish_payload(decode_canonical(payload))
        elif vector_id == "TV15_WorkAgreementPublishPayload":
            payload = _decode_hex(entry.get("work_agreement_publish_payload_cbor_hex"), "work_agreement_publish_payload_cbor_hex")
            expected_hash = _decode_hex(entry.get("work_agreement_publish_payload_sha256_hex"), "work_agreement_publish_payload_sha256_hex")
            digest = sha256(payload)
            if digest != expected_hash:
                raise VectorError("TV15_WorkAgreementPublishPayload hash mismatch")
            _ensure_roundtrip(vector_id, payload)
            parse_work_agreement_publish_payload(decode_canonical(payload))
        elif vector_id == "TV16_WorkAgreementUpdatePayload":
            payload = _decode_hex(entry.get("work_agreement_update_payload_cbor_hex"), "work_agreement_update_payload_cbor_hex")
            expected_hash = _decode_hex(entry.get("work_agreement_update_payload_sha256_hex"), "work_agreement_update_payload_sha256_hex")
            digest = sha256(payload)
            if digest != expected_hash:
                raise VectorError("TV16_WorkAgreementUpdatePayload hash mismatch")
            _ensure_roundtrip(vector_id, payload)
            parse_work_agreement_update_payload(decode_canonical(payload))
        elif vector_id == "TV17_WorkAgreementClosePayload":
            payload = _decode_hex(entry.get("work_agreement_close_payload_cbor_hex"), "work_agreement_close_payload_cbor_hex")
            expected_hash = _decode_hex(entry.get("work_agreement_close_payload_sha256_hex"), "work_agreement_close_payload_sha256_hex")
            digest = sha256(payload)
            if digest != expected_hash:
                raise VectorError("TV17_WorkAgreementClosePayload hash mismatch")
            _ensure_roundtrip(vector_id, payload)
            parse_work_agreement_close_payload(decode_canonical(payload))
        else:
            raise VectorError(f"unknown vector id: {vector_id}")

    print("vector verification complete")


if __name__ == "__main__":
    main()
