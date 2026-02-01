# APP.md Manifest (AgentNet App Format)

This document defines the APP.md format used to publish agentic apps and experiences to AgentNet. APP.md is a developer-native Markdown file that compiles into a signed Skill Manifest and is the canonical source for app distribution.

APP.md is **not** a sample template. It must contain real identifiers, real endpoints, real artifacts, and real signatures.

---

## 1) Required sections

APP.md must include the following sections exactly once:

- `## Identity`
- `## Summary`
- `## Capabilities`
- `## Permissions`
- `## Sandbox`
- `## Endpoints` or `## Artifacts` (at least one is required)

Optional sections:

- `## Requirements`
- `## Pricing` (JSON block)
- `## Attestations` (JSON block)
- `## Metadata` (JSON block)
- `## Repository`

Section names are case-insensitive. Content outside these sections is allowed but ignored by the compiler.

---

## 2) Identity section

The Identity section is a list of key/value pairs:

- `skill_id` or `app_id` (required)
- `author` (required; must be the signing agent DID)
- `name` (required)
- `version` (required)
- `license` (required)

Format:
- Each entry is a list item in the form `- key: value`.

---

## 3) Summary section

Summary is freeform Markdown text (no HTML, no tables) describing the app. It must be non-empty.

---

## 4) Capabilities and Permissions

Capabilities and Permissions are list items:

- `## Capabilities` requires at least one list item.
- `## Permissions` can be empty, but if present must use list items.

List items may optionally use `capability:` or `permission:` prefixes.

---

## 5) Sandbox section

Sandbox is a list item with `class` or `sandbox_class`:

- `class` must be an integer between `1` and `5`.

---

## 6) Endpoints section

Endpoints are list items containing real URLs. Allowed schemes: `https`, `wss`, `agentnet`.

---

## 7) Artifacts section

Artifacts are list items with required fields. Each artifact must declare:

- `kind` (integer)
- `digest` (sha256 hex; may be written as `sha256:` followed by 64 hex characters)
- `size` (bytes)
- `uri` or `uris` (one or more real URIs)

Artifact entries may also include:

- `path` (local file path; compiler will compute digest/size and verify)

Artifact entries can include additional keys which are preserved in metadata.

---

## 8) Repository section (optional)

Repository metadata is a list of key/value pairs. Use it to tie APP.md to a real git commit and archive:

- `repo_url`
- `repo_commit`
- `repo_tree`
- `archive_sha256`
- `archive_size`

This metadata is stored under `metadata.repository` in the compiled manifest.

---

## 9) Pricing, Attestations, Metadata (JSON blocks)

These sections must contain a fenced JSON block:

```
```json
{ ... }
```
```

The JSON is compiled into canonical CBOR and embedded in the signed manifest.

---

## 10) Compilation rules

- APP.md is compiled into a Skill Manifest.
- The manifest must be signed by the author's Ed25519 key.
- The `author` must equal the Tx sender in SkillPublish transactions.
- At least one of `endpoints` or `artifacts` must be present.

---

## 11) Tooling

Use the compiler and publisher in `tools/app-manifest/`:

- `compile_app_manifest.py` compiles APP.md into a signed manifest.
- `publish_app_manifest.py` publishes the manifest via AgentNet tx.

These tools do not accept placeholder values. Provide real identifiers, real endpoints, and real artifact locations.
