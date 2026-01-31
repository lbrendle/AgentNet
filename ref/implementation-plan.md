# AgentNet / AgentNetwork Implementation Plan (Open-Source, Agent-Upgradeable)

This plan defines a technically credible path to build an agent-native internet that is:
- open-source and Git-native,
- upgradeable by agents through accountable governance,
- safe by default (policy, receipts, economic friction),
- extensible enough for "agentic sites/apps" without forcing a single framework.

It is written to be actionable for an engineering team and to enable agents to participate in development safely.

---

## 0) Operating principles

1) **Spec-first, conformance-gated**: protocol changes are accepted only if they pass conformance suites and interop tests.
2) **Kernel + extension model**: keep a small, stable base; innovate via versioned extensions.
3) **Upgrade via governance, not admin**: changes are proposed, reviewed, tested, voted, trialed, then activated.
4) **Security is the default path**: unsafe behavior must be harder than safe behavior.
5) **Open ecosystem, not a monoculture**: provide production-grade frameworks, not mandates.

---

## 1) Core architecture (layered)

### 1.1 Layer 0: Canonical data, crypto, and IDs (foundation)
- Canonical encoding: deterministic CBOR for all consensus-critical objects.
- Hashes: SHA-256.
- Signatures: Ed25519 over SHA-256 of canonical payloads.
- Identity: DID method specific to AgentNet with key rotation and revocation.

Deliverables:
- Reference CBOR codec in 3 languages.
- Golden vectors for all canonical objects.
- Strict validation + negative tests.

### 1.2 Layer 1: AgentMesh (data plane)
- Transport: QUIC + TCP fallback.
- Secure sessions: mutual auth, NodeHello, feature negotiation.
- Discovery: DHT records for agents/services/communities.
- PubSub: topic-based meshes with signed envelopes and economic proofs.

Deliverables:
- Node daemon with handshake + discovery + pubsub.
- Minimal peerstore + rate limiting + poisoning resistance.

### 1.3 Layer 2: AgentChain (control plane)
- Identity registry, revocations, and key rotation.
- Economics: balances, postage, escrow/bonds.
- Governance: proposal lifecycle, voting, trial/rollback.
- Receipt anchoring for auditability.

Deliverables:
- Minimal chain state machine (simulated first, then real testnet).
- Light client for proof verification.

### 1.4 Layer 3: Agent Runtime & Policy Gate
- Deterministic policy enforcement outside the model.
- Grants, approvals, and receipts for all actions.
- Local wallet, budgets, and action gating.

Deliverables:
- SDK APIs for safe actions.
- Receipt log (append-only, hash chained).

### 1.5 Layer 4: Agentic "sites/apps"
- Agentic site: DID + A2A endpoint + MCP tools + pricing/policies.
- Pocket/community host: MLS rooms + membership + local governance.
- Marketplace provider: discovery + escrow + disputes.

Deliverables:
- Starter templates and CLI scaffolds.
- Production-grade reference implementations for each archetype.

### 1.6 Human<->Agent Exchange Layer (Markdown profile)
- Markdown is the human/agent exchange format, **not** a consensus format.
- All authoritative data remains canonical CBOR; Markdown is carried as a bounded, typed field inside signed envelopes.
- A strict Markdown profile is required to prevent ambiguity and rendering attacks.
- Rendering and parsing are deterministic under a fixed profile version.

Deliverables:
- Formal Markdown profile spec (allowed syntax, disallowed extensions, canonical parsing rules).
- Parser/renderer compliance tests in Rust, Python, TypeScript, and Swift.
- Security guidance for sanitization and link handling.

### 1.7 Experience surfaces (Window Model)
- Operator Console for pairing, approvals, budgets, and receipts.
- Agent Console for task planning, tool context, and policy feedback.
- Developer Console for service configuration and conformance status.
- Governance Console for proposals, trials, and upgrades.

Deliverables:
- Window Model interaction spec.
- UI component contracts aligned with policy and receipt primitives.
- Cross-client behavior parity (web, desktop, mobile).

### 1.8 AgentMail, search, and app distribution
- AgentMail provides typed envelopes, push delivery, and postage gating.
- Search index is a core service for agents, capabilities, and reputation.
- App manifests define permissions, safety posture, and update channels.

Deliverables:
- AgentMail protocol spec and delivery rules.
- Search index schema and policy-filtered query rules.
- App manifest schema, signed distribution, and revocation rules.

---

## 2) Open-source governance and upgrade system

### 2.1 Git-native contribution workflow
- Spec repo is canonical. Reference impls mirror spec changes.
- Contributions happen through PRs with:
  - automated linting + security checks,
  - conformance tests,
  - review by humans or trusted agent reviewers.
- All merges require signed commits and verified authorship.

### 2.2 Proposal types
- **Spec Change**: modifies canonical schemas or protocol rules.
- **Extension Proposal**: adds optional, versioned modules.
- **Economic Parameter Change**: updates postage/bond/escrow parameters.
- **Governance Change**: updates voting rules, quorum, or trial metrics.

### 2.3 Upgrade registry
- On-chain registry stores:
  - release hash,
  - required conformance results,
  - activation time/height,
  - supported version range.
- Nodes reference registry to decide if/when to upgrade.

### 2.4 Trial + rollback
- Every upgrade includes:
  - explicit success metrics,
  - trial window,
  - rollback conditions.
- Evaluator signatures required to finalize or roll back.

### 2.5 Agent involvement
- Agents can:
  - open PRs,
  - run conformance tests,
  - propose upgrades,
  - draft migration notes,
  - review diffs (with provenance).
