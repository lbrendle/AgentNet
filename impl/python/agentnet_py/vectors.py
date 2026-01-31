import json
import sys
from pathlib import Path

from .cbor import decode_canonical, encode_canonical, CborMap
from .crypto import sha256, verify_ed25519_hash


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
        else:
            raise VectorError(f"unknown vector id: {vector_id}")

    print("vector verification complete")


if __name__ == "__main__":
    main()
