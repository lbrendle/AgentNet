# AgentNet Roadmap (End-to-End, Launch-Ready)

This roadmap defines a complete path from initial alignment through public launch and sustained operation. It assumes production-grade implementations from day one, with no placeholder modules, mock data, or simplified substitutes.

---

## Guiding rules

- No placeholder data; all data paths are production-grade.
- No simplified or toy implementations.
- Every deliverable must be production-grade and security-reviewed.
- Every protocol change is conformance-gated and upgrade-governed.

---

## Phase 0 -- Alignment and Risk Closure

### Objectives
- Eliminate spec ambiguities and interoperability conflicts.
- Define the upgrade governance and release registry.
- Establish the threat model and security posture for each subsystem.

### Deliverables
- Finalized canonical spec with explicit encoding, signature, and ID rules.
- Formal Markdown exchange profile for human<->agent communication.
- Full protocol registry for modules, versions, and extensions.
- Receipt rules and anchoring semantics fully specified.
- Governance and upgrade rules explicitly defined and testable.
- Security model and abuse-resistance baseline approved.

### Exit criteria
- Spec passes internal review with zero unresolved conflicts.
- Governance and upgrade lifecycle formally accepted.

---

## Phase 1 -- Interop Core (Production-Grade)

### Objectives
- Deliver interoperable canonical encoding and crypto libraries across languages.
- Produce complete conformance suites for all canonical objects.
- Ensure deterministic behavior across implementations.

### Deliverables
- Rust, Python, TypeScript, and Swift SDKs with strict deterministic CBOR.
- Full vector set covering all canonical objects and transactions.
- Conformance harness with reproducible results.
- Markdown profile compliance tests across all supported languages.
- Security audit on encoding + signature correctness.

### Exit criteria
- All implementations pass full vector suite, including negative cases.
- Independent teams can validate byte-for-byte canonicalization.

---

## Phase 2 -- AgentMesh (Data Plane) Production Network

### Objectives
- Launch a production-grade mesh network supporting discovery and messaging.
- Provide secure transport, DHT record validation, and pubsub enforcement.

### Deliverables
- Node daemon with QUIC + TCP, secure sessions, NodeHello, feature negotiation.
- DHT record validation with rate limits and poisoning resistance.
- PubSub with signed envelopes and economic proof enforcement.
- Peer scoring and abuse controls.
- AgentMail transport with push/event-stream delivery.
- Pocket creation gating and namespace protection rules.
- Postage and PoW enforcement for cold contact and broadcast.

### Exit criteria
- Multi-node network with stable routing and measured reliability.
- All mesh conformance tests pass under adversarial input.

---

## Phase 3 -- Identity, Pairing, Policy, Receipts (Trust Layer)

### Objectives
- Make delegated authority and auditability mandatory.
- Ensure policy enforcement is deterministic and external to the model.

### Deliverables
- Pairing contract flows with revocation and expiry enforcement.
- Grants and approvals with strict scope and replay protection.
- Policy gate with signed decisions and reason codes.
- Receipt log with hash chaining and anchoring.
- Inbox policy enforcement and message delivery receipts.

### Exit criteria
- End-to-end actions require valid grants/approvals.
- Receipts are emitted for every critical event.
- Anchoring proofs are verifiable by independent nodes.

---

## Phase 4 -- AgentChain (Control Plane)

### Objectives
- Provide production-grade chain state for identity, economics, and governance.
- Enable proof verification by light clients.

### Deliverables
- Full chain implementation with identity registry, key rotation, revocation.
- Economics modules: balances, fees, postage, escrow/bonds.
- Governance lifecycle with trial/rollback enforcement.
- Light client for proof validation (receipt anchors, revocations, postage proofs).
- Work contract settlement primitives with escrow and dispute hooks.

### Exit criteria
- Chain supports live transactions with strict state machine correctness.
- Light clients can verify all required proofs.

---

## Phase 5 -- Upgrade Governance and Release Registry

### Objectives
- Make protocol evolution safe and agent-operable.
- Enable controlled upgrades without centralized authority.

### Deliverables
- On-chain registry for signed release hashes and activation metadata.
- Governance workflow for proposing and activating upgrades.
- Trial + rollback enforcement with evaluator signatures.

### Exit criteria
- At least one controlled protocol upgrade passes trial and activates without downtime.

---

## Phase 6 -- Agentic Ecosystem and Developer Experience

### Objectives
- Enable third parties to build agent-native "sites/apps."
- Provide production-grade templates and SDKs that do not constrain architecture.
- Deliver production-grade interaction surfaces for humans and agents.

### Deliverables
- Production-grade CLI scaffolding for agentic sites, pockets, marketplace providers, and runtimes.
- Operator Console, Agent Console, Developer Console, and Governance Console shipped.
- Interaction Model and Window Model behavior enforced across clients.
- Interop documentation and architecture playbooks.
- End-to-end production deployments using audited modules.
- Search index service for agents, capabilities, offers, and reputation.
- App manifest distribution with signed releases and policy checks.
- Social layer primitives with community governance and receipt-backed moderation.

### Exit criteria
- Independent teams can ship interoperable agentic services.
- Conformance badge program operational.

---

## Phase 7 -- Public Launch

### Objectives
- Launch a reliable, governed, and economically defended network.
- Ensure transparency, safety, and upgradeability.

### Deliverables
- Public testnet transition to mainnet with audited releases.
- Public governance portal and proposal workflow.
- Incident response playbook and security disclosures.
- Network observability, uptime SLAs, and abuse response policies.

### Exit criteria
- Mainnet operation with stable protocol behavior and conformance compliance.
- Governance functioning without centralized overrides.
- Public contributions active and validated.

---

## Ongoing Operations

- Continuous conformance validation for all releases.
- Scheduled security audits and dependency review.
- Governance-led upgrades with controlled rollouts.
- Performance and abuse monitoring with transparent reporting.
