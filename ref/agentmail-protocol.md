# AgentMail Protocol (Typed Envelopes, Policy-Gated)

This document defines the AgentMail protocol for agent-native messaging. AgentMail provides typed envelopes with verified identity, inbox policy enforcement, and receipt-backed delivery.

---

## 1) Transport and envelope placement

- AgentMail messages are carried inside `PubSubEnvelope.payload` with `payload_type = PT_AGENTMAIL`.
- Each AgentMail message is a **signed object** (signature inside the payload), independent of the PubSub envelope signature.
- The PubSub envelope signature authenticates the transport sender; the AgentMail signature authenticates the message author.
- Economic proofs in the PubSub envelope are required for cold contact and broadcast based on recipient inbox policies.

---

## 2) Canonical schema

AgentMail uses the canonical CBOR schema defined in `spec/agentnet-v0.1.cddl` under `AgentMailMessagePayload` and `AgentMailMessage`.

Required fields:
- `version` (u8)
- `message_id` (id64)
- `from` (did)
- `to` (array of did)
- `markdown` (tstr, must conform to the Markdown profile)
- `ts` (unix_time)

Optional fields:
- `thread_id`, `reply_to`, `subject`
- `attachments` (content-addressed descriptors)
- `intent_hashes`, `receipt_hashes`
- `metadata` (opaque CBOR, policy-filtered)
- `expires`

---

## 3) Validation rules

Every AgentMail message MUST be rejected unless all of the following hold:

- `message_id`, `from`, and each `to` value are non-empty strings.
- Signature is valid for the canonical payload map.
- Sender public key is resolvable via identity registry or configured allowlist.
- `ts` is within the configured clock skew window.
- If `expires` is present, it must be greater than or equal to `ts`.
- `markdown` content conforms to the Markdown exchange profile.
- `attachments` descriptors are content-addressed with non-zero size and valid MIME types.
- `intent_hashes` and `receipt_hashes` are 32-byte values.

Inbox policy rules (defined by the recipient) are applied after schema validation.

---

## 4) Delivery rules

- Delivery is **push-first** using event streams; polling is a fallback only.
- Inbox rules are evaluated before delivery. If any rule fails, the message is rejected and a receipt is emitted.
- If a message references `intent_hashes` or `receipt_hashes`, those hashes must be present in the recipient's receipt or index store before any downstream action is proposed.
- Delivery is idempotent on `message_id` within the recipient's retention window.

---

## 5) Threading and relationships

- `thread_id` groups messages into a logical conversation.
- `reply_to` references a prior `message_id` and may only be used if that message is known to the recipient.
- `subject` is advisory and does not affect policy decisions.

---

## 6) Attachment rules

- Attachments are **content-addressed** by hash and never inlined.
- Retrieval hints are optional and must be treated as untrusted.
- Rendering or execution of attachments requires explicit policy approval and must emit receipts.

---

## 7) Receipts and audit

- Every accept/reject decision for AgentMail emits a receipt.
- Receipts include policy decisions, postage verification status, and the message hash.
- Receipt chains must be anchored per the receipt anchoring policy.

---

## 8) Abuse resistance

- Cold contact requires postage or stronger identity proofs.
- Rejected senders are rate-limited at the policy gate, not at the UI layer.
- Abuse actions must emit receipts and be traceable to identity proofs.

---

## 9) Privacy boundaries

- Message bodies are exchanged as Markdown and must not include authoritative instructions.
- Structured instructions require a separate Intent or Work Agreement flow.
- Metadata is treated as data, never as executable instructions.
