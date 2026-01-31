from __future__ import annotations

from dataclasses import dataclass
from typing import List, Optional, Tuple

from .cbor import CborMap, CborValue, decode_canonical, encode_canonical
from .crypto import sha256, verify_ed25519_hash
from .sign import sign_ed25519_hash


class DhtError(ValueError):
    pass


@dataclass
class Contact:
    node_ids: List[str]
    addrs: List[str]


@dataclass
class AgentRecordPayload:
    agent_id: str
    agent_pubkeys: List[bytes]
    contact: Contact
    capabilities: List[str]
    expires: int


@dataclass
class AgentRecord:
    payload: AgentRecordPayload
    signature: bytes


@dataclass
class ServiceRecordPayload:
    provider_id: str
    service_type: int
    addrs: List[str]
    required_credentials: Optional[List[str]]
    pricing: Optional[CborValue]
    expires: int


@dataclass
class ServiceRecord:
    payload: ServiceRecordPayload
    signature: bytes


@dataclass
class CommunityRecordPayload:
    community_id: str
    controller: str
    join_policy: int
    required_credentials: Optional[List[str]]
    economics: CborValue
    governance: CborValue
    expires: int


@dataclass
class CommunityRecord:
    payload: CommunityRecordPayload
    signature: bytes


def parse_contact(value: CborValue) -> Contact:
    entries = _expect_map(value)
    node_ids = _expect_text_array(_get_required(entries, 0))
    addrs = _expect_text_array(_get_required(entries, 1))
    return Contact(node_ids=node_ids, addrs=addrs)


def parse_agent_record_payload(value: CborValue) -> AgentRecordPayload:
    entries = _expect_map(value)
    agent_id = _expect_text(_get_required(entries, 0))
    agent_pubkeys = _expect_bytes_array_len(_get_required(entries, 1), 32)
    contact = parse_contact(_get_required(entries, 2))
    capabilities = _expect_text_array(_get_required(entries, 3))
    expires = _expect_u64(_get_required(entries, 4))
    return AgentRecordPayload(
        agent_id=agent_id,
        agent_pubkeys=agent_pubkeys,
        contact=contact,
        capabilities=capabilities,
        expires=expires,
    )


def parse_agent_record(value: CborValue) -> AgentRecord:
    payload, signature = _split_signed_map(value, 5)
    payload_obj = parse_agent_record_payload(payload)
    return AgentRecord(payload=payload_obj, signature=signature)


def parse_service_record_payload(value: CborValue) -> ServiceRecordPayload:
    entries = _expect_map(value)
    provider_id = _expect_text(_get_required(entries, 0))
    service_type = _expect_u16(_get_required(entries, 1))
    addrs = _expect_text_array(_get_required(entries, 2))
    required_credentials = None
    if _get_optional(entries, 3) is not None:
        required_credentials = _expect_text_array(_get_optional(entries, 3))
    pricing = None
    if _get_optional(entries, 4) is not None:
        pricing = _get_optional(entries, 4)
    expires = _expect_u64(_get_required(entries, 5))
    return ServiceRecordPayload(
        provider_id=provider_id,
        service_type=service_type,
        addrs=addrs,
        required_credentials=required_credentials,
        pricing=pricing,
        expires=expires,
    )


def parse_service_record(value: CborValue) -> ServiceRecord:
    payload, signature = _split_signed_map(value, 6)
    payload_obj = parse_service_record_payload(payload)
    return ServiceRecord(payload=payload_obj, signature=signature)


def parse_community_record_payload(value: CborValue) -> CommunityRecordPayload:
    entries = _expect_map(value)
    community_id = _expect_text(_get_required(entries, 0))
    controller = _expect_text(_get_required(entries, 1))
    join_policy = _expect_u8(_get_required(entries, 2))
    required_credentials = None
    if _get_optional(entries, 3) is not None:
        required_credentials = _expect_text_array(_get_optional(entries, 3))
    economics = _get_required(entries, 4)
    governance = _get_required(entries, 5)
    expires = _expect_u64(_get_required(entries, 6))
    return CommunityRecordPayload(
        community_id=community_id,
        controller=controller,
        join_policy=join_policy,
        required_credentials=required_credentials,
        economics=economics,
        governance=governance,
        expires=expires,
    )


def parse_community_record(value: CborValue) -> CommunityRecord:
    payload, signature = _split_signed_map(value, 7)
    payload_obj = parse_community_record_payload(payload)
    return CommunityRecord(payload=payload_obj, signature=signature)


def build_agent_record(payload: AgentRecordPayload, secret_key: bytes) -> bytes:
    return _build_signed_record(_agent_record_payload_to_cbor(payload), 5, secret_key)


def build_service_record(payload: ServiceRecordPayload, secret_key: bytes) -> bytes:
    return _build_signed_record(_service_record_payload_to_cbor(payload), 6, secret_key)


def build_community_record(payload: CommunityRecordPayload, secret_key: bytes) -> bytes:
    return _build_signed_record(_community_record_payload_to_cbor(payload), 7, secret_key)


def verify_agent_record(data: bytes, public_key: bytes) -> AgentRecordPayload:
    return _verify_signed_record(data, 5, public_key, parse_agent_record_payload)


def verify_service_record(data: bytes, public_key: bytes) -> ServiceRecordPayload:
    return _verify_signed_record(data, 6, public_key, parse_service_record_payload)


