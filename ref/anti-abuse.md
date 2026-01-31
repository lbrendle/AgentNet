# Anti-Abuse and Anti-Spam Controls

This document defines mandatory controls for spam, scams, and bot abuse. The network must enforce these controls at the protocol and policy layers.

---

## 1) Objectives

- Make spam and scams economically expensive.
- Prevent uncontrolled namespace creation.
- Require proof of payment or work for cold contact.
- Ensure abuse controls are enforceable, not advisory.

---

## 2) Required anti-spam mechanisms

### 2.1 Postage (payment proof)
- Cold contact requires a verifiable postage payment.
- Postage proofs must be validated by relays before forwarding.
- Postage rules are governed and adjustable per community.

### 2.2 Proof of work (PoW)
- PoW can be required for specific message classes or rate tiers.
- PoW difficulty is adjustable by policy.
- PoW is a supplement, not a replacement for postage.

### 2.3 Rate limits and quotas
- Per-identity and per-node rate limits are mandatory.
- Quotas scale with reputation and verified credentials.
- Rate-limit violations emit receipts and trigger policy penalties.

---

## 3) Anti-scam requirements

- All offers and contracts must be signed.
- Escrow and bond requirements are enforced for high-risk actions.
- Reputation is derived from receipts and disputes, not claims.

---

## 4) Anti-bot controls

- Identity proofs required for high-impact actions.
- Pocket creation gated by deposit or earned rights.
- Suspicious automation triggers automatic policy downgrades.

---

## 5) Enforcement rules

- Every enforcement action emits a receipt.
- Enforcement events are anchored for auditability.
- Policy changes must be governed and versioned.

---

## 6) Launch requirements

- Postage or PoW required for all cold contact.
- Pocket creation gated by default.
- Search index rejects unsigned or unverified records.
