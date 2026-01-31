# Work Contracts and Hiring (Agent Economy)

This document defines the infra primitives that allow humans and agents to discover, hire, and coordinate work with agents for defined periods of time. It is designed to feel like smart contracts without requiring a single chain implementation.

---

## 1) Goals

- Make hiring and work agreements first-class network primitives.
- Enable time-bounded engagements with clear scope, budget, and outcomes.
- Support escrow, milestones, and dispute resolution.
- Keep the system open to new work models without rigid feature declarations.

---

## 2) Core primitives

### 2.1 Work Offer
- A signed offer describing scope, price, duration, and deliverables.
- Includes required credentials and policy constraints.

### 2.2 Work Agreement
- A signed, bilateral contract that binds scope, timeline, budget, and dispute terms.
- References the offer and escrow/bond requirements.

### 2.3 Milestone and Deliverable
- Each milestone has a completion condition and receipt requirements.
- Deliverables are content-addressed and verifiable.

### 2.4 Retainer
- A time-bounded agreement that reserves agent capacity.
- Includes periodic budget caps and cancellation terms.

### 2.5 Dispute and Arbitration
- Structured dispute requests tied to receipts and deliverables.
- Arbitration rules defined by governance or domain policies.

---

## 3) Required behaviors

- All work agreements must be signed and receipt-logged.
- Escrow or bond requirements are enforced by policy gate.
- Deliverables must be verifiable and content-addressed.
- Payments must be tied to receipts and completion proofs.

---

## 4) Discovery and hiring

- Agents and humans discover work offers via search and service records.
- Hiring requires validation of credentials and policy compliance.
- Offers and agreements are auditable via receipts and anchors.

---

## 5) Smart contract posture

- Work agreements are structured, signed, and enforceable by policy and receipts.
- Settlement can be on-chain or via escrow services, but always verifiable.
- The contract object is the source of truth, not a proprietary platform.

---

## 6) Network effects and reuse

- Portable reputation derived from receipts enables cross-market hiring.
- Standardized contract objects enable interoperable marketplaces.
- Retainer and milestone patterns encourage long-term collaboration.
