# Repository Organization

This document defines the repository structure, ownership boundaries, and artifact responsibilities. It is designed for a production-grade, multi-language, agent-upgradeable network.

---

## 1) Top-level structure

```
/ref
  docs-index.md
  implementation-plan.md
  architecture.md
  roadmap.md
  ui-ux.md
  interaction-model.md
  ecosystem-insights.md
  details-backlog.md
  runbooks/
  guides/

/spec
  agentnet-v0.1.cddl
  agentnet-state-machines-v0.1.md
  agentnet-conformance-testplan-v0.1.md
  agentnet-test-vectors-v0.1.json
  agentnet-markdown-profile.md

/impl
  /rust
  /python
  /ts
  /swift

/tools
  conformance-runner
  vector-generator
  interop-harness
  release-signer

/templates
  agentic-site
  pocket-host
  marketplace
  agent-runtime

/infra
  deployment
  monitoring
  security

/governance
  proposals
  votes
  trials
  releases
```

---

## 2) Ownership boundaries

- **/spec**: protocol definitions, canonical schemas, and normative rules.
- **/impl**: production implementations and SDKs.
- **/tools**: conformance, interoperability, and release tooling.
- **/templates**: production-grade scaffolds for agentic sites and services.
- **/infra**: operational deployment, monitoring, and security infrastructure.
- **/governance**: proposal artifacts, voting outcomes, and release metadata.
- **/ref**: design, runbooks, guides, and operational documentation.

---

## 3) Release artifacts

- Release artifacts must be reproducible and signed.
- SBOMs are required for all releases.
- Release hashes are registered on-chain with activation metadata.

---

## 4) Documentation governance

- Docs are versioned with protocol releases.
- Runbooks and guides must match current release behavior.
- Any change to protocol or workflow requires doc updates as part of the same release.
