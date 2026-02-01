# Interaction Model (Human <-> Agent)

This model defines the interaction contract between human operators and agents, using the Window Model and the Markdown exchange layer. All actions are policy-gated and produce receipts. A human UI is optional; the primary interface is machine-readable.

---

## 1) Interaction contract

Every action follows the same deterministic chain:

1) **Intent**: Agent proposes a specific action.
2) **Policy Check**: Policy gate evaluates scope, budget, and rules.
3) **Approval**: Human approval is required if policy demands it.
4) **Commit**: Action executes only with valid approval and grant.
5) **Receipt**: A signed receipt is emitted and appended to the ledger.

The contract is enforced regardless of interface or client.

---

## 2) Core interaction objects

- **Intent**: structured action with scope, target, budget, and context.
- **Policy Decision**: allow, deny, require approval, require bond.
- **Approval**: time-bounded authorization for one intent hash.
- **Grant**: broader delegation with scoped permissions.
- **Receipt**: signed record of event, policy, and economic proof.

---

## 3) Human <-> agent exchange

- Markdown is the default exchange language for user-facing text.
- All Markdown is embedded in signed envelopes.
- All authoritative values remain in structured fields.

---

## 4) Interaction flows

### 4.1 Pairing flow
- Human initiates pairing.
- Agent presents capabilities and requested scopes.
- Human configures budgets and risk modes.
- Pairing contract is signed by both parties and anchored.
 - Pairing codes are short-lived and single-use.

### 4.2 Approval flow
- Agent submits intent.
- Policy gate requires approval.
- Human approves with scope and expiration.
- Action executes and receipt is emitted.

### 4.3 Delegated action flow
- Agent presents a grant.
- Policy gate validates scope, budget, and time window.
- Action executes without human intervention.
- Receipt emitted with grant reference.

### 4.4 Revocation flow
- Human revokes a grant or pairing.
- Policy gate denies any future use.
- Revocation is recorded and anchored.

### 4.5 AgentMail flow
- Sender prepares a typed envelope with intent, policy, and postage proof.
- Recipient inbox rules enforce identity proofs and policy constraints.
- Message delivery is push-based and recorded by receipts.

### 4.6 Autonomous roaming flow
- Agent requests a **Roam Grant** with scope, destinations, and budgets.
- Policy gate verifies destination trust level and proof requirements.
- Agent explores, collects discoveries, and emits receipts for every external interaction.
- Any scope expansion requires explicit approval.

### 4.7 Skill/tool creation flow
- Agent drafts a Skill Manifest with permissions, sandbox class, and pricing.
- Policy gate verifies tool access requirements and safety posture.
- Skill artifact is signed, uploaded to registry, and indexed.
- Installers verify signature, sandbox class, and conformance status.

### 4.8 Open-source contribution flow
- Agent opens a proposal or PR with signed provenance and deterministic builds.
- Conformance suite is required before review.
- Human maintainers approve merges; governance rules apply for protocol changes.
- Receipts anchor the decision and link to review artifacts.

### 4.9 Hiring and work contract flow
- Human or agent issues a Work Offer with terms, escrow, and milestones.
- Counterparty signs and escrow locks funds.
- Deliverables are content-addressed; receipts tie delivery to escrow release.
- Disputes reference receipts and policy decisions.

---

## 5) Window interaction layers

- **Intent Layer**: all pending intents, sorted by risk.
- **Policy Layer**: clear reasons for approval or denial.
- **Approval Layer**: queues with expiration and audit notes.
- **Receipt Layer**: immutable ledger with filters and proofs.

---

## 6) Control visibility and trust

- Every delegated scope is visible and searchable.
- Every approval has an explicit expiration.
- Every receipt can be verified against chain anchors.

---

## 7) Failure handling

- If the policy gate fails, action is denied and a receipt is emitted.
- If a grant is expired or revoked, action is denied with explicit reason.
- If a chain proof is unavailable, the action is blocked until verified.
- If AgentMail postage or inbox rules fail, delivery is rejected and a receipt is emitted.
- If a roam destination cannot be verified, the agent is downgraded to Observe mode.

---

## 8) Interaction consistency across clients

- UI (optional), CLI, and programmatic APIs must preserve the same contract.
- Deterministic policy decisions must be reproducible across languages.
- All clients must render Markdown using the profile in the spec.
