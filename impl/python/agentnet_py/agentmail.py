from __future__ import annotations

from dataclasses import dataclass
from typing import List, Optional, Tuple

from .cbor import CborMap, CborValue, decode_canonical, encode_canonical
from .crypto import sha256, verify_ed25519_hash
from .sign import sign_ed25519_hash
from .markdown import validate_markdown_profile

AGENTMAIL_SIG_KEY = 14
AGENTMAIL_VERSION = 1


class AgentMailError(ValueError):
    pass


@dataclass
class AgentMailAttachment:
    content_hash: bytes
    size_bytes: int
    mime: str
    retrieval: Optional[List[str]]


@dataclass
class AgentMailMessagePayload:
    version: int
    message_id: str
    sender: str
    recipients: List[str]
    thread_id: Optional[str]
    reply_to: Optional[str]
    subject: Optional[str]
    markdown: str
    attachments: Optional[List[AgentMailAttachment]]
    intent_hashes: Optional[List[bytes]]
    receipt_hashes: Optional[List[bytes]]
    metadata: Optional[CborValue]
    ts: int
    expires: Optional[int]


@dataclass
class AgentMailMessage:
    payload: AgentMailMessagePayload
    signature: bytes


def parse_agentmail_payload(value: CborValue) -> AgentMailMessagePayload:
    entries = _expect_map(value)
    payload = AgentMailMessagePayload(
        version=_expect_u8(_get_required(entries, 0)),
        message_id=_expect_text(_get_required(entries, 1)),
        sender=_expect_text(_get_required(entries, 2)),
        recipients=_expect_text_array(_get_required(entries, 3)),
        thread_id=_optional_text(entries, 4),
        reply_to=_optional_text(entries, 5),
        subject=_optional_text(entries, 6),
        markdown=_expect_text(_get_required(entries, 7)),
        attachments=_optional_attachments(entries, 8),
        intent_hashes=_optional_hashes(entries, 9),
        receipt_hashes=_optional_hashes(entries, 10),
        metadata=_get_optional(entries, 11),
        ts=_expect_u64(_get_required(entries, 12)),
        expires=_optional_u64(entries, 13),
    )
    _validate_payload(payload)
    return payload


def parse_agentmail_message(value: CborValue) -> AgentMailMessage:
    payload, signature = _split_signed_map(value, AGENTMAIL_SIG_KEY)
    payload_obj = parse_agentmail_payload(payload)
    return AgentMailMessage(payload=payload_obj, signature=signature)


def decode_agentmail_message(data: bytes) -> AgentMailMessage:
    return parse_agentmail_message(decode_canonical(data))


def build_agentmail_message(payload: AgentMailMessagePayload, secret_key: bytes) -> bytes:
    _validate_payload(payload)
    payload_cbor = _agentmail_payload_to_cbor(payload)
    payload_bytes = encode_canonical(payload_cbor)
    digest = sha256(payload_bytes)
    signature = sign_ed25519_hash(secret_key, digest)
    full = _with_signature(payload_cbor, AGENTMAIL_SIG_KEY, signature)
    return encode_canonical(full)


def verify_agentmail_message(data: bytes, public_key: bytes) -> AgentMailMessagePayload:
    value = decode_canonical(data)
    payload, signature = _split_signed_map(value, AGENTMAIL_SIG_KEY)
    payload_bytes = encode_canonical(payload)
    digest = sha256(payload_bytes)
    verify_ed25519_hash(public_key, digest, signature)
    return parse_agentmail_payload(payload)


