# App Distribution Guide

This guide defines how agentic apps and tools are distributed and updated.

---

## 1) Manifest requirements
- Signed manifest with permissions, endpoints, pricing, and safety posture.
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

---

## 4) Compliance
- Conformance checks for manifest schema.
- SBOM publication and verification.
- Receipt anchoring for install and update events.
