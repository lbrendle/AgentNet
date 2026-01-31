from __future__ import annotations

from dataclasses import dataclass
from typing import List, Optional, Tuple

from .cbor import CborMap, CborValue, decode_canonical, encode_canonical
from .crypto import sha256, verify_ed25519_hash
from .sign import sign_ed25519_hash

WORK_SIG_KEY = 16


class WorkError(ValueError):
    pass


@dataclass
class WorkMilestone:
    milestone_id: str
    description: str
    due_ts: int
    amount: int
    deliverable_hash: Optional[bytes]


@dataclass
class WorkOfferPayload:
    offer_id: str
    issuer: str
    title: str
    summary: str
    scope: str
    budget_amount: int
    budget_currency: str
    duration_sec: int
    deliverables: List[str]
    requirements: Optional[List[str]]
    ts: int
    exp: int


@dataclass
class WorkOffer:
    payload: WorkOfferPayload
    signature: bytes


@dataclass
class WorkAgreementPayload:
    agreement_id: str
    offer_id: str
    issuer: str
    counterparty: str
    budget_amount: int
    budget_currency: str
    start_ts: int
    end_ts: int
    deliverables: List[str]
    milestones: Optional[List[WorkMilestone]]
    escrow_id: Optional[str]
    dispute_policy: Optional[CborValue]
    ts: int


@dataclass
class WorkAgreement:
    payload: WorkAgreementPayload
    signature: bytes


@dataclass
class WorkOfferPublishPayload:
    offer: bytes
    ts: int


@dataclass
class WorkAgreementPublishPayload:
    agreement: bytes
    ts: int


@dataclass
class WorkAgreementUpdatePayload:
    agreement_id: str
    prev_agreement_hash: bytes
    agreement: bytes
    ts: int


@dataclass
class WorkAgreementClosePayload:
    agreement_id: str
    agreement_hash: bytes
    reason: str
    ts: int


def parse_work_offer_payload(value: CborValue) -> WorkOfferPayload:
    entries = _expect_map(value)
    payload = WorkOfferPayload(
        offer_id=_expect_text(_get_required(entries, 0)),
        issuer=_expect_text(_get_required(entries, 1)),
        title=_expect_text(_get_required(entries, 2)),
        summary=_expect_text(_get_required(entries, 3)),
        scope=_expect_text(_get_required(entries, 4)),
        budget_amount=_expect_u64(_get_required(entries, 5)),
        budget_currency=_expect_text(_get_required(entries, 6)),
        duration_sec=_expect_u64(_get_required(entries, 7)),
        deliverables=_expect_text_array(_get_required(entries, 8)),
        requirements=_optional_text_array(entries, 9),
        ts=_expect_u64(_get_required(entries, 10)),
        exp=_expect_u64(_get_required(entries, 11)),
    )
    _validate_offer(payload)
    return payload


def parse_work_offer(value: CborValue) -> WorkOffer:
    payload, signature = _split_signed_map(value, WORK_SIG_KEY)
    payload_obj = parse_work_offer_payload(payload)
    return WorkOffer(payload=payload_obj, signature=signature)


def decode_work_offer(data: bytes) -> WorkOffer:
    return parse_work_offer(decode_canonical(data))


def build_work_offer(payload: WorkOfferPayload, secret_key: bytes) -> bytes:
    _validate_offer(payload)
    payload_cbor = _work_offer_payload_to_cbor(payload)
    payload_bytes = encode_canonical(payload_cbor)
    digest = sha256(payload_bytes)
    signature = sign_ed25519_hash(secret_key, digest)
    full = _with_signature(payload_cbor, WORK_SIG_KEY, signature)
    return encode_canonical(full)


def verify_work_offer(data: bytes, public_key: bytes) -> WorkOfferPayload:
    value = decode_canonical(data)
    payload, signature = _split_signed_map(value, WORK_SIG_KEY)
    payload_bytes = encode_canonical(payload)
    digest = sha256(payload_bytes)
    verify_ed25519_hash(public_key, digest, signature)
    return parse_work_offer_payload(payload)


def parse_work_agreement_payload(value: CborValue) -> WorkAgreementPayload:
    entries = _expect_map(value)
    payload = WorkAgreementPayload(
        agreement_id=_expect_text(_get_required(entries, 0)),
        offer_id=_expect_text(_get_required(entries, 1)),
        issuer=_expect_text(_get_required(entries, 2)),
        counterparty=_expect_text(_get_required(entries, 3)),
        budget_amount=_expect_u64(_get_required(entries, 4)),
        budget_currency=_expect_text(_get_required(entries, 5)),
        start_ts=_expect_u64(_get_required(entries, 6)),
        end_ts=_expect_u64(_get_required(entries, 7)),
        deliverables=_expect_text_array(_get_required(entries, 8)),
        milestones=_optional_milestones(entries, 9),
        escrow_id=_optional_text(entries, 10),
        dispute_policy=_get_optional(entries, 11),
        ts=_expect_u64(_get_required(entries, 12)),
    )
    _validate_agreement(payload)
    return payload


