# Mainnet Readiness Gates

This document defines the non-negotiable gates for a public, federated mainnet launch.

---

## 1) Protocol and conformance

- Canonical encoding vectors pass across at least two independent implementations.
- All negative tests for signatures, hashes, and schema validation pass.
- Markdown exchange profile is deterministic and interoperable.
- NodeHello negotiation rejects incompatible versions.

---

## 2) Security and abuse resistance

- Policy gate blocks high-risk actions by default.
- Postage or PoW enforced for cold contact and broadcast.
- Pocket creation is gated and rate-limited.
- Security audit completed for transport, receipt log, and policy gate.

---

## 3) Identity and proofs

- DID resolution, key rotation, and revocation enforced by policy.
- Economic proofs (voucher and on-chain) are verifiable by independent nodes.
- Receipt anchoring proofs are verifiable by light clients.

---

## 4) Federation and access

- Multiple independent operators run seed nodes.
- Gateways are optional and enforce policy/receipts.
- Hosted nodes can be migrated away from without identity loss.

---

## 5) Governance and upgrade safety

- Governance portal operational with proposal lifecycle.
- Upgrade registry in place with trial/rollback rules.
- At least one upgrade completes trial on testnet before mainnet.

---

## 6) UX and interaction safety

- Window Model enforced across Operator and Agent consoles.
- Approvals and receipts are visible within one interaction step.
- Kill switch is single-operator and hardware-backed.

---

## 7) Operations and reliability

- Incident response, key compromise, and abuse runbooks tested.
- Observability metrics and alerting in place.
- Uptime and performance thresholds met under load tests.
- Search index service operational with registry-snapshot ingestion and enforced rate limits.
