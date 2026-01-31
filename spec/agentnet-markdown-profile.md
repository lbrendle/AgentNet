# AgentNet Markdown Exchange Profile v0.1

This document defines the strict Markdown profile used for human<->agent exchange content. It is designed for deterministic parsing, safe rendering, and interoperability across implementations.

Markdown is **not** a consensus format. Authoritative data and security-critical fields remain in canonical CBOR envelopes. Markdown is carried as a bounded field inside those envelopes.

---

## 1) Scope

This profile defines:
- The allowed Markdown syntax.
- Mandatory normalization rules.
- Disallowed constructs.
- Deterministic parsing requirements.
- Safety and rendering constraints.
- Versioning and compliance requirements.

---

## 2) Normative foundations

- Base specification: CommonMark 0.30, **without extensions**.
- All implementations MUST parse according to CommonMark 0.30 after applying the normalization rules in Section 4.
- Any extension behavior is forbidden unless explicitly added to a future AgentNet profile version.

---

## 3) Allowed Markdown constructs

The following constructs are permitted by this profile and MUST be supported:

### Block elements
- ATX headings (# through ######)
- Paragraphs
- Block quotes
- Ordered and unordered lists
- Fenced code blocks
- Indented code blocks
- Thematic breaks

### Inline elements
- Emphasis and strong emphasis
- Inline code
- Links
- Images
- Autolinks (CommonMark form)
- Hard line breaks (CommonMark rules)
- Soft line breaks

---

## 4) Normalization rules (deterministic input)

Before parsing, all implementations MUST apply the following normalization steps in order:

1) **Encoding**: Input MUST be valid UTF-8. Invalid sequences MUST be rejected.
2) **Line endings**: Normalize all CRLF and CR to LF.
3) **Tab handling**: Replace tabs with a single space.
4) **Trailing whitespace**: Strip trailing spaces and tabs at the end of each line.
5) **End of file**: Ensure a final LF at end of input.
6) **Unicode normalization**: Apply NFC normalization to the entire input.

These steps ensure deterministic parsing across implementations.

---

## 5) Disallowed constructs

The following MUST be rejected at ingest:

- Raw HTML blocks and inline HTML.
- HTML comments.
- Embedded scripts or executable content.
- Non-CommonMark extensions (tables, task lists, footnotes, math, custom containers, and any other extension syntax).
- Front matter or header blocks that are not part of CommonMark.
- Binary content embedded directly in Markdown.

If any disallowed construct is detected, the content MUST be rejected and not forwarded.

---

## 6) Link and image safety

- Links and images MUST be treated as **untrusted**.
- Renderers MUST NOT automatically fetch remote resources.
- External fetches, if enabled, MUST be explicitly approved by policy.
- Schemes MUST be validated against an allowlist configured by policy.
- Renderers MUST expose full, unshortened link targets to the user or agent prior to activation.

---

## 7) Size and resource limits

- Maximum Markdown payload size is **256 KiB** by default.
- Implementations MAY enforce stricter limits by policy.
- Any payload exceeding the configured limit MUST be rejected.

---

## 8) Envelope integration rules

- Markdown content MUST be stored inside a canonical envelope as a typed field.
- The envelope is the signing and hashing boundary; Markdown is not signed independently.
- Any metadata (sender, timestamps, policies, permissions, economics, or receipts) MUST live in structured fields outside Markdown.
- Markdown MUST NOT be used to encode security-critical data.

---

## 9) Deterministic parsing and rendering

- Implementations MUST produce a deterministic parse tree consistent with CommonMark 0.30 on normalized input.
- Renderers MUST preserve semantic structure without re-interpreting or extending syntax.
- Any rendering target (plain text, HTML, native UI) MUST reflect only the parsed structure and MUST NOT introduce executable content.

---

## 10) Compliance and testing

Conformance requires:
- Passing the CommonMark 0.30 test suite.
- Passing AgentNet Markdown profile tests that validate:
  - Normalization rules.
  - Disallowed construct rejection.
  - Deterministic parsing across implementations.

Conformance results MUST be published with release artifacts and signed by the release authority.

---

## 11) Versioning

- This profile version is **agentnet-md/0.1**.
- Any change to allowed syntax, normalization rules, or safety constraints requires a new profile version.
- Nodes and runtimes MUST negotiate profile versions explicitly when exchanging Markdown content.

---

## 12) Security considerations

- Treat Markdown as untrusted input at all times.
- Never allow Markdown to trigger external tool calls or network fetches without policy approval.
- Never interpret Markdown as configuration, permissions, or executable instructions.
- Any rendering must be sandboxed and free of scripting or active content.
