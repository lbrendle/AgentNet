from __future__ import annotations

from dataclasses import dataclass

from .cbor import CborMap, CborValue


class IdentityError(ValueError):
    pass


@dataclass
class IdentityRegisterPayload:
    agent_id: str
    pk_ed25519: bytes
    pk_x25519: bytes
    created: int


@dataclass
class IdentityRotatePayload:
    agent_id: str
    pk_ed25519: bytes
    pk_x25519: bytes
    ts: int


@dataclass
class CredentialRevokePayload:
    issuer: str
    credential_id_hash: bytes
    ts: int


def parse_identity_register_payload(value: CborValue) -> IdentityRegisterPayload:
    entries = _expect_map(value)
    pk_ed25519 = _expect_bytes(_get_required(entries, 1))
    pk_x25519 = _expect_bytes(_get_required(entries, 2))
    if len(pk_ed25519) != 32 or len(pk_x25519) != 32:
        raise IdentityError("invalid public key length")
    return IdentityRegisterPayload(
        agent_id=_expect_text(_get_required(entries, 0)),
        pk_ed25519=pk_ed25519,
        pk_x25519=pk_x25519,
        created=_expect_u64(_get_required(entries, 3)),
    )


def parse_identity_rotate_payload(value: CborValue) -> IdentityRotatePayload:
    entries = _expect_map(value)
    pk_ed25519 = _expect_bytes(_get_required(entries, 1))
    pk_x25519 = _expect_bytes(_get_required(entries, 2))
    if len(pk_ed25519) != 32 or len(pk_x25519) != 32:
        raise IdentityError("invalid public key length")
    return IdentityRotatePayload(
        agent_id=_expect_text(_get_required(entries, 0)),
        pk_ed25519=pk_ed25519,
        pk_x25519=pk_x25519,
        ts=_expect_u64(_get_required(entries, 3)),
    )


def parse_credential_revoke_payload(value: CborValue) -> CredentialRevokePayload:
    entries = _expect_map(value)
    cred_hash = _expect_bytes(_get_required(entries, 1))
    if len(cred_hash) != 32:
        raise IdentityError("credential_id_hash must be 32 bytes")
    return CredentialRevokePayload(
        issuer=_expect_text(_get_required(entries, 0)),
        credential_id_hash=cred_hash,
        ts=_expect_u64(_get_required(entries, 2)),
    )


def identity_register_payload_to_cbor(payload: IdentityRegisterPayload) -> CborValue:
    return CborMap([
        (0, payload.agent_id),
        (1, payload.pk_ed25519),
        (2, payload.pk_x25519),
        (3, payload.created),
    ])


def identity_rotate_payload_to_cbor(payload: IdentityRotatePayload) -> CborValue:
    return CborMap([
        (0, payload.agent_id),
        (1, payload.pk_ed25519),
        (2, payload.pk_x25519),
        (3, payload.ts),
    ])


def credential_revoke_payload_to_cbor(payload: CredentialRevokePayload) -> CborValue:
    return CborMap([
        (0, payload.issuer),
        (1, payload.credential_id_hash),
        (2, payload.ts),
    ])


def _expect_map(value: CborValue):
    if isinstance(value, CborMap):
        return value.entries
    raise IdentityError("expected map")


def _get_required(entries, key: int) -> CborValue:
    for k, v in entries:
        if isinstance(k, int) and k == key:
            return v
    raise IdentityError("missing required key")


def _expect_text(value: CborValue) -> str:
    if isinstance(value, str):
        return value
    raise IdentityError("expected text")


def _expect_bytes(value: CborValue) -> bytes:
    if isinstance(value, (bytes, bytearray)):
        return bytes(value)
    raise IdentityError("expected bytes")


def _expect_u64(value: CborValue) -> int:
    if isinstance(value, int) and value >= 0:
        return value
    raise IdentityError("expected unsigned")