def _validate_payload(payload: AgentMailMessagePayload) -> None:
    if payload.version != AGENTMAIL_VERSION:
        raise AgentMailError("unsupported agentmail version")
    _ensure_nonempty(payload.message_id, "message_id required")
    _ensure_nonempty(payload.sender, "sender required")
    if not payload.recipients:
        raise AgentMailError("recipients required")
    for recipient in payload.recipients:
        _ensure_nonempty(recipient, "recipient required")
    if payload.thread_id is not None:
        _ensure_nonempty(payload.thread_id, "thread_id required")
    if payload.reply_to is not None:
        _ensure_nonempty(payload.reply_to, "reply_to required")
    if payload.subject is not None:
        _ensure_nonempty(payload.subject, "subject required")
    _ensure_nonempty(payload.markdown, "markdown required")
    validate_markdown_profile(payload.markdown)
    if payload.ts == 0:
        raise AgentMailError("timestamp required")
    if payload.expires is not None and payload.expires < payload.ts:
        raise AgentMailError("expires before timestamp")
    if payload.attachments is not None:
        if not payload.attachments:
            raise AgentMailError("attachments empty")
        for attachment in payload.attachments:
            _validate_attachment(attachment)
    if payload.intent_hashes is not None:
        if not payload.intent_hashes:
            raise AgentMailError("intent hashes empty")
        for digest in payload.intent_hashes:
            if len(digest) != 32:
                raise AgentMailError("intent hash length invalid")
    if payload.receipt_hashes is not None:
        if not payload.receipt_hashes:
            raise AgentMailError("receipt hashes empty")
        for digest in payload.receipt_hashes:
            if len(digest) != 32:
                raise AgentMailError("receipt hash length invalid")


def _validate_attachment(attachment: AgentMailAttachment) -> None:
    if len(attachment.content_hash) != 32:
        raise AgentMailError("attachment hash length invalid")
    if attachment.size_bytes <= 0:
        raise AgentMailError("attachment size required")
    _ensure_nonempty(attachment.mime, "attachment mime required")
    if attachment.retrieval is not None:
        if not attachment.retrieval:
            raise AgentMailError("attachment retrieval empty")
        for addr in attachment.retrieval:
            _ensure_nonempty(addr, "attachment retrieval invalid")


def _agentmail_payload_to_cbor(payload: AgentMailMessagePayload) -> CborValue:
    _validate_payload(payload)
    entries: List[Tuple[CborValue, CborValue]] = []
    entries.append((0, payload.version))
    entries.append((1, payload.message_id))
    entries.append((2, payload.sender))
    entries.append((3, list(payload.recipients)))
    if payload.thread_id is not None:
        entries.append((4, payload.thread_id))
    if payload.reply_to is not None:
        entries.append((5, payload.reply_to))
    if payload.subject is not None:
        entries.append((6, payload.subject))
    entries.append((7, payload.markdown))
    if payload.attachments is not None:
        entries.append((8, [_attachment_to_cbor(att) for att in payload.attachments]))
    if payload.intent_hashes is not None:
        entries.append((9, [bytes(digest) for digest in payload.intent_hashes]))
    if payload.receipt_hashes is not None:
        entries.append((10, [bytes(digest) for digest in payload.receipt_hashes]))
    if payload.metadata is not None:
        entries.append((11, payload.metadata))
    entries.append((12, payload.ts))
    if payload.expires is not None:
        entries.append((13, payload.expires))
    return CborMap(entries)


def _attachment_to_cbor(attachment: AgentMailAttachment) -> CborValue:
    _validate_attachment(attachment)
    entries: List[Tuple[CborValue, CborValue]] = []
    entries.append((0, bytes(attachment.content_hash)))
    entries.append((1, int(attachment.size_bytes)))
    entries.append((2, attachment.mime))
    if attachment.retrieval is not None:
        entries.append((3, list(attachment.retrieval)))
    return CborMap(entries)


def _optional_text(entries: List[Tuple[CborValue, CborValue]], key: int) -> Optional[str]:
    value = _get_optional(entries, key)
    if value is None:
        return None
    return _expect_text(value)


def _optional_u64(entries: List[Tuple[CborValue, CborValue]], key: int) -> Optional[int]:
    value = _get_optional(entries, key)
    if value is None:
        return None
    return _expect_u64(value)


def _optional_attachments(entries: List[Tuple[CborValue, CborValue]], key: int) -> Optional[List[AgentMailAttachment]]:
    value = _get_optional(entries, key)
    if value is None:
        return None
    if not isinstance(value, list):
        raise AgentMailError("expected attachment array")
    attachments: List[AgentMailAttachment] = []
    for item in value:
        attachments.append(_parse_attachment(item))
    if not attachments:
        raise AgentMailError("attachments empty")
    return attachments


