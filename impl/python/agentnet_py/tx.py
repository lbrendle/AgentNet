from __future__ import annotations

from dataclasses import dataclass
from typing import Tuple

from .cbor import CborMap, CborValue, decode_canonical, encode_canonical
from .crypto import sha256, verify_ed25519_hash
from .sign import sign_ed25519_hash


class TxError(ValueError):
    pass


@dataclass
class TxEnvelopePayload:
    tx_type: int
    sender: str
    nonce: int
    fee: int
    payload: CborValue


@dataclass
class TxEnvelope:
    payload: TxEnvelopePayload
    signature: bytes


def parse_tx_envelope_payload(value: CborValue) -> TxEnvelopePayload:
    entries = _expect_map(value)
    return TxEnvelopePayload(
        tx_type=_expect_u64(_get_required(entries, 0)),
        sender=_expect_text(_get_required(entries, 1)),
        nonce=_expect_u64(_get_required(entries, 2)),
        fee=_expect_u64(_get_required(entries, 3)),
        payload=_get_required(entries, 4),
    )


def parse_tx_envelope(value: CborValue) -> TxEnvelope:
    payload, signature = _split_signed_map(value, 5)
    return TxEnvelope(payload=parse_tx_envelope_payload(payload), signature=signature)


def decode_tx_envelope(data: bytes) -> TxEnvelope:
    return parse_tx_envelope(decode_canonical(data))


def build_tx_envelope(payload: TxEnvelopePayload, secret_key: bytes) -> bytes:
    payload_map = tx_envelope_payload_to_cbor(payload)
    payload_cbor = encode_canonical(payload_map)
    digest = sha256(payload_cbor)
    signature = sign_ed25519_hash(secret_key, digest)
    signed = _with_signature(payload_map, 5, signature)
    return encode_canonical(signed)


def verify_tx_envelope(data: bytes, public_key: bytes) -> TxEnvelopePayload:
    value = decode_canonical(data)
    payload, signature = _split_signed_map(value, 5)
    payload_cbor = encode_canonical(payload)
    digest = sha256(payload_cbor)
    verify_ed25519_hash(public_key, digest, signature)
    return parse_tx_envelope_payload(payload)


def tx_envelope_payload_to_cbor(payload: TxEnvelopePayload) -> CborValue:
    return CborMap([
        (0, payload.tx_type),
        (1, payload.sender),
        (2, payload.nonce),
        (3, payload.fee),
        (4, payload.payload),
    ])


def _split_signed_map(value: CborValue, sig_key: int) -> Tuple[CborValue, bytes]:
    entries = _expect_map(value)
    payload_entries = []
    signature = None
    for key, val in entries:
        if isinstance(key, int) and key == sig_key:
            if signature is not None:
                raise TxError("duplicate signature key")
            if not isinstance(val, (bytes, bytearray)):
                raise TxError("signature must be bytes")
            signature = bytes(val)
            continue
        payload_entries.append((key, val))
    if signature is None:
        raise TxError("missing signature")
    if len(signature) != 64:
        raise TxError("invalid signature length")
    return CborMap(payload_entries), signature


def _with_signature(payload: CborValue, sig_key: int, signature: bytes) -> CborValue:
    entries = list(_expect_map(payload))
    entries.append((sig_key, signature))
    return CborMap(entries)


def _expect_map(value: CborValue):
    if isinstance(value, CborMap):
        return value.entries
    raise TxError("expected map")


def _get_required(entries, key: int) -> CborValue:
    for k, v in entries:
        if isinstance(k, int) and k == key:
            return v
    raise TxError("missing required key")


def _expect_text(value: CborValue) -> str:
    if isinstance(value, str):
        return value
    raise TxError("expected text")


def _expect_u64(value: CborValue) -> int:
    if isinstance(value, int) and value >= 0:
        return value
    raise TxError("expected unsigned")
