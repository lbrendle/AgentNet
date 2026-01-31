# Markdown Exchange Profile (Deterministic, Safe)

This document defines the strict Markdown profile used for human<->agent exchange. The profile is designed for deterministic parsing and rendering across implementations while preventing execution or rendering ambiguity.

---

## 1) Profile goals

- Deterministic parsing across languages.
- Safe rendering with no executable content.
- Stable canonicalization for receipts and hashing.
- Strict separation of human-readable content from authoritative data.

---

## 2) Allowed block elements

- Paragraphs
- ATX headings (`#` to `######`)
- Unordered lists (dash `-` only)
- Ordered lists (decimal `1.` only)
- Block quotes
- Fenced code blocks (backticks only)
- Horizontal rules (`---` only)

Disallowed blocks:
- HTML blocks
- Tables
- Footnotes
- Definition lists
- Indented code blocks

---

## 3) Allowed inline elements

- Emphasis and strong emphasis
- Inline code
- Links with restricted schemes
- Hard line breaks (two-space line endings are NOT allowed; use explicit `\\` line breaks if needed)

Disallowed inline elements:
- Inline HTML
- Images
- Autolinks
- Raw HTML entities that resolve to tags

---

## 4) Link policy

- Only the following schemes are allowed: `https`, `agentnet`, `did`.
- Any other scheme MUST be rejected at parse time.
- Links are rendered as text plus URL, with no automatic fetching.

---

## 5) Canonicalization rules

Every compliant renderer MUST produce canonical output with the following rules:

- Normalize line endings to `\n`.
- Replace tabs with four spaces before parsing.
- Remove trailing whitespace on every line.
- Collapse runs of more than two blank lines into two.
- Render headings using `#` style with a single space after the marker.
- Render unordered lists using `-` and ordered lists using `1.`
- Render block quotes with a single `>` and a single space.
- Render fenced code blocks with triple backticks and no language info string unless explicitly provided.
- Escape any characters that could create disallowed constructs.

Canonicalization is applied before hashing and signature validation.

---

## 6) Size and safety limits

- Maximum Markdown payload size is enforced at the policy gate.
- Maximum line length is enforced to avoid parser abuse.
- Parsing failures MUST reject the message and emit a receipt.

---

## 7) Rendering requirements

- Rendering is deterministic and does not execute scripts, load remote resources, or interpret HTML.
- All content is treated as untrusted input.
- Rendering MUST preserve source order and avoid reflow that would change semantics.

---

## 8) Interop requirements

- All implementations MUST pass the Markdown profile compliance tests.
- The profile version is pinned and negotiated in `NodeHello.features`.
- Any profile mismatch MUST result in a policy downgrade or message rejection.
