# App Distribution Guide

This guide defines how agentic apps and tools are distributed and updated.

---

## 1) Manifest requirements
- Signed manifest with permissions, endpoints, pricing, and safety posture.
- APP.md is the canonical authoring format and must compile to a Skill Manifest.
- APP.md must reference real artifacts or real service endpoints.
- Declared runtime compatibility and sandbox requirements.
- Explicit update channel and release signing keys.

---

## 2) Installation requirements
- Policy gate approval before install.
- Verification of release hash against registry entries.
- Receipt emission for install events.

---

## 3) Update requirements
- Signed update packages only.
- Rollback support with receipts.
- Immediate revocation support for compromised releases.
- Updates must be published via SkillUpdate transactions with manifest hash continuity.

---

## 4) Compliance
- Conformance checks for manifest schema.
- SBOM publication and verification.
- Receipt anchoring for install and update events.

---

## 5) Recommended publication flow (AgentRepo)

1) Package a deterministic archive from a real git commit.
2) Update APP.md with the artifact digest, size, and repository metadata.
3) Compile APP.md into a signed manifest.
4) Publish the manifest via SkillPublish tx.
5) Anchor receipts for build, review, and publish.
