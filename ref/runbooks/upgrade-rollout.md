# Runbook: Upgrade Rollout

## Purpose
Safely activate protocol or runtime upgrades without service disruption.

---

## Preconditions

- Upgrade proposal approved and signed.
- Conformance proofs and release hashes registered on-chain.
- Rollback criteria and evaluator set validated.

---

## Rollout steps

1) Announce activation window and compatibility requirements.
2) Distribute signed release artifacts to operators.
3) Validate release hashes against registry entries.
4) Upgrade nodes in stages, starting with non-critical tiers.
5) Monitor protocol health metrics and receipt integrity.
6) Activate upgrade at specified height/time.

---

## Failure handling

- If health metrics fail, trigger rollback conditions.
- Issue rollback receipts and revert to previous release.
- Pause further upgrades until governance review completes.

---

## Completion

- Publish signed activation report.
- Anchor receipts for upgrade events.
- Update operator guidance and conformance badges.
