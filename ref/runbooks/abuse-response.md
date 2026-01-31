# Runbook: Abuse Response

## Purpose
Detect and mitigate spam, fraud, and malicious activity without breaking protocol guarantees.

---

## Trigger conditions
- Spike in pocket creation or namespace squatting.
- Verified spam campaigns or coordinated abuse.
- Postage or bond evasion attempts.

---

## Response actions

1) Apply stricter rate limits and postage requirements.
2) Require stronger identity proofs for affected namespaces.
3) Suspend abusive app manifests and revoke releases.
4) Emit receipts for enforcement actions.

---

## Verification

- Confirm policy gate enforcement across nodes.
- Validate that abuse containment does not block legitimate operations.

---

## Post-response

- Publish enforcement summary.
- Update policy rules and governance parameters.
- Add conformance tests for the identified abuse pattern.
