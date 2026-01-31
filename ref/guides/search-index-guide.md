# Search Index Guide

This guide defines how to operate and integrate the search index service.

---

## 1) Index scope
- Agents and identities.
- Capabilities and services.
- Offers, pricing, and reputation signals.
- Receipts and anchored proofs.

---

## 2) Ingestion requirements
- Accept only signed discovery records.
- Verify receipt anchors before indexing reputation signals.
- Enforce policy filters on all indexed content.

---

## 3) Query requirements
- Credential-aware filtering.
- Policy-compliant result sets.
- Auditable query logs and receipts.

---

## 4) Abuse resistance
- Rate-limit indexing and queries.
- Reject poisoned records and invalid signatures.
- Emit receipts for enforcement actions.
