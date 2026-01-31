from __future__ import annotations

from dataclasses import dataclass
from typing import List, Optional, Tuple

from .cbor import CborMap, CborValue, decode_canonical, encode_canonical
from .crypto import sha256, verify_ed25519_hash
from .sign import sign_ed25519_hash

SKILL_SIG_KEY = 16
SANDBOX_MIN = 1
SANDBOX_MAX = 5


class SkillError(ValueError):
    pass


@dataclass
class SkillArtifact:
    kind: int
    digest: bytes
    size: int
    uris: List[str]


@dataclass
class SkillManifestPayload:
    skill_id: str
    author: str
    name: str
    version: str
    summary: str
    license: str
    capabilities: List[str]
    permissions: List[str]
    sandbox_class: int
    endpoints: Optional[List[str]]
    artifacts: Optional[List[SkillArtifact]]
    requirements: Optional[List[str]]
    pricing: Optional[CborValue]
    attestations: Optional[CborValue]
    metadata: Optional[CborValue]
    ts: int


@dataclass
class SkillManifest:
    payload: SkillManifestPayload
    signature: bytes


@dataclass
class SkillPublishPayload:
    manifest: bytes
    ts: int


@dataclass
class SkillUpdatePayload:
    skill_id: str
    prev_manifest_hash: bytes
    manifest: bytes
    ts: int


@dataclass
class SkillRevokePayload:
    skill_id: str
    manifest_hash: bytes
    reason: str
    ts: int


def parse_skill_manifest_payload(value: CborValue) -> SkillManifestPayload:
    entries = _expect_map(value)
    payload = SkillManifestPayload(
        skill_id=_expect_text(_get_required(entries, 0)),
        author=_expect_text(_get_required(entries, 1)),
        name=_expect_text(_get_required(entries, 2)),
        version=_expect_text(_get_required(entries, 3)),
        summary=_expect_text(_get_required(entries, 4)),
        license=_expect_text(_get_required(entries, 5)),
        capabilities=_expect_text_array(_get_required(entries, 6)),
        permissions=_expect_text_array(_get_required(entries, 7)),
        sandbox_class=_expect_u16(_get_required(entries, 8)),
        endpoints=_optional_text_array(entries, 9),
        artifacts=_optional_artifacts(entries, 10),
        requirements=_optional_text_array(entries, 11),
        pricing=_get_optional(entries, 12),
        attestations=_get_optional(entries, 13),
        metadata=_get_optional(entries, 14),
        ts=_expect_u64(_get_required(entries, 15)),
    )
    _validate_payload(payload)
    return payload


def parse_skill_manifest(value: CborValue) -> SkillManifest:
    payload, signature = _split_signed_map(value, SKILL_SIG_KEY)
    payload_obj = parse_skill_manifest_payload(payload)
    return SkillManifest(payload=payload_obj, signature=signature)


def decode_skill_manifest(data: bytes) -> SkillManifest:
    return parse_skill_manifest(decode_canonical(data))


def build_skill_manifest(payload: SkillManifestPayload, secret_key: bytes) -> bytes:
    _validate_payload(payload)
    payload_cbor = _skill_payload_to_cbor(payload)
    payload_bytes = encode_canonical(payload_cbor)
    digest = sha256(payload_bytes)
    signature = sign_ed25519_hash(secret_key, digest)
    full = _with_signature(payload_cbor, SKILL_SIG_KEY, signature)
    return encode_canonical(full)


def verify_skill_manifest(data: bytes, public_key: bytes) -> SkillManifestPayload:
    value = decode_canonical(data)
    payload, signature = _split_signed_map(value, SKILL_SIG_KEY)
    payload_bytes = encode_canonical(payload)
    digest = sha256(payload_bytes)
    verify_ed25519_hash(public_key, digest, signature)
    return parse_skill_manifest_payload(payload)


def parse_skill_publish_payload(value: CborValue) -> SkillPublishPayload:
    entries = _expect_map(value)
    manifest = _expect_bytes(_get_required(entries, 0))
    ts = _expect_u64(_get_required(entries, 1))
    if ts == 0:
        raise SkillError("timestamp required")
    decode_skill_manifest(manifest)
    return SkillPublishPayload(manifest=manifest, ts=ts)


def parse_skill_update_payload(value: CborValue) -> SkillUpdatePayload:
    entries = _expect_map(value)
    skill_id = _expect_text(_get_required(entries, 0))
    prev_manifest_hash = _expect_bytes_len(_get_required(entries, 1), 32)
    manifest = _expect_bytes(_get_required(entries, 2))
    ts = _expect_u64(_get_required(entries, 3))
    if ts == 0:
        raise SkillError("timestamp required")
    decode_skill_manifest(manifest)
    return SkillUpdatePayload(
        skill_id=skill_id,
        prev_manifest_hash=prev_manifest_hash,
        manifest=manifest,
        ts=ts,
    )


