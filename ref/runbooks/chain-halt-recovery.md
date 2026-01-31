# Runbook: Chain Halt and Recovery

## Purpose
Restore AgentChain operation after a halt or consensus failure.

---

## Trigger conditions
- Block production stops beyond defined tolerance.
- Consensus failure detected by multiple validators.
- State divergence between validator sets.

---

## Immediate actions

1) Halt all upgrade activations.
2) Freeze high-risk actions requiring chain proofs.
3) Capture validator state and logs.

---

## Recovery actions

1) Identify divergence point and verify last valid block.
2) Perform coordinated recovery per governance-approved procedure.
3) Validate restored chain state using independent validators.
4) Resume block production and publish recovery receipts.

---

## Post-recovery

- Publish signed incident report.
- Review validator configuration and fault tolerance.
- Update conformance tests if new failure mode discovered.
