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
- Restrict kill switch key custody to a single operator.
- Disable remote kill switch release; require local operator action to restore service.
- Treat transaction signer registries as critical security assets.
- Identity registry writes are high-risk and require explicit policy approval.
- Economic proof verification must fail closed by default.

---

## 4) Incident handling
- Follow runbooks for incident response and key compromise.
- Issue signed incident reports and receipts.
