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
