# Networking Overview (Research Summary for a New Internet Layer)

This document summarizes proven networking patterns and standards relevant to building a federated, agent-native internet. It is not a spec; it informs architecture decisions.

---

## 1) Transport and reliability

- **QUIC (RFC 9000)** provides low-latency, secure transport with multiplexed streams and connection migration. It is the default transport for AgentMesh where available.
- **TLS 1.3 integration** is part of QUIC's security model and should be required for peer sessions.

Implication for AgentNet:
- Prefer QUIC for peer sessions, with TCP fallback only for constrained environments.
- Make transport negotiation explicit during NodeHello.

---

## 2) Group security for pockets/communities

- **Messaging Layer Security (RFC 9420)** provides scalable, asynchronous group key establishment with forward secrecy and post-compromise security.
- **MLS Architecture (RFC 9750)** defines deployment guidance and security tradeoffs for secure group messaging systems.

Implication for AgentNet:
- MLS is the default for private pockets and org rooms.
- Group membership changes must be receipt-backed and auditable.

---

## 3) Federation and social graph patterns

- **ActivityPub (W3C Recommendation)** defines client-server and server-server federation for social activity routing.
- ActivityPub's separation of client-to-server and server-to-server flows is a strong pattern for an agentic network.

Implication for AgentNet:
- Keep a clear separation between local actions and federation delivery.
- Every federated delivery should be policy-gated and receipt-backed.

---

## 4) Decentralized identity

- **Decentralized Identifiers (DID Core v1.0)** define a method for decentralized identity with controller-managed keys.
- Draft versions (e.g., DID v1.1) should not be used for production until standardized.

Implication for AgentNet:
- Base identity on DID v1.0 or equivalent stable standards.
- Key rotation and revocation must be first-class and receipt-backed.

---

## 5) Architectural takeaways

- Use **federation-first** protocols rather than centralized gateways.
- Separate **data plane** (AgentMesh) from **control plane** (AgentChain).
- Make **policy and receipts** core to every interaction.
- Design for **agent autonomy**, but always inside scope, budget, and proof rules.

