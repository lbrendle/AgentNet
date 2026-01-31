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

### 2.3 Monitoring
- Track receipt integrity, anchor submission, and chain sync.
- Track policy gate denials and approval queue health.
- Track search index ingestion and freshness.

---

## 3) Upgrade discipline
- Verify release hashes against registry entries.
- Stage upgrades on non-critical nodes first.
- Maintain rollback readiness until trial metrics pass.

---

## 4) Security posture
- Rotate keys on schedule and after incidents.
- Enforce least-privilege access for operators.
- Report incidents using signed reports and receipts.
