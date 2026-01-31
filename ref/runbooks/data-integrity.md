# Runbook: Data Integrity and Receipt Verification

## Purpose
Validate integrity of receipts, anchors, and critical logs.

---

## Trigger conditions
- Receipt chain verification failure.
- Anchor mismatch between nodes.
- Audit request from governance or operators.

---

## Verification steps

1) Recompute receipt hashes for the affected chain range.
2) Compare local receipt chain head to latest anchor.
3) Validate signatures for receipts and anchors.
4) Check for missing or reordered receipts.

---

## Remediation

- If missing receipts are detected, quarantine affected node logs.
- Rebuild local receipt log from trusted sources.
- Issue integrity receipts for the recovery procedure.

---

## Post-verification

- Publish signed integrity report.
- Update monitoring to detect similar issues earlier.
