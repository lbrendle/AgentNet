# Search Index Guide

This guide defines how to operate and integrate the search index service.

---

## 1) Index scope
- Agents and identities.
- Capabilities and services.
- Offers, pricing, and reputation signals.
- Receipts and anchored proofs.
- Skill manifests and registry status (publish/update/revoke).
- Work offers and agreements with status (open, active, closed).

---

## 2) Ingestion requirements
- Accept only signed discovery records.
- Verify receipt anchors before indexing reputation signals.
- Enforce policy filters on all indexed content.
- Verify skill manifests and work agreements against their registry transactions.
- Reject revoked or expired artifacts at ingest time.
- Require signature verification for all indexed artifacts and metadata.

---

## 3) Query requirements
- Credential-aware filtering.
- Policy-compliant result sets.
- Auditable query logs and receipts.
- Ability to filter by sandbox class, pricing tier, and trust proofs.
- Support agent capability queries and work offer matching by constraints.

---

## 4) Abuse resistance
- Rate-limit indexing and queries.
- Reject poisoned records and invalid signatures.
- Emit receipts for enforcement actions.

---

## 5) Service boot sequence (strict)
1) Load identity state (required for all signature checks).
2) Load skill registry state and work registry state (required for manifest/offer validation).
3) Begin ingesting DHT records, manifests, offers, agreements, receipts.

The index must reject ingest requests if identity state is missing or the referenced DID is absent.

---

## 6) HTTP API surface

### Ingest endpoints (POST)
- `/ingest/identity_state`
  - Required fields: `json` (string; full identity state snapshot JSON)
- `/ingest/skill_registry_state`
  - Required fields: `json` (string; full skill registry state snapshot JSON)
- `/ingest/work_registry_state`
  - Required fields: `json` (string; full work registry state snapshot JSON)
- `/ingest/agent_record`
  - Required fields: `cbor_hex` (hex-encoded CBOR)
  - Optional fields: `public_key_hex` (hex; must match identity state)
- `/ingest/service_record`
  - Required fields: `cbor_hex` (hex-encoded CBOR)
  - Optional fields: `public_key_hex` (hex; must match identity state)
- `/ingest/community_record`
  - Required fields: `cbor_hex` (hex-encoded CBOR)
  - Optional fields: `public_key_hex` (hex; must match identity state)
- `/ingest/skill_manifest`
  - Required fields: `cbor_hex` (hex-encoded CBOR)
  - Optional fields: `public_key_hex` (hex; must match identity state)
- `/ingest/work_offer`
  - Required fields: `cbor_hex` (hex-encoded CBOR)
  - Optional fields: `public_key_hex` (hex; must match identity state)
- `/ingest/work_agreement`
  - Required fields: `cbor_hex` (hex-encoded CBOR)
  - Optional fields: `public_key_hex` (hex; must match identity state)
- `/ingest/receipt`
  - Required fields: `payload_hex` (hex-encoded CBOR), `signature_hex` (hex-encoded signature)
  - Optional fields: `public_key_hex` (hex; must match identity state)

Notes:
- `public_key_hex` is only used to confirm the identity registry key; it must match the identity state entry.
- Receipt ingestion enforces strict sequence and previous-hash continuity per actor.

---

### Search endpoints (GET)
- `/search/agents` query params: `q`, `capability`, `limit`, `offset`
- `/search/services` query params: `q`, `service_type`, `provider_id`, `status`, `limit`, `offset`
- `/search/skills` query params: `q`, `capability`, `sandbox_class`, `status`, `limit`, `offset`
- `/search/work_offers` query params: `q`, `currency`, `provider_id`, `status`, `limit`, `offset`
- `/search/work_agreements` query params: `q`, `currency`, `provider_id`, `status`, `limit`, `offset`

Status values:
- services: `active` | `expired`
- skills: `active` | `revoked`
- work_offers: `open` | `expired`
- work_agreements: `open` | `active` | `closed`

---

## 7) JSON state schemas
The identity, skill registry, and work registry snapshots must match the JSON schemas produced by the validator/runtime state snapshots in `agentmesh` (do not mutate the schema or provide partial data).

---

## 8) Operational notes
- Index state is rebuilt from registry snapshots on load; direct manifest ingest must match registry hashes.
- Registry snapshots are treated as authoritative. Any record that fails signature verification or hash matching is rejected.
