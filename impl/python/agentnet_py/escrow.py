from __future__ import annotations

from dataclasses import dataclass
from typing import Optional

from .cbor import CborMap, CborValue


class EscrowError(ValueError):
    pass


@dataclass
class EscrowLockPayload:
    escrow_id: str
    payer: str
    payee: str
    amount: int
    currency: str
    release_condition: CborValue
    dispute_window_sec: int
    expiry: int


@dataclass
class EscrowReleasePayload:
    escrow_id: str
    evidence_receipt_hash: bytes
    ts: int


@dataclass
class EscrowDisputePayload:
    escrow_id: str
    reason: str
    evidence_anchor_or_receipt: bytes
    ts: int


@dataclass
class EscrowResolvePayload:
    escrow_id: str
    outcome: int
    split_amount_to_payee: Optional[int]
    ts: int


def parse_escrow_lock_payload(value: CborValue) -> EscrowLockPayload:
    entries = _expect_map(value)
    return EscrowLockPayload(
        escrow_id=_expect_text(_get_required(entries, 0)),
        payer=_expect_text(_get_required(entries, 1)),
        payee=_expect_text(_get_required(entries, 2)),
        amount=_expect_u64(_get_required(entries, 3)),
        currency=_expect_text(_get_required(entries, 4)),
        release_condition=_get_required(entries, 5),
        dispute_window_sec=_expect_u64(_get_required(entries, 6)),
        expiry=_expect_u64(_get_required(entries, 7)),
    )


def parse_escrow_release_payload(value: CborValue) -> EscrowReleasePayload:
    entries = _expect_map(value)
    evidence = _expect_bytes(_get_required(entries, 1))
    if len(evidence) != 32:
        raise EscrowError("evidence_receipt_hash must be 32 bytes")
    return EscrowReleasePayload(
        escrow_id=_expect_text(_get_required(entries, 0)),
        evidence_receipt_hash=evidence,
        ts=_expect_u64(_get_required(entries, 2)),
    )


def parse_escrow_dispute_payload(value: CborValue) -> EscrowDisputePayload:
    entries = _expect_map(value)
    evidence = _expect_bytes(_get_required(entries, 2))
    if len(evidence) != 32:
        raise EscrowError("evidence_anchor_or_receipt must be 32 bytes")
    return EscrowDisputePayload(
        escrow_id=_expect_text(_get_required(entries, 0)),
        reason=_expect_text(_get_required(entries, 1)),
        evidence_anchor_or_receipt=evidence,
        ts=_expect_u64(_get_required(entries, 3)),
    )


def parse_escrow_resolve_payload(value: CborValue) -> EscrowResolvePayload:
    entries = _expect_map(value)
    split_amount = None
    if _get_optional(entries, 2) is not None:
        split_amount = _expect_u64(_get_optional(entries, 2))
    return EscrowResolvePayload(
        escrow_id=_expect_text(_get_required(entries, 0)),
        outcome=_expect_u8(_get_required(entries, 1)),
        split_amount_to_payee=split_amount,
        ts=_expect_u64(_get_required(entries, 3)),
    )


def escrow_lock_payload_to_cbor(payload: EscrowLockPayload) -> CborValue:
    return CborMap([
        (0, payload.escrow_id),
        (1, payload.payer),
        (2, payload.payee),
        (3, payload.amount),
        (4, payload.currency),
        (5, payload.release_condition),
        (6, payload.dispute_window_sec),
        (7, payload.expiry),
    ])


def escrow_release_payload_to_cbor(payload: EscrowReleasePayload) -> CborValue:
    return CborMap([
        (0, payload.escrow_id),
        (1, payload.evidence_receipt_hash),
        (2, payload.ts),
    ])


def escrow_dispute_payload_to_cbor(payload: EscrowDisputePayload) -> CborValue:
    return CborMap([
        (0, payload.escrow_id),
        (1, payload.reason),
        (2, payload.evidence_anchor_or_receipt),
        (3, payload.ts),
    ])


def escrow_resolve_payload_to_cbor(payload: EscrowResolvePayload) -> CborValue:
    entries = [
        (0, payload.escrow_id),
        (1, payload.outcome),
        (3, payload.ts),
    ]
    if payload.split_amount_to_payee is not None:
        entries.append((2, payload.split_amount_to_payee))
    return CborMap(entries)


def _expect_map(value: CborValue):
    if isinstance(value, CborMap):
        return value.entries
    raise EscrowError("expected map")


def _get_required(entries, key: int) -> CborValue:
    for k, v in entries:
        if isinstance(k, int) and k == key:
            return v
    raise EscrowError("missing required key")


def _get_optional(entries, key: int) -> Optional[CborValue]:
    for k, v in entries:
        if isinstance(k, int) and k == key:
            return v
    return None


def _expect_text(value: CborValue) -> str:
    if isinstance(value, str):
        return value
    raise EscrowError("expected text")


def _expect_bytes(value: CborValue) -> bytes:
    if isinstance(value, (bytes, bytearray)):
        return bytes(value)
    raise EscrowError("expected bytes")


def _expect_u64(value: CborValue) -> int:
    if isinstance(value, int) and value >= 0:
        return value
    raise EscrowError("expected unsigned")


def _expect_u8(value: CborValue) -> int:
    if isinstance(value, int) and 0 <= value <= 0xFF:
        return value
    raise EscrowError("expected u8")
