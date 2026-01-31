# Pairing UX (Human <-> Agent)

This document defines a pairing method that is easier than API keys while being safer and fully auditable.

---

## 1) Goals

- Faster than API keys.
- No static secrets or copy-paste tokens.
- Explicit scope, budget, and risk tiers.
- Revocable in one step.
- Emits receipts for every pairing event.

---

## 2) Pairing primitives

- **Pairing Code**: short-lived, single-use code or QR.
- **Pairing Contract**: signed by both parties with scopes, budgets, and expiry.
- **Grant**: scoped delegation tied to the pairing contract.
- **Approval**: per-intent authorization when required.

---

## 3) Default pairing flow (human-first)

1) Human opens Operator Console and selects Pair Agent.
2) Console displays a QR or device code with a short expiration.
3) Agent scans or inputs code and proves its DID.
4) Human approves scopes, budgets, and risk mode.
5) Pairing Contract is co-signed and anchored.
6) Receipts are emitted for all steps.

This removes the need for static API keys.

---

## 4) Remote pairing flow (agent-first)

1) Agent requests pairing and presents a verifiable DID.
2) Human receives a pairing request in the approval queue.
3) Human approves or rejects with explicit scope and budgets.
4) Pairing Contract is signed and anchored.
5) Receipts emitted and stored.

---

## 5) Organization pairing flow

1) Org admin creates an agent role policy bundle.
2) Multi-admin approval is required for high-risk scopes.
3) Pairing Contract includes policy bundle hash and revocation path.
4) Receipts are anchored and auditable.

---

## 6) Authentication method

- Human authentication uses passkeys or equivalent strong credentials.
- Agent authentication uses DID-based proof and short-lived challenge response.
- No persistent bearer secrets are issued; tokens are short-lived and scoped.

---

## 7) Safety controls

- Pairing codes expire quickly and cannot be reused.
- Pairing requires explicit confirmation on the human side.
- Any scope expansion triggers a new approval flow.
- Revocation is immediate and recorded.

---

## 8) UX requirements

- Pairing must complete in under a minute for default flows.
- Scope, budget, and risk tiers are visible before approval.
- Receipts for pairing are visible in the ledger immediately.
