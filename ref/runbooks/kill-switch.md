# Runbook: Kill Switch

## Purpose
Provide a single-operator, hardware-backed emergency control to halt outbound activity and reduce blast radius during a critical incident.

---

## Authority model
- Exactly one operator controls the kill switch key.
- The kill switch key is stored in hardware-backed custody.
- All kill switch actions produce signed payloads and receipts.
- Network-wide release payloads are disabled by default; release is a local operator action.

---

## Preconditions
- Kill switch public key is configured in all nodes.
- Kill switch topic and payload type are registered and validated.
- Receipt logging and alerting are operational.

---

## Engage procedure

1) Verify the incident severity warrants network-wide halt of outbound actions.
2) Authenticate as the single kill switch operator using hardware-backed credentials.
3) Create a signed kill switch payload with:
   - action = engage
   - reason = incident classification
   - timestamp = current Unix time
   - nonce = random 16 bytes
4) Publish the kill switch payload to the kill switch topic.
5) Confirm:
   - Nodes acknowledge the kill switch event.
   - Outbound publish and DHT updates are blocked.
   - Receipts show the kill switch activation.

---

## Release procedure

1) Validate containment and integrity checks are complete.
2) Authenticate as the same kill switch operator.
3) Perform a controlled release by manually disabling kill switch enforcement on each node (local operator action).
4) Restart nodes and verify outbound activity resumes only after policy gates pass.
5) Confirm:
   - Nodes re-enable outbound activity.
   - Receipt logs show the release action and subsequent policy decisions.

---

## Audit requirements
- Preserve signed kill switch payloads as immutable artifacts.
- Anchor receipts that include kill switch events.
- File a post-incident report referencing the activation and release.