def parse_skill_revoke_payload(value: CborValue) -> SkillRevokePayload:
    entries = _expect_map(value)
    skill_id = _expect_text(_get_required(entries, 0))
    manifest_hash = _expect_bytes_len(_get_required(entries, 1), 32)
    reason = _expect_text(_get_required(entries, 2))
    ts = _expect_u64(_get_required(entries, 3))
    if ts == 0:
        raise SkillError("timestamp required")
    if reason.strip() == "":
        raise SkillError("reason required")
    return SkillRevokePayload(
        skill_id=skill_id,
        manifest_hash=manifest_hash,
        reason=reason,
        ts=ts,
    )


def skill_publish_payload_to_cbor(payload: SkillPublishPayload) -> CborValue:
    if payload.ts == 0:
        raise SkillError("timestamp required")
    decode_skill_manifest(payload.manifest)
    return CborMap([(0, bytes(payload.manifest)), (1, payload.ts)])


def skill_update_payload_to_cbor(payload: SkillUpdatePayload) -> CborValue:
    if payload.ts == 0:
        raise SkillError("timestamp required")
    _ensure_nonempty(payload.skill_id, "skill id required")
    if len(payload.prev_manifest_hash) != 32:
        raise SkillError("invalid manifest hash length")
    decode_skill_manifest(payload.manifest)
    return CborMap(
        [
            (0, payload.skill_id),
            (1, bytes(payload.prev_manifest_hash)),
            (2, bytes(payload.manifest)),
            (3, payload.ts),
        ]
    )


def skill_revoke_payload_to_cbor(payload: SkillRevokePayload) -> CborValue:
    if payload.ts == 0:
        raise SkillError("timestamp required")
    _ensure_nonempty(payload.skill_id, "skill id required")
    _ensure_nonempty(payload.reason, "reason required")
    if len(payload.manifest_hash) != 32:
        raise SkillError("invalid manifest hash length")
    return CborMap(
        [
            (0, payload.skill_id),
            (1, bytes(payload.manifest_hash)),
            (2, payload.reason),
            (3, payload.ts),
        ]
    )


def _skill_payload_to_cbor(payload: SkillManifestPayload) -> CborValue:
    _validate_payload(payload)
    entries: List[Tuple[CborValue, CborValue]] = [
        (0, payload.skill_id),
        (1, payload.author),
        (2, payload.name),
        (3, payload.version),
        (4, payload.summary),
        (5, payload.license),
        (6, list(payload.capabilities)),
        (7, list(payload.permissions)),
        (8, payload.sandbox_class),
        (15, payload.ts),
    ]
    if payload.endpoints is not None:
        entries.append((9, list(payload.endpoints)))
    if payload.artifacts is not None:
        entries.append((10, [_artifact_to_cbor(a) for a in payload.artifacts]))
    if payload.requirements is not None:
        entries.append((11, list(payload.requirements)))
    if payload.pricing is not None:
        entries.append((12, payload.pricing))
    if payload.attestations is not None:
        entries.append((13, payload.attestations))
    if payload.metadata is not None:
        entries.append((14, payload.metadata))
    return CborMap(entries)


def _artifact_to_cbor(artifact: SkillArtifact) -> CborValue:
    _validate_artifact(artifact)
    return CborMap(
        [
            (0, artifact.kind),
            (1, bytes(artifact.digest)),
            (2, artifact.size),
            (3, list(artifact.uris)),
        ]
    )


def _optional_text_array(entries: List[Tuple[CborValue, CborValue]], key: int) -> Optional[List[str]]:
    value = _get_optional(entries, key)
    if value is None:
        return None
    return _expect_text_array(value)


def _optional_artifacts(entries: List[Tuple[CborValue, CborValue]], key: int) -> Optional[List[SkillArtifact]]:
    value = _get_optional(entries, key)
    if value is None:
        return None
    if not isinstance(value, list):
        raise SkillError("expected artifact array")
    if len(value) == 0:
        raise SkillError("artifacts required")
    artifacts: List[SkillArtifact] = []
    for item in value:
        artifacts.append(_parse_artifact(item))
    return artifacts


def _parse_artifact(value: CborValue) -> SkillArtifact:
    entries = _expect_map(value)
    artifact = SkillArtifact(
        kind=_expect_u8(_get_required(entries, 0)),
        digest=_expect_bytes_len(_get_required(entries, 1), 32),
        size=_expect_u64(_get_required(entries, 2)),
        uris=_expect_text_array(_get_required(entries, 3)),
    )
    _validate_artifact(artifact)
    return artifact


