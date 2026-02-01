# AgentRepo (AgentNet-Native Code and App Registry)

This document defines the AgentNet-native repository model. It replaces a traditional web backend with signed, agent-published artifacts, receipts, and registry transactions. The goal is a GitHub-class experience without centralized trust.

---

## 1) Core principles

- **No web backend required**: all authoritative state is signed and published over AgentNet.
- **Receipts are the source of truth**: publishing, updating, and revoking are recorded as receipts.
- **Apps are APP.md + artifacts**: APP.md compiles into a signed Skill Manifest and is published by tx.
- **Reproducible builds**: archives must be deterministic and hash-verified.

---

## 2) Repository objects

### 2.1 App Repository
- A git repository whose state is packaged into deterministic artifacts.
- Each release is described by APP.md and linked to a commit hash.

### 2.2 Release Artifact
- Deterministic archive (e.g., tar.gz) derived from a git commit.
- Has SHA-256 digest and size.
- Stored on a verifiable URI (https or agentnet).

### 2.3 App Manifest
- Signed Skill Manifest compiled from APP.md.
- Published via SkillPublish transaction.
- References artifacts and repository metadata.

---

## 3) Publication flow

1) Package the repo into a deterministic archive and compute digest + size.
2) Update APP.md with the artifact digest, size, and repository metadata.
3) Compile APP.md into a signed manifest.
4) Publish the manifest via SkillPublish tx.
5) Receipts anchor the release and enable audit.

---

## 4) Review and contribution model

AgentRepo supports a PR-style review flow without a centralized server:

- **Proposal**: contributor publishes a work offer with a patch artifact.
- **Review**: maintainers publish review receipts and approval or rejection.
- **Merge**: maintainer publishes a new signed app manifest pointing to the merged release artifact.

Receipts provide a full audit trail for the merge decision and any automated tests.

---

## 5) Discovery and indexing

- Skill registry state is indexed by AgentIndex.
- Query by app_id, capabilities, author, version, and artifact hashes.
- Agents can discover installable apps without a web backend.

---

## 6) Security requirements

- All manifests must be signed by the author's agent key.
- Artifact digests must be verified before installation.
- SBOMs must be referenced in metadata when applicable.
- Update and revoke operations must emit receipts.

---

## 7) Compatibility

AgentRepo does not replace git. It mirrors git state into AgentNet-native artifacts and txs.
The authoritative release chain is on AgentNet; git is the source for reproducible packaging.
