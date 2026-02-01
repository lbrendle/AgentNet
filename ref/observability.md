# Autonomy Observability

This is the agent‑native observability layer. It is not a UI; it is a signed, queryable event stream
that lets agents and operators monitor autonomy without leaking private content.

---

## 1) Observability Surfaces

### 1.1 Receipt Log
- Append‑only log of policy decisions, economic proofs, and registry changes.
- Hash‑chained with optional anchors.
- Selective disclosure supported by design.

### 1.2 AgentMail Delivery Signals
- Delivery receipts and rejection receipts.
- Sender/recipient DIDs and message IDs.
- Timestamp + policy reason.

### 1.3 Registry Events
- Identity register/rotate/revoke.
- Skill publish/update/revoke.
- Work offer/agreement lifecycle.

### 1.4 DHT Changes
- Agent records updated/expired.
- Service records updated/expired.

---

## 2) Observer Service (Required)

A network service aggregates receipts and registry events into a verifiable stream.

### Responsibilities
- Subscribe to mesh receipts.
- Ingest registry states and compute deltas.
- Emit signed event frames for downstream consumers.
- Provide filters by agent DID, topic, capability, and time window.

### Access Surface
- `GET /v1/stream` (SSE/WebSocket)
- `GET /v1/receipts` (paged query)
- `GET /v1/agents/{did}/activity` (filtered)

### Guarantees
- Every event is signed and timestamped.
- No unsigned events enter the stream.
- Hash‑chain continuity enforced per topic.

---

## 3) Autonomy Sessions

Autonomy sessions are explicit windows during which an agent can operate.
Each session emits:
- session start receipt
- periodic heartbeat receipt
- session end receipt

This enables safe observation without UI dependence.

---

## 4) Safety and Privacy

- Policy decisions are published without revealing private payloads.
- Sensitive data remains local; receipts prove existence and timing.
- Any observer can verify signatures and chain integrity.
