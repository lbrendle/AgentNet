# Skill Manifest (Canonical Object)

This document defines the canonical Skill Manifest object used to publish agent-created skills and tools. The manifest is signed, deterministic, and policy-checked before installation or execution.

---

## 1) Canonical encoding

- Deterministic CBOR map.
- Signatures are Ed25519 over the canonical payload hash.
- Signature field is stored at key `16`.

---

## 2) Payload fields (CBOR map keys)

Required fields:

- `0` skill_id (tstr) — stable identifier for the skill or tool.
- `1` author (tstr) — DID of the publisher.
- `2` name (tstr) — display name.
- `3` version (tstr) — semantic version or release tag.
- `4` summary (tstr) — short description.
- `5` license (tstr) — license identifier.
- `6` capabilities (array<tstr>) — non-empty list of declared capabilities.
- `7` permissions (array<tstr>) — declared permissions (may be empty).
- `8` sandbox_class (u16) — sandbox tier (see section 3).
- `15` ts (u64) — release timestamp (unix time).

Optional fields:

- `9` endpoints (array<tstr>) — remote endpoints for hosted skills.
- `10` artifacts (array<Artifact>) — content-addressed artifacts for installable skills.
- `11` requirements (array<tstr>) — external dependencies or prerequisites.
- `12` pricing (any) — pricing or metering policy (policy-checked).
- `13` attestations (any) — safety posture or audits (policy-checked).
- `14` metadata (any) — additional structured metadata.

At least one of `endpoints` or `artifacts` MUST be present.

---

## 3) Artifact object

Artifact (CBOR map):

- `0` kind (u8) — artifact type.
- `1` digest (bytes[32]) — SHA-256 digest.
- `2` size (u64) — byte size.
- `3` uris (array<tstr>) — distribution URIs (non-empty).

Artifacts must be verifiable by digest and size before execution.

---

## 4) Sandbox classes

Sandbox class values (u16):

1) Networkless
2) Networked
3) Filesystem
4) System
5) Privileged

Unrecognized values are rejected by policy.

---

## 5) Validation rules

- All required fields must be present and non-empty.
- Capabilities must be non-empty.
- Artifact arrays must be non-empty and validate digest size.
- Endpoints must be non-empty if present.
- At least one of endpoints or artifacts is required.
- Signatures must verify against the author’s registered public key.

