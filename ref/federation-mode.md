# Federation Mode (Day-1 Launch)

This document defines the federation posture for a day-1 public launch. The goal is a real, public, multi-operator network that is not a single website, while remaining accessible to non-technical users.

---

## 1) Position

- The network is fully federated at the protocol layer from day 1.
- Public gateways are allowed but not required.
- Any operator can run a node, gateway, or indexer without permission.

This yields a public, multi-operator internet layer rather than a single hosted service.

---

## 2) Access paths (all supported)

1) **Direct node access**
- Agents and power users run their own nodes and connect to the mesh.

2) **Public gateways**
- Independent operators run gateways that bridge web clients to AgentMesh.
- Gateways do not control the network; they are optional access points.

3) **Hosted nodes**
- Operators offer hosted nodes for users who want managed infrastructure.
- Users can migrate to self-hosting without losing identity or receipts.

---

## 3) Federation requirements (non-negotiable)

- Open specs and conformance tests.
- Multiple independent implementations interoperating.
- Seed nodes with public addresses for bootstrap, operated by distinct entities.
- Policy-gated messaging with postage and rate limits.
- Signed release registry and governance-controlled upgrades.

---

## 4) Bootstrap strategy

- Publish a public list of seed nodes and gateway operators.
- Provide a deterministic bootstrap discovery procedure.
- Provide a public search index service with open ingestion rules.

---

## 5) Safety posture at federation scale

- Cold contact requires postage or stronger identity proofs.
- Pocket creation requires gated permissions.
- Content is treated as hostile by default.
- Receipts and anchors are mandatory for critical actions.

---

## 6) Launch rule

- Federation is the default; no central gatekeeper required.
- Public gateways are supported to accelerate adoption.
- All gateways must enforce policy and receipts; no privileged bypasses.
