# End-to-End Task List (Three AI Agents)

This task list divides all work across three AI agents. Each task is production-grade, security-reviewed, and explicitly avoids placeholder or simplified implementations. Dependencies are noted where necessary.

---

## Agent 1 -- Protocol, Conformance, and SDK Integrity

### Phase 0: Spec Alignment
- Resolve canonical encoding, signature, and DID rules across all documents.
- Formalize receipt hashing and anchoring semantics.
- Define upgrade proposal schema and registry rules.
- Define AgentMail inbox policy schema and enforcement rules.
- Define app manifest schema and distribution rules.
- Define search index schema and query policy rules.
- Define Skill Manifest schema and sandbox classes.
- Define Roam Grant and Work Contract schemas.

### Phase 1: Conformance Suites
- Produce full canonical vector set for every protocol object and transaction type.
- Implement conformance harness with deterministic results.
- Build negative tests for malformed encodings and signature failures.
- Build Markdown profile compliance tests (parser/renderer equivalence).

### Phase 2: SDK Foundations (Rust, Python, TypeScript, Swift)
- Implement deterministic CBOR encoding in all SDKs.
- Implement signature and hash validation consistently across languages.
- Provide strict schema validation for all canonical objects.
- Implement Markdown profile parsing and rendering with deterministic output.
- Implement Skill Manifest validation and signing helpers.
- Implement Work Contract and Roam Grant validation helpers.

### Phase 3: Conformance Badge Program
- Create signed conformance attestation format.
- Integrate badge verification into discovery and service records.

### Phase 4: Release Integrity
- Define reproducible build requirements and SBOM generation.
- Integrate signature verification for releases and upgrades.

Dependencies
- Must complete spec alignment before SDK and vector work.

---

## Agent 2 -- AgentMesh, Runtime, and Policy Enforcement

### Phase 1: AgentMesh Core
- Implement QUIC/TCP transport with secure session establishment.
- Implement NodeHello negotiation and protocol capability checks.
- Implement DHT record storage with signature validation and expiry enforcement.
- Implement PubSub with signed envelopes and postage proof enforcement.
- Implement AgentMail delivery with push/event streams.
- Enforce pocket creation gating and namespace protection.
- Implement PoW verification for rate tiers and cold contact where required.

### Phase 2: Runtime + Policy Gate
- Implement deterministic policy gate with signed decisions.
- Implement grant and approval enforcement with replay protection.
- Integrate receipts for all significant actions.
- Implement inbox rules and message delivery receipts.
- Implement anti-abuse policy escalation and downgrade paths.
- Implement social layer gating to prevent execution bypass.
- Implement Roam Grants with destination verification and receipt trails.
- Implement skill/tool install gates with sandbox enforcement.
- Implement AgentMail CLI bridge (tail + send) for non-UI agent runtime integration.
- Implement Agent Interface Gateway (HTTP/gRPC) with signed, receipt-backed actions.

### Phase 3: Receipt Log and Anchoring
- Implement append-only receipt log with hash chaining.
- Implement anchoring submission and verification.
- Implement Observer service to stream signed receipts and registry deltas.

### Phase 4: Agentic Service Frameworks
- Implement agentic site runtime layer (A2A + MCP + policy + receipts).
- Implement pocket/community host with MLS encryption and governance hooks.
- Implement Operator Console and Agent Console aligned with Window Model.
- Implement approval queue, receipt ledger, and policy visibility surfaces.
- Implement Agent Forge runtime for skill creation, signing, and publication.
- Implement Work Contract UX for escrow and deliverables.
- Implement App/Experience manifest deployment pipeline and revocation hooks.

Dependencies
- SDK validation from Agent 1 required before runtime integration.

---

## Agent 3 -- AgentChain, Economics, Governance, and Launch Ops

### Phase 1: AgentChain Core
- Implement identity registry, key rotation, and revocation.
- Implement account balances and fee management.
- Implement postage enforcement and escrow/bond contracts.
- Implement app manifest registry hooks and revocation support.
- Implement work contract settlement with escrow and dispute hooks.
- Implement skill/tool registry with versioned manifests and revocations.

### Phase 2: Governance and Upgrades
- Implement proposal lifecycle, voting, and trial/rollback logic.
- Implement upgrade registry with signed releases and activation rules.
- Implement RepoOps policy and signed PR pipeline for protocol evolution.

### Phase 3: Light Client and Proof Verification
- Implement light client for receipt anchors and economic proofs.
- Ensure proofs are verifiable by runtime and mesh relays.
- Implement search index ingestion of receipt anchors and reputation signals.

### Phase 4: Launch Operations
- Build monitoring, incident response playbooks, and disclosure process.
- Set up governance portal and public proposal workflow.
- Establish public testnet and mainnet activation procedures.
- Implement Governance Console for proposal, trial, and upgrade control.
- Operate search index service with policy filtering and abuse controls.
- Operate public gateway program with conformance and identity disclosure.

Dependencies
- Conformance suites and canonical object rules required before chain integration.

---

## Cross-Agent Coordination Tasks

- Shared threat model and security review checklist.
- Interop testnet with multiple implementations.
- Consistent versioning and module registry updates.
- Public documentation and developer onboarding guides.
- Window Model consistency across all clients and SDKs.

---

## End-to-End Launch Criteria

- Two independent implementations pass full conformance suites.
- Mesh and chain interoperate with policy enforcement enabled.
- Governance can approve and activate an upgrade safely.
- Receipt anchoring works end-to-end with light client verification.
- Production-grade templates produce interoperable, policy-compliant services.

---

## Explicit Constraints

- No placeholder code or logic; no non-production data in any deliverable.
- All releases are signed, reproducible, and security-reviewed.
- Protocol changes require conformance proofs and governance approval.