def parse_work_agreement(value: CborValue) -> WorkAgreement:
    payload, signature = _split_signed_map(value, WORK_SIG_KEY)
    payload_obj = parse_work_agreement_payload(payload)
    return WorkAgreement(payload=payload_obj, signature=signature)


def decode_work_agreement(data: bytes) -> WorkAgreement:
    return parse_work_agreement(decode_canonical(data))


def build_work_agreement(payload: WorkAgreementPayload, secret_key: bytes) -> bytes:
    _validate_agreement(payload)
    payload_cbor = _work_agreement_payload_to_cbor(payload)
    payload_bytes = encode_canonical(payload_cbor)
    digest = sha256(payload_bytes)
    signature = sign_ed25519_hash(secret_key, digest)
    full = _with_signature(payload_cbor, WORK_SIG_KEY, signature)
    return encode_canonical(full)


def verify_work_agreement(data: bytes, public_key: bytes) -> WorkAgreementPayload:
    value = decode_canonical(data)
    payload, signature = _split_signed_map(value, WORK_SIG_KEY)
    payload_bytes = encode_canonical(payload)
    digest = sha256(payload_bytes)
    verify_ed25519_hash(public_key, digest, signature)
    return parse_work_agreement_payload(payload)


def parse_work_offer_publish_payload(value: CborValue) -> WorkOfferPublishPayload:
    entries = _expect_map(value)
    offer = _expect_bytes(_get_required(entries, 0))
    ts = _expect_u64(_get_required(entries, 1))
    if ts == 0:
        raise WorkError("timestamp required")
    decode_work_offer(offer)
    return WorkOfferPublishPayload(offer=offer, ts=ts)


def parse_work_agreement_publish_payload(value: CborValue) -> WorkAgreementPublishPayload:
    entries = _expect_map(value)
    agreement = _expect_bytes(_get_required(entries, 0))
    ts = _expect_u64(_get_required(entries, 1))
    if ts == 0:
        raise WorkError("timestamp required")
    decode_work_agreement(agreement)
    return WorkAgreementPublishPayload(agreement=agreement, ts=ts)


def parse_work_agreement_update_payload(value: CborValue) -> WorkAgreementUpdatePayload:
    entries = _expect_map(value)
    agreement_id = _expect_text(_get_required(entries, 0))
    prev_hash = _expect_bytes_len(_get_required(entries, 1), 32)
    agreement = _expect_bytes(_get_required(entries, 2))
    ts = _expect_u64(_get_required(entries, 3))
    if ts == 0:
        raise WorkError("timestamp required")
    decode_work_agreement(agreement)
    return WorkAgreementUpdatePayload(
        agreement_id=agreement_id,
        prev_agreement_hash=prev_hash,
        agreement=agreement,
        ts=ts,
    )


def parse_work_agreement_close_payload(value: CborValue) -> WorkAgreementClosePayload:
    entries = _expect_map(value)
    agreement_id = _expect_text(_get_required(entries, 0))
    agreement_hash = _expect_bytes_len(_get_required(entries, 1), 32)
    reason = _expect_text(_get_required(entries, 2))
    ts = _expect_u64(_get_required(entries, 3))
    if ts == 0:
        raise WorkError("timestamp required")
    if reason.strip() == "":
        raise WorkError("reason required")
    return WorkAgreementClosePayload(
        agreement_id=agreement_id,
        agreement_hash=agreement_hash,
        reason=reason,
        ts=ts,
    )


def work_offer_publish_payload_to_cbor(payload: WorkOfferPublishPayload) -> CborValue:
    if payload.ts == 0:
        raise WorkError("timestamp required")
    decode_work_offer(payload.offer)
    return CborMap([(0, bytes(payload.offer)), (1, payload.ts)])


def work_agreement_publish_payload_to_cbor(payload: WorkAgreementPublishPayload) -> CborValue:
    if payload.ts == 0:
        raise WorkError("timestamp required")
    decode_work_agreement(payload.agreement)
    return CborMap([(0, bytes(payload.agreement)), (1, payload.ts)])


def work_agreement_update_payload_to_cbor(payload: WorkAgreementUpdatePayload) -> CborValue:
    if payload.ts == 0:
        raise WorkError("timestamp required")
    _ensure_nonempty(payload.agreement_id, "agreement id required")
    if len(payload.prev_agreement_hash) != 32:
        raise WorkError("invalid agreement hash length")
    decode_work_agreement(payload.agreement)
    return CborMap(
        [
            (0, payload.agreement_id),
            (1, bytes(payload.prev_agreement_hash)),
            (2, bytes(payload.agreement)),
            (3, payload.ts),
        ]
    )