def verify_community_record(data: bytes, public_key: bytes) -> CommunityRecordPayload:
    return _verify_signed_record(data, 7, public_key, parse_community_record_payload)


def _agent_record_payload_to_cbor(payload: AgentRecordPayload) -> CborValue:
    return CborMap(
        [
            (0, payload.agent_id),
            (1, [bytes(key) for key in payload.agent_pubkeys]),
            (2, _contact_to_cbor(payload.contact)),
            (3, list(payload.capabilities)),
            (4, payload.expires),
        ]
    )


def _service_record_payload_to_cbor(payload: ServiceRecordPayload) -> CborValue:
    entries: List[Tuple[CborValue, CborValue]] = [
        (0, payload.provider_id),
        (1, payload.service_type),
        (2, list(payload.addrs)),
    ]
    if payload.required_credentials is not None:
        entries.append((3, list(payload.required_credentials)))
    if payload.pricing is not None:
        entries.append((4, payload.pricing))
    entries.append((5, payload.expires))
    return CborMap(entries)


def _community_record_payload_to_cbor(payload: CommunityRecordPayload) -> CborValue:
    entries: List[Tuple[CborValue, CborValue]] = [
        (0, payload.community_id),
        (1, payload.controller),
        (2, payload.join_policy),
    ]
    if payload.required_credentials is not None:
        entries.append((3, list(payload.required_credentials)))
    entries.append((4, payload.economics))
    entries.append((5, payload.governance))
    entries.append((6, payload.expires))
    return CborMap(entries)


def _contact_to_cbor(contact: Contact) -> CborValue:
    return CborMap(
        [
            (0, list(contact.node_ids)),
            (1, list(contact.addrs)),
        ]
    )


def _build_signed_record(payload: CborValue, sig_key: int, secret_key: bytes) -> bytes:
    payload_cbor = encode_canonical(payload)
    digest = sha256(payload_cbor)
    signature = sign_ed25519_hash(secret_key, digest)
    signed = _with_signature(payload, sig_key, signature)
    return encode_canonical(signed)


def _verify_signed_record(data: bytes, sig_key: int, public_key: bytes, parser) -> CborValue:
    value = decode_canonical(data)
    payload, signature = _split_signed_map(value, sig_key)
    payload_cbor = encode_canonical(payload)
    digest = sha256(payload_cbor)
    verify_ed25519_hash(public_key, digest, signature)
    return parser(payload)


def _split_signed_map(value: CborValue, sig_key: int) -> Tuple[CborValue, bytes]:
    entries = _expect_map(value)
    payload_entries: List[Tuple[CborValue, CborValue]] = []
    signature: Optional[bytes] = None
    for key, val in entries:
        if isinstance(key, int) and key == sig_key:
            if signature is not None:
                raise DhtError("duplicate signature key")
            if not isinstance(val, (bytes, bytearray)):
                raise DhtError("signature must be bytes")
            signature = bytes(val)
            continue
        payload_entries.append((key, val))
    if signature is None:
        raise DhtError("missing signature")
    if len(signature) != 64:
        raise DhtError("invalid signature length")
    return CborMap(payload_entries), signature


def _with_signature(payload: CborValue, sig_key: int, signature: bytes) -> CborValue:
    entries = list(_expect_map(payload))
    entries.append((sig_key, signature))
    return CborMap(entries)


def _expect_map(value: CborValue) -> List[Tuple[CborValue, CborValue]]:
    if isinstance(value, CborMap):
        return value.entries
    raise DhtError("expected map")


def _get_required(entries: List[Tuple[CborValue, CborValue]], key: int) -> CborValue:
    for k, v in entries:
        if isinstance(k, int) and k == key:
            return v
    raise DhtError("missing required key")


def _get_optional(entries: List[Tuple[CborValue, CborValue]], key: int) -> Optional[CborValue]:
    for k, v in entries:
        if isinstance(k, int) and k == key:
            return v
    return None


def _expect_text(value: CborValue) -> str:
    if isinstance(value, str):
        return value
    raise DhtError("expected text")


def _expect_text_array(value: CborValue) -> List[str]:
    if isinstance(value, list):
        out: List[str] = []
        for item in value:
            out.append(_expect_text(item))
        return out
    raise DhtError("expected text array")


def _expect_bytes_array_len(value: CborValue, size: int) -> List[bytes]:
    if isinstance(value, list):
        out: List[bytes] = []
        for item in value:
            out.append(_expect_bytes_len(item, size))
        return out
    raise DhtError("expected bytes array")


def _expect_bytes_len(value: CborValue, size: int) -> bytes:
    if isinstance(value, (bytes, bytearray)):
        data = bytes(value)
        if len(data) != size:
            raise DhtError("invalid length")
        return data
    raise DhtError("expected bytes")


def _expect_u64(value: CborValue) -> int:
    if isinstance(value, int) and value >= 0:
        return value
    raise DhtError("expected unsigned")


def _expect_u16(value: CborValue) -> int:
    if isinstance(value, int) and 0 <= value <= 0xFFFF:
        return value
    raise DhtError("expected u16")


def _expect_u8(value: CborValue) -> int:
    if isinstance(value, int) and 0 <= value <= 0xFF:
        return value
    raise DhtError("expected u8")