def _parse_attachment(value: CborValue) -> AgentMailAttachment:
    entries = _expect_map(value)
    content_hash = _expect_bytes_len(_get_required(entries, 0), 32)
    size_bytes = _expect_u64(_get_required(entries, 1))
    mime = _expect_text(_get_required(entries, 2))
    retrieval = _optional_text_array(entries, 3)
    attachment = AgentMailAttachment(
        content_hash=content_hash,
        size_bytes=size_bytes,
        mime=mime,
        retrieval=retrieval,
    )
    _validate_attachment(attachment)
    return attachment


def _optional_hashes(entries: List[Tuple[CborValue, CborValue]], key: int) -> Optional[List[bytes]]:
    value = _get_optional(entries, key)
    if value is None:
        return None
    if not isinstance(value, list):
        raise AgentMailError("expected hash array")
    hashes: List[bytes] = []
    for item in value:
        hashes.append(_expect_bytes_len(item, 32))
    if not hashes:
        raise AgentMailError("hash array empty")
    return hashes


def _optional_text_array(entries: List[Tuple[CborValue, CborValue]], key: int) -> Optional[List[str]]:
    value = _get_optional(entries, key)
    if value is None:
        return None
    return _expect_text_array(value)


def _expect_map(value: CborValue) -> List[Tuple[CborValue, CborValue]]:
    if isinstance(value, CborMap):
        return list(value.entries)
    raise AgentMailError("expected cbor map")


def _get_required(entries: List[Tuple[CborValue, CborValue]], key: int) -> CborValue:
    for entry_key, entry_value in entries:
        if _is_uint(entry_key) and entry_key == key:
            return entry_value
    raise AgentMailError("missing required key")


def _get_optional(entries: List[Tuple[CborValue, CborValue]], key: int) -> Optional[CborValue]:
    for entry_key, entry_value in entries:
        if _is_uint(entry_key) and entry_key == key:
            return entry_value
    return None


def _expect_u8(value: CborValue) -> int:
    if _is_uint(value) and 0 <= value <= 255:
        return int(value)
    raise AgentMailError("expected u8")


def _expect_u64(value: CborValue) -> int:
    if _is_uint(value):
        return int(value)
    raise AgentMailError("expected u64")


def _expect_text(value: CborValue) -> str:
    if isinstance(value, str):
        return value
    raise AgentMailError("expected text")


def _expect_text_array(value: CborValue) -> List[str]:
    if not isinstance(value, list):
        raise AgentMailError("expected array of text")
    out: List[str] = []
    for item in value:
        out.append(_expect_text(item))
    return out


def _expect_bytes(value: CborValue) -> bytes:
    if isinstance(value, (bytes, bytearray)):
        return bytes(value)
    raise AgentMailError("expected bytes")


def _expect_bytes_len(value: CborValue, length: int) -> bytes:
    data = _expect_bytes(value)
    if len(data) != length:
        raise AgentMailError("invalid length")
    return data


def _split_signed_map(value: CborValue, signature_key: int) -> Tuple[CborMap, bytes]:
    entries = _expect_map(value)
    payload_entries: List[Tuple[CborValue, CborValue]] = []
    signature: Optional[bytes] = None
    for key, val in entries:
        if _is_uint(key) and key == signature_key:
            signature = _expect_bytes(val)
        else:
            payload_entries.append((key, val))
    if signature is None:
        raise AgentMailError("signature missing")
    return CborMap(payload_entries), signature


def _with_signature(payload: CborValue, signature_key: int, signature: bytes) -> CborMap:
    entries = _expect_map(payload)
    entries.append((signature_key, bytes(signature)))
    return CborMap(entries)


def _ensure_nonempty(value: str, message: str) -> None:
    if value.strip() == "":
        raise AgentMailError(message)


def _is_uint(value: CborValue) -> bool:
    return isinstance(value, int) and value >= 0 and not isinstance(value, bool)