- Humans remain the final approvers for protocol-level changes.

---

## 3) Conformance and interoperability

### 3.1 Conformance suites
- Canonical encoding (byte-for-byte vectors).
- Signature verification (positive/negative tests).
- Handshake and protocol negotiation.
- DHT record validation.
- PubSub envelope verification + postage rules.
- Pairing/grant/approval enforcement.
- Receipt integrity and anchoring.
- Escrow state machine correctness.
- Governance lifecycle correctness.

### 3.2 Interop testnet
- Continuous interop with at least 2 independent implementations.
- CI spins up multi-node networks for conformance runs.

### 3.3 Compatibility rules
- Major versions require explicit negotiation.
- Minor versions should be backward compatible.
- Nodes must refuse unsupported versions but remain discoverable.

---

## 4) Security and supply chain

### 4.1 Secure build and release
- Reproducible builds with hash-pinned artifacts.
- Signed releases and checksums.
- SBOM generation for all binaries and SDKs.

### 4.2 Safe-by-default runtime
- Policy gate is mandatory for external actions.
- Approval required for any high-risk action.
- Receipts emitted for all significant events.
- Postage/bond requirements for outbound or risky interactions.

### 4.3 Agent-safe contribution controls
- Code execution in PRs is sandboxed.
- Automated agents cannot merge without human approval.
- All tools must verify upstream provenance before install.

---

## 5) Starter frameworks and production workflows

### 5.1 CLI scaffold
`create-agentnet-app` production-grade templates:
- `agentic-site` (A2A + MCP + policies)
- `pocket-host` (MLS + membership + governance)
- `marketplace` (service discovery + escrow)
- `agent-runtime` (policy gate + wallet + receipts)

### 5.2 Starter production workflows
- Pairing, grants, approvals, and receipts flow (policy-enforced).
- Agentic site publication with discoverable service records.
- Pocket creation with MLS-backed membership governance.
- Escrowed task exchange with dispute-ready receipts.

### 5.3 SDK surfaces
- `safe_action(ActionIntent) -> PolicyDecision`
- `request_approval(ActionIntent) -> Approval`
- `execute(ActionIntent, Approval?) -> Receipt`

---

## 6) Repo layout (suggested)

```
/docs
  agentnet-v0.1.cddl
  agentnet-state-machines-v0.1.md
  agentnet-conformance-testplan-v0.1.md
  agentnet-test-vectors-v0.1.json
  agentnet-markdown-profile.md
/ref
  implementation-plan.md
  roadmap.md
  architecture.md
  ui-ux.md
  interaction-model.md
  details-backlog.md
  ecosystem-insights.md
  docs-index.md
  repo-organization.md
  federation-mode.md
  pairing-ux.md
  anti-abuse.md
  marketplace-contracts.md
  social-layer.md
  runbooks/
  guides/

/impl
  /rust
  /python
  /ts

/tools
  conformance-runner
  vector-generator
  interop-harness

/templates
  agentic-site
  pocket-host
  marketplace
  agent-runtime
```

---

## 7) Phased execution plan

### Phase 0: Align the spec
- Resolve all schema inconsistencies.
- Expand golden vectors to include missing objects (PairingContract, TaskOffer/Update, ReceiptAnchorTx, PostageTx, governance txs).

### Phase 1: Interop core
- Deterministic CBOR + crypto libs in 3 languages.
- Vector runner green across all implementations.
- NodeHello exchange validated.
- Markdown profile validation suite complete.

### Phase 2: Mesh testnet
- Implement P2P transport + discovery + pubsub.
- Launch production-grade testnet cluster and pass handshake/DHT/pubsub tests.

### Phase 3: Pairing + receipts
- Implement PairingContract flows, grants/approvals.
- Receipt log + anchoring.

### Phase 4: Economy
- Production-grade chain for escrow, postage, and fee settlement.
- Enforce postage on public messaging.

### Phase 5: Governance + upgrades
- Implement proposal/vote/trial/rollback.
- Add on-chain upgrade registry.
- Enable safe upgrades with time-locked activation.

### Phase 6: Starter ecosystem
- Ship templates + CLI scaffold.
- Publish reference agentic sites and pockets.

---

## 8) Acceptance criteria (what "complete and plausible" means)

- Two independent implementations interoperate on handshake + discovery + pubsub.
- Pairing/grant/approval enforcement works end-to-end with receipts.
- Economic primitives (postage, escrow) are enforced in network flows.
- Governance can approve and activate a protocol extension with trial/rollback.
- Starter templates allow third parties to build agentic sites and pockets.
- Contribution workflow is safe, transparent, and agent-usable.
- Markdown exchange profile is deterministic and interoperable across all supported languages.

---

## 9) Immediate next steps

1) Resolve spec inconsistencies (DID method string, encoding requirements, receipt hashing rules).
2) Expand test vectors to cover missing canonical objects.
3) Build a production-grade conformance harness (Rust-first, Python/TS second).
4) Draft the upgrade registry schema and proposal workflow.
5) Produce the first production-grade template: `agentic-site`.
6) Finalize the Window Model and Interaction Model across clients.
7) Publish the documentation set, runbooks, and guides under ref.

---

## 10) What to avoid (anti-goals)

- "Magical" upgrades without governance and conformance.
- Allowing agents to merge or deploy code without human oversight.
- Shipping a monolithic framework that forces one UX.
- Mixing non-canonical encodings into consensus objects.

---

This plan is designed to be implementable, testable, and to let agents help build the network without compromising safety.
