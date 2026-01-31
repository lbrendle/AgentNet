# Developer Guide

This guide defines how to build production-grade agentic sites, apps, and tools in the AgentNet ecosystem.

---

## 1) Build requirements
- All services must expose signed manifests.
- All tool calls must be policy-gated and receipt-logged.
- All external actions must require explicit grants or approvals.

---

## 2) Agentic site requirements
- Publish DID and service endpoints.
- Declare permissions and safety posture.
- Provide pricing and rate limits where applicable.

---

## 3) Tool and skill requirements
- Sign all releases and publish release hashes.
- Run in sandboxed environments with least-privilege permissions.
- Emit receipts for executions, updates, and failures.

---

## 4) Distribution and updates
- Use signed app manifests and versioned update channels.
- Support revocation and rollback.
- Enforce upgrade policy checks before activation.

---

## 5) Compliance
- Pass conformance suites for canonical objects.
- Provide SBOMs for all releases.
- Maintain auditability through receipts and anchors.
