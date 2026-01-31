from __future__ import annotations

from dataclasses import dataclass
from typing import Optional, Tuple

from .cbor import CborMap, CborValue, decode_canonical, encode_canonical
from .crypto import sha256, verify_ed25519_hash
from .sign import sign_ed25519_hash


class PubSubError(ValueError):
    pass


@dataclass
class EconomicProof:
    kind: int
    data: bytes

    @staticmethod
    def onchain_tx(tx_hash: bytes) -> "EconomicProof":
        return EconomicProof(1, tx_hash)

    @staticmethod
    def voucher(voucher: bytes) -> "EconomicProof":
        return EconomicProof(2, voucher)

    def to_cbor(self) -> CborValue:
        return CborMap([(0, self.kind), (1, self.data)])


@dataclass
class PubSubEnvelopePayload:
    version: int
    topic: str
    sender: str
    ts: int
    seq: int
    payload_type: int
    payload: CborValue
    economic_proof: Optional[EconomicProof]


@dataclass
class PubSubEnvelope:
    payload: PubSubEnvelopePayload
    signature: bytes


def parse_pubsub_payload(value: CborValue) -> PubSubEnvelopePayload:
    entries = _expect_map(value)
    version = _expect_u8(_get_required(entries, 0))
    topic = _expect_text(_get_required(entries, 1))
    sender = _expect_text(_get_required(entries, 2))
    ts = _expect_u64(_get_required(entries, 3))
    seq = _expect_u64(_get_required(entries, 4))
    payload_type = _expect_u16(_get_required(entries, 5))
    payload = _get_required(entries, 6)
    economic_proof = None
    if _get_optional(entries, 7) is not None:
        economic_proof = _parse_economic_proof(_get_optional(entries, 7))
    return PubSubEnvelopePayload(
        version=version,
        topic=topic,
        sender=sender,
        ts=ts,
        seq=seq,
        payload_type=payload_type,
        payload=payload,
        economic_proof=economic_proof,
    )


def parse_pubsub_envelope(value: CborValue) -> PubSubEnvelope:
    payload, signature = _split_signed_map(value, 8)
    payload_obj = parse_pubsub_payload(payload)
    return PubSubEnvelope(payload=payload_obj, signature=signature)


def decode_pubsub_envelope(data: bytes) -> PubSubEnvelope:
    return parse_pubsub_envelope(decode_canonical(data))


def build_pubsub_envelope(payload: PubSubEnvelopePayload, secret_key: bytes) -> bytes:
    payload_map = _pubsub_payload_to_cbor(payload)
    payload_cbor = encode_canonical(payload_map)
    digest = sha256(payload_cbor)
    signature = sign_ed25519_hash(secret_key, digest)
    signed = _with_signature(payload_map, 8, signature)
    return encode_canonical(signed)


def verify_pubsub_envelope(data: bytes, public_key: bytes) -> PubSubEnvelopePayload:
    value = decode_canonical(data)
    payload, signature = _split_signed_map(value, 8)
    payload_cbor = encode_canonical(payload)
    digest = sha256(payload_cbor)
    verify_ed25519_hash(public_key, digest, signature)
    return parse_pubsub_payload(payload)


def _pubsub_payload_to_cbor(payload: PubSubEnvelopePayload) -> CborValue:
    entries = [
        (0, payload.version),
        (1, payload.topic),
        (2, payload.sender),
        (3, payload.ts),
        (4, payload.seq),
        (5, payload.payload_type),
        (6, payload.payload),
    ]
    if payload.economic_proof is not None:
        entries.append((7, payload.economic_proof.to_cbor()))
    return CborMap(entries)


def _parse_economic_proof(value: CborValue) -> EconomicProof:
    entries = _expect_map(value)
    kind = _expect_u8(_get_required(entries, 0))
    data = _expect_bytes(_get_required(entries, 1))
    if kind == 1:
        if len(data) != 32:
            raise PubSubError("invalid onchain tx hash length")
        return EconomicProof.onchain_tx(data)
    if kind == 2:
        return EconomicProof.voucher(data)
    raise PubSubError("unsupported economic proof")


def _split_signed_map(value: CborValue, sig_key: int) -> Tuple[CborValue, bytes]:
    entries = _expect_map(value)
    payload_entries = []
    signature = None
    for key, val in entries:
        if isinstance(key, int) and key == sig_key:
            if signature is not None:
                raise PubSubError("duplicate signature key")
            if not isinstance(val, (bytes, bytearray)):
                raise PubSubError("signature must be bytes")
            signature = bytes(val)
            continue
        payload_entries.append((key, val))
    if signature is None:
        raise PubSubError("missing signature")
    if len(signature) != 64:
        raise PubSubError("invalid signature length")
    return CborMap(payload_entries), signature


def _with_signature(payload: CborValue, sig_key: int, signature: bytes) -> CborValue:
    entries = list(_expect_map(payload))
    entries.append((sig_key, signature))
    return CborMap(entries)


def _expect_map(value: CborValue):
    if isinstance(value, CborMap):
        return value.entries
    raise PubSubError("expected map")


def _get_required(entries, key: int) -> CborValue:
    for k, v in entries:
        if isinstance(k, int) and k == key:
            return v
    raise PubSubError("missing required key")


def _get_optional(entries, key: int) -> Optional[CborValue]:
    for k, v in entries:
        if isinstance(k, int) and k == key:
            return v
    return None


def _expect_text(value: CborValue) -> str:
    if isinstance(value, str):
        return value
    raise PubSubError("expected text")


def _expect_bytes(value: CborValue) -> bytes:
    if isinstance(value, (bytes, bytearray)):
        return bytes(value)
    raise PubSubError("expected bytes")


def _expect_u64(value: CborValue) -> int:
    if isinstance(value, int) and value >= 0:
        return value
    raise PubSubError("expected unsigned")


def _expect_u16(value: CborValue) -> int:
    if isinstance(value, int) and 0 <= value <= 0xFFFF:
        return value
    raise PubSubError("expected u16")


def _expect_u8(value: CborValue) -> int:
    if isinstance(value, int) and 0 <= value <= 0xFF:
        return value
    raise PubSubError("expected u8")