def work_agreement_close_payload_to_cbor(payload: WorkAgreementClosePayload) -> CborValue:
    if payload.ts == 0:
        raise WorkError("timestamp required")
    _ensure_nonempty(payload.agreement_id, "agreement id required")
    _ensure_nonempty(payload.reason, "reason required")
    if len(payload.agreement_hash) != 32:
        raise WorkError("invalid agreement hash length")
    return CborMap(
        [
            (0, payload.agreement_id),
            (1, bytes(payload.agreement_hash)),
            (2, payload.reason),
            (3, payload.ts),
        ]
    )


def _work_offer_payload_to_cbor(payload: WorkOfferPayload) -> CborValue:
    _validate_offer(payload)
    entries: List[Tuple[CborValue, CborValue]] = [
        (0, payload.offer_id),
        (1, payload.issuer),
        (2, payload.title),
        (3, payload.summary),
        (4, payload.scope),
        (5, payload.budget_amount),
        (6, payload.budget_currency),
        (7, payload.duration_sec),
        (8, list(payload.deliverables)),
        (10, payload.ts),
        (11, payload.exp),
    ]
    if payload.requirements is not None:
        entries.append((9, list(payload.requirements)))
    return CborMap(entries)


def _work_agreement_payload_to_cbor(payload: WorkAgreementPayload) -> CborValue:
    _validate_agreement(payload)
    entries: List[Tuple[CborValue, CborValue]] = [
        (0, payload.agreement_id),
        (1, payload.offer_id),
        (2, payload.issuer),
        (3, payload.counterparty),
        (4, payload.budget_amount),
        (5, payload.budget_currency),
        (6, payload.start_ts),
        (7, payload.end_ts),
        (8, list(payload.deliverables)),
        (12, payload.ts),
    ]
    if payload.milestones is not None:
        entries.append((9, [_milestone_to_cbor(m) for m in payload.milestones]))
    if payload.escrow_id is not None:
        entries.append((10, payload.escrow_id))
    if payload.dispute_policy is not None:
        entries.append((11, payload.dispute_policy))
    return CborMap(entries)


def _milestone_to_cbor(milestone: WorkMilestone) -> CborValue:
    _validate_milestone(milestone)
    entries: List[Tuple[CborValue, CborValue]] = [
        (0, milestone.milestone_id),
        (1, milestone.description),
        (2, milestone.due_ts),
        (3, milestone.amount),
    ]
    if milestone.deliverable_hash is not None:
        if len(milestone.deliverable_hash) != 32:
            raise WorkError("deliverable hash must be 32 bytes")
        entries.append((4, bytes(milestone.deliverable_hash)))
    return CborMap(entries)


def _optional_text_array(entries: List[Tuple[CborValue, CborValue]], key: int) -> Optional[List[str]]:
    value = _get_optional(entries, key)
    if value is None:
        return None
    return _expect_text_array(value)


def _optional_text(entries: List[Tuple[CborValue, CborValue]], key: int) -> Optional[str]:
    value = _get_optional(entries, key)
    if value is None:
        return None
    return _expect_text(value)


def _optional_milestones(entries: List[Tuple[CborValue, CborValue]], key: int) -> Optional[List[WorkMilestone]]:
    value = _get_optional(entries, key)
    if value is None:
        return None
    if not isinstance(value, list) or len(value) == 0:
        raise WorkError("milestones required")
    milestones: List[WorkMilestone] = []
    for item in value:
        milestones.append(_parse_milestone(item))
    return milestones


def _parse_milestone(value: CborValue) -> WorkMilestone:
    entries = _expect_map(value)
    milestone = WorkMilestone(
        milestone_id=_expect_text(_get_required(entries, 0)),
        description=_expect_text(_get_required(entries, 1)),
        due_ts=_expect_u64(_get_required(entries, 2)),
        amount=_expect_u64(_get_required(entries, 3)),
        deliverable_hash=_get_optional_deliverable_hash(entries, 4),
    )
    _validate_milestone(milestone)
    return milestone


def _get_optional_deliverable_hash(entries: List[Tuple[CborValue, CborValue]], key: int) -> Optional[bytes]:
    value = _get_optional(entries, key)
    if value is None:
        return None
    return _expect_bytes_len(value, 32)


