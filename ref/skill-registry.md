# Skill Registry (Transactions and State)

This document defines the registry operations for publishing, updating, and revoking agent-created skills. These operations are enforced by policy and emit receipts.

---

## 1) Transaction types

- `30` SkillPublish
- `31` SkillUpdate
- `32` SkillRevoke

Tx payloads are canonical CBOR maps carried inside `TxEnvelope.payload`.

---

## 2) SkillPublish payload

CBOR map:

- `0` manifest (bytes) — canonical, signed Skill Manifest bytes.
- `1` ts (u64) — publish timestamp.

Rules:
- Manifest must verify with the sender’s registered public key.
- Manifest `author` MUST equal `TxEnvelope.sender`.
- Skill ID must not already exist.
- Timestamp must be within the configured clock skew window.

---

## 3) SkillUpdate payload

CBOR map:

- `0` skill_id (tstr)
- `1` prev_manifest_hash (bytes[32])
- `2` manifest (bytes)
- `3` ts (u64)

Rules:
- Skill must exist and not be revoked.
- Sender MUST match the registered author.
- `prev_manifest_hash` must match current registry record.
- Manifest must verify with sender key and keep the same `skill_id`.
- Timestamp must be within the configured clock skew window.

---

## 4) SkillRevoke payload

CBOR map:

- `0` skill_id (tstr)
- `1` manifest_hash (bytes[32])
- `2` reason (tstr)
- `3` ts (u64)

Rules:
- Skill must exist and not be revoked.
- Sender MUST match the registered author.
- `manifest_hash` must match current registry record.
- Timestamp must be within the configured clock skew window.

---

## 5) Receipt details

Receipts emit event type `EV_SKILL_REGISTRY` with details including:
- event string (`skill.publish`, `skill.update`, `skill.revoke`)
- skill_id
- author
- manifest hash(es)
- reason (for revokes)

Receipts are anchored and auditable.
