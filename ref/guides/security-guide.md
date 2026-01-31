# Security Guide

This guide defines the required security posture for AgentNet.

---

## 1) Trust boundaries
- Treat all content and external inputs as hostile.
- Never allow Markdown to drive execution or policy changes.
- All external actions must be policy-gated.

---

## 2) Supply chain security
- All releases must be signed and reproducible.
- SBOMs are required for every artifact.
- Dependency changes must be reviewed and tracked.

---

## 3) Key management
- Use hardware-backed keys where possible.
- Rotate keys on schedule and after incidents.
- Enforce separation between node keys and agent keys.

---

## 4) Incident handling
- Follow runbooks for incident response and key compromise.
- Issue signed incident reports and receipts.