def _validate_offer(payload: WorkOfferPayload) -> None:
    _ensure_nonempty(payload.offer_id, "offer id required")
    _ensure_nonempty(payload.issuer, "issuer required")
    _ensure_nonempty(payload.title, "title required")
    _ensure_nonempty(payload.summary, "summary required")
    _ensure_nonempty(payload.scope, "scope required")
    _ensure_positive(payload.budget_amount, "budget amount required")
    _ensure_nonempty(payload.budget_currency, "budget currency required")
    _ensure_positive(payload.duration_sec, "duration required")
    _ensure_list_nonempty(payload.deliverables, "deliverables required")
    if payload.requirements is not None:
        _ensure_list_items(payload.requirements, "requirements required")
    _ensure_positive(payload.ts, "timestamp required")
    if payload.exp <= payload.ts:
        raise WorkError("expiry must be after timestamp")


def _validate_agreement(payload: WorkAgreementPayload) -> None:
    _ensure_nonempty(payload.agreement_id, "agreement id required")
    _ensure_nonempty(payload.offer_id, "offer id required")
    _ensure_nonempty(payload.issuer, "issuer required")
    _ensure_nonempty(payload.counterparty, "counterparty required")
    _ensure_positive(payload.budget_amount, "budget amount required")
    _ensure_nonempty(payload.budget_currency, "budget currency required")
    _ensure_positive(payload.start_ts, "start_ts required")
    _ensure_positive(payload.end_ts, "end_ts required")
    if payload.end_ts <= payload.start_ts:
        raise WorkError("end_ts must be after start_ts")
    _ensure_list_nonempty(payload.deliverables, "deliverables required")
    if payload.milestones is not None:
        if len(payload.milestones) == 0:
            raise WorkError("milestones required")
        for milestone in payload.milestones:
            _validate_milestone(milestone)
    if payload.escrow_id is not None:
        _ensure_nonempty(payload.escrow_id, "escrow id required")
    _ensure_positive(payload.ts, "timestamp required")


def _validate_milestone(milestone: WorkMilestone) -> None:
    _ensure_nonempty(milestone.milestone_id, "milestone id required")
    _ensure_nonempty(milestone.description, "milestone description required")
    _ensure_positive(milestone.due_ts, "milestone due_ts required")
    _ensure_positive(milestone.amount, "milestone amount required")


def _ensure_nonempty(value: str, message: str) -> None:
    if value is None or value.strip() == "":
        raise WorkError(message)


def _ensure_list_nonempty(values: List[str], message: str) -> None:
    if len(values) == 0:
        raise WorkError(message)
    _ensure_list_items(values, message)


def _ensure_list_items(values: List[str], message: str) -> None:
    for item in values:
        if item is None or str(item).strip() == "":
            raise WorkError(message)


def _ensure_positive(value: int, message: str) -> None:
    if value <= 0:
        raise WorkError(message)


def _split_signed_map(value: CborValue, sig_key: int) -> Tuple[CborValue, bytes]:
    entries = _expect_map(value)
    payload_entries: List[Tuple[CborValue, CborValue]] = []
    signature: Optional[bytes] = None
    for key, val in entries:
        if key == sig_key:
            if signature is not None:
                raise WorkError("duplicate signature key")
            if not isinstance(val, (bytes, bytearray)):
                raise WorkError("signature must be bytes")
            signature = bytes(val)
            continue
        payload_entries.append((key, val))
    if signature is None:
        raise WorkError("missing signature")
    if len(signature) != 64:
        raise WorkError("invalid signature length")
    return CborMap(payload_entries), signature


def _with_signature(payload: CborValue, sig_key: int, signature: bytes) -> CborValue:
    entries = list(_expect_map(payload))
    entries.append((sig_key, signature))
    return CborMap(entries)


def _expect_map(value: CborValue) -> List[Tuple[CborValue, CborValue]]:
    if isinstance(value, CborMap):
        return value.entries
    raise WorkError("expected map")


def _get_required(entries: List[Tuple[CborValue, CborValue]], key: int) -> CborValue:
    for k, v in entries:
        if k == key:
            return v
    raise WorkError("missing required key")


def _get_optional(entries: List[Tuple[CborValue, CborValue]], key: int) -> Optional[CborValue]:
    for k, v in entries:
        if k == key:
            return v
    return None


def _expect_text(value: CborValue) -> str:
    if isinstance(value, str):
        return value
    raise WorkError("expected text")


def _expect_text_array(value: CborValue) -> List[str]:
    if isinstance(value, list):
        out: List[str] = []
        for item in value:
            out.append(_expect_text(item))
        return out
    raise WorkError("expected array of text")


def _expect_u64(value: CborValue) -> int:
    if isinstance(value, int) and value >= 0:
        return value
    raise WorkError("expected unsigned")


def _expect_bytes_len(value: CborValue, length: int) -> bytes:
    if isinstance(value, (bytes, bytearray)):
        data = bytes(value)
        if len(data) != length:
            raise WorkError("invalid length")
        return data
    raise WorkError("expected bytes")


def _expect_bytes(value: CborValue) -> bytes:
    if isinstance(value, (bytes, bytearray)):
        return bytes(value)
    raise WorkError("expected bytes")
