# Runbook: Key Compromise

## Purpose
Contain and recover from compromised keys while preserving audit trails.

---

## Trigger conditions
- Verified unauthorized signing activity.
- Lost or stolen operator key material.
- Evidence of leaked signing secrets.

---

## Immediate actions

1) Revoke compromised keys on-chain.
2) Rotate to new keys with explicit receipt issuance.
3) Invalidate active grants or approvals tied to compromised keys.
4) Suspend affected app manifests and releases.

---

## Verification

1) Verify revocation status across nodes and clients.
2) Confirm new keys are propagated and accepted by policy gates.
3) Validate receipt chain continuity for audit integrity.

---

## Post-compromise steps

- Issue a signed disclosure report.
- Update key custody policies and enforcement checks.
- Add detection rules to monitoring and governance.
