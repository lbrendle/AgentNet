# Operator Guide

This guide defines how to operate AgentNet nodes and services in production.

---

## 1) Core responsibilities
- Maintain node availability and network participation.
- Enforce policy gates and postage rules.
- Verify receipt anchors and chain proofs.
- Apply upgrades only after registry verification.

---

## 2) Node lifecycle

### 2.1 Provisioning
- Use hardened hosts with secure key storage.
- Generate node keys using approved cryptographic modules.
- Register node identity and capabilities.

### 2.2 Runtime enforcement
- Verify NodeHello negotiation on all inbound connections.
- Reject invalid DHT records and invalid signatures.
- Enforce pubsub postage rules for cold contact.
- Configure transports explicitly (QUIC for UDP-capable hosts; WebSocket for HTTP-only ingress).
- Configure an economic proof validator for postage and escrow receipts.
- Run the economic proof verifier as an isolated service with strict failure handling.
- Keep economic proof validation fail-closed unless explicitly approved.
- Verify transaction envelopes against registered sender keys.
- Persist escrow state and event logs on durable storage.
- Enable per-sender rate limits with explicit window and quota settings.
- Operate identity registry storage with rotation and revocation enabled only by policy.
- Enforce budget caps per sender and currency with persisted budget windows.
- Operate skill registry storage with publish/update/revoke policy enforced.
- Operate work registry storage with offer/agreement publish/update/close policy enforced.

### 2.3 Monitoring
- Track receipt integrity, anchor submission, and chain sync.
- Track policy gate denials and approval queue health.
- Track search index ingestion and freshness.
- Ensure receipt logging is enabled and stored on durable media.

---

## 3) Upgrade discipline
- Verify release hashes against registry entries.
- Stage upgrades on non-critical nodes first.
- Maintain rollback readiness until trial metrics pass.

---

## 4) Security posture
- Rotate keys on schedule and after incidents.
- Enforce least-privilege access for operators.
- Restrict kill switch access to a single operator with hardware-backed credentials.
- Keep kill switch release as a manual, local operator action.
- Report incidents using signed reports and receipts.
