from __future__ import annotations

from dataclasses import dataclass

from .cbor import CborMap, CborValue


class EconomyError(ValueError):
    pass


@dataclass
class TransferPayload:
    from_did: str
    to_did: str
    amount: int
    currency: str
    ts: int


@dataclass
class PostagePayload:
    payer: str
    amount: int
    currency: str
    purpose: str
    ts: int


def parse_transfer_payload(value: CborValue) -> TransferPayload:
    entries = _expect_map(value)
    return TransferPayload(
        from_did=_expect_text(_get_required(entries, 0)),
        to_did=_expect_text(_get_required(entries, 1)),
        amount=_expect_u64(_get_required(entries, 2)),
        currency=_expect_text(_get_required(entries, 3)),
        ts=_expect_u64(_get_required(entries, 4)),
    )


def parse_postage_payload(value: CborValue) -> PostagePayload:
    entries = _expect_map(value)
    return PostagePayload(
        payer=_expect_text(_get_required(entries, 0)),
        amount=_expect_u64(_get_required(entries, 1)),
        currency=_expect_text(_get_required(entries, 2)),
        purpose=_expect_text(_get_required(entries, 3)),
        ts=_expect_u64(_get_required(entries, 4)),
    )


def transfer_payload_to_cbor(payload: TransferPayload) -> CborValue:
    return CborMap([
        (0, payload.from_did),
        (1, payload.to_did),
        (2, payload.amount),
        (3, payload.currency),
        (4, payload.ts),
    ])


def postage_payload_to_cbor(payload: PostagePayload) -> CborValue:
    return CborMap([
        (0, payload.payer),
        (1, payload.amount),
        (2, payload.currency),
        (3, payload.purpose),
        (4, payload.ts),
    ])


def _expect_map(value: CborValue):
    if isinstance(value, CborMap):
        return value.entries
    raise EconomyError("expected map")


def _get_required(entries, key: int) -> CborValue:
    for k, v in entries:
        if isinstance(k, int) and k == key:
            return v
    raise EconomyError("missing required key")


def _expect_text(value: CborValue) -> str:
    if isinstance(value, str):
        return value
    raise EconomyError("expected text")


def _expect_u64(value: CborValue) -> int:
    if isinstance(value, int) and value >= 0:
        return value
    raise EconomyError("expected unsigned")