def _validate_payload(payload: SkillManifestPayload) -> None:
    _ensure_nonempty(payload.skill_id, "skill id required")
    _ensure_nonempty(payload.author, "author required")
    _ensure_nonempty(payload.name, "name required")
    _ensure_nonempty(payload.version, "version required")
    _ensure_nonempty(payload.summary, "summary required")
    _ensure_nonempty(payload.license, "license required")
    _ensure_list_nonempty(payload.capabilities, "capabilities required")
    _ensure_list_items(payload.permissions, "permissions required")
    if payload.sandbox_class < SANDBOX_MIN or payload.sandbox_class > SANDBOX_MAX:
        raise SkillError("invalid sandbox class")
    if payload.endpoints is not None:
        _ensure_list_nonempty(payload.endpoints, "endpoints required")
    if payload.artifacts is not None:
        if len(payload.artifacts) == 0:
            raise SkillError("artifacts required")
        for artifact in payload.artifacts:
            _validate_artifact(artifact)
    if payload.requirements is not None:
        _ensure_list_items(payload.requirements, "requirements required")
    if payload.endpoints is None and payload.artifacts is None:
        raise SkillError("skill requires endpoints or artifacts")


def _validate_artifact(artifact: SkillArtifact) -> None:
    if artifact.kind <= 0:
        raise SkillError("artifact kind required")
    if len(artifact.digest) != 32:
        raise SkillError("artifact digest must be 32 bytes")
    if artifact.size <= 0:
        raise SkillError("artifact size required")
    _ensure_list_nonempty(artifact.uris, "artifact uris required")


def _ensure_nonempty(value: str, message: str) -> None:
    if value is None or value.strip() == "":
        raise SkillError(message)


def _ensure_list_nonempty(values: List[str], message: str) -> None:
    if len(values) == 0:
        raise SkillError(message)
    _ensure_list_items(values, message)


def _ensure_list_items(values: List[str], message: str) -> None:
    for item in values:
        if item is None or str(item).strip() == "":
            raise SkillError(message)


def _split_signed_map(value: CborValue, sig_key: int) -> Tuple[CborValue, bytes]:
    entries = _expect_map(value)
    payload_entries: List[Tuple[CborValue, CborValue]] = []
    signature: Optional[bytes] = None
    for key, val in entries:
        if key == sig_key:
            if signature is not None:
                raise SkillError("duplicate signature key")
            if not isinstance(val, (bytes, bytearray)):
                raise SkillError("signature must be bytes")
            signature = bytes(val)
            continue
        payload_entries.append((key, val))
    if signature is None:
        raise SkillError("missing signature")
    if len(signature) != 64:
        raise SkillError("invalid signature length")
    return CborMap(payload_entries), signature


def _with_signature(payload: CborValue, sig_key: int, signature: bytes) -> CborValue:
    entries = list(_expect_map(payload))
    entries.append((sig_key, signature))
    return CborMap(entries)


def _expect_map(value: CborValue) -> List[Tuple[CborValue, CborValue]]:
    if isinstance(value, CborMap):
        return value.entries
    raise SkillError("expected map")


def _get_required(entries: List[Tuple[CborValue, CborValue]], key: int) -> CborValue:
    for k, v in entries:
        if k == key:
            return v
    raise SkillError("missing required key")


def _get_optional(entries: List[Tuple[CborValue, CborValue]], key: int) -> Optional[CborValue]:
    for k, v in entries:
        if k == key:
            return v
    return None


def _expect_text(value: CborValue) -> str:
    if isinstance(value, str):
        return value
    raise SkillError("expected text")


def _expect_text_array(value: CborValue) -> List[str]:
    if isinstance(value, list):
        out: List[str] = []
        for item in value:
            out.append(_expect_text(item))
        return out
    raise SkillError("expected array of text")


def _expect_bytes_len(value: CborValue, length: int) -> bytes:
    if isinstance(value, (bytes, bytearray)):
        data = bytes(value)
        if len(data) != length:
            raise SkillError("invalid length")
        return data
    raise SkillError("expected bytes")


def _expect_bytes(value: CborValue) -> bytes:
    if isinstance(value, (bytes, bytearray)):
        return bytes(value)
    raise SkillError("expected bytes")


def _expect_u64(value: CborValue) -> int:
    if isinstance(value, int) and value >= 0:
        return value
    raise SkillError("expected unsigned")


def _expect_u16(value: CborValue) -> int:
    if isinstance(value, int) and 0 <= value <= 0xFFFF:
        return value
    raise SkillError("expected u16")


def _expect_u8(value: CborValue) -> int:
    if isinstance(value, int) and 0 <= value <= 0xFF:
        return value
    raise SkillError("expected u8")
