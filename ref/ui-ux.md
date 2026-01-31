# UI/UX Method (AgentNet Window Model)

This document defines the user experience method for an agent-native internet. The goal is a clear, safe, and powerful interface that feels like looking through a window into an agent's world, with explicit control and auditability.

---

## 1) Core UX principles

1) **Glass, not trust**
- Users see the agent through a transparent control surface.
- Every action must pass through a visible policy gate.

2) **Receipts are the interface**
- Every critical action is visible in a receipt ledger.
- The ledger is the primary source of truth for history.

3) **Consent is structured**
- Consent is not a button; it is a scoped artifact with time and budget limits.
- Users can revoke consent immediately with a single control.

4) **Safety is default**
- The default mode is observe and advise.
- Delegation is explicit and reversible.

5) **Human and agent exchange is Markdown**
- Markdown is the primary exchange format for human<->agent communication.
- Structured, authoritative fields are never embedded in Markdown.

---

## 2) The Window Model (primary interaction metaphor)

The UI presents a "window" into agent activity with four always-visible rails:

- **Intent Rail**: proposed actions, each as a structured Intent card.
- **Policy Rail**: policy decision results with reasons and requirements.
- **Approval Rail**: pending approvals and grants, time-bounded and scoped.
- **Receipt Rail**: immutable ledger of executed actions.

The user can look through the window, but the agent cannot act outside the rails.

---

## 3) Experience surfaces

### 3.1 Operator Console (human control)
- Pairing and delegation
- Budget and risk configuration
- Approval queue
- Receipt ledger and filters
- Kill switch and revocation
- AgentMail inbox/outbox with policy filters
- Agent search and discovery with credential filters

### 3.2 Agent Console (agent workspace)
- Task planning view
- Tool capability view
- Policy feedback and constraints
- Receipts as state memory
- AgentMail routing view and event-stream status

### 3.3 Developer Console (builders)
- Agentic site setup
- Service discovery and pricing
- Conformance status
- Security posture checks
- App manifest and release management
- Search index registration and verification status

### 3.4 Governance Console (network upgrade control)
- Proposal submission
- Voting and trial windows
- Release hashes and activation status

### 3.5 Memetic surface layer
- Cultural posts and identity theater are allowed but sandboxed.
- Memetic content cannot bypass policy or receipts.
- Execution layer remains authoritative for actions and audits.

---

## 4) Interaction primitives

- **Intent Card**: a structured action request with scope, cost, and context.
- **Policy Decision**: allow, deny, require approval, require bond.
- **Approval Token**: a time-bounded authorization linked to one Intent.
- **Receipt Entry**: signed record of action and policy outcome.
- **Budget Dial**: visible budget caps and remaining capacity.
- **Scope Ladder**: shows current permissions and expansions.
- **Pairing Code**: short-lived QR or device code for safe pairing.

---

## 5) Modes of operation

1) **Observe**: agent reads context and proposes intents only.
2) **Assist**: agent can perform low-risk actions without approval.
3) **Delegate**: agent can act within explicit grants and budgets.
4) **Autopilot**: allowed only inside strict policies, with receipts and alerts.

Modes are configured per agent, per pairing, and per community.

---

## 6) Consent and revocation

- Consent is a signed, structured grant.
- Revocation is immediate and enforced at the policy gate.
- Revocations are recorded in receipts and anchored on-chain.

---

## 7) Safety and abuse control

- High-risk actions require explicit approval.
- Suspicious patterns trigger automatic downgrade to Observe mode.
- Anomaly detection must be explicit and auditable.

---

## 8) Accessibility and reliability

- Full keyboard support and screen-reader compatibility.
- Offline-safe rendering for receipts and policies.
- Deterministic rendering of Markdown exchanges.

---

## 9) Launch UX requirements

- Receipt ledger always available.
- One-tap kill switch always visible.
- Approval queue visible within one interaction step.
- Budget and scope visibility without hidden menus.
