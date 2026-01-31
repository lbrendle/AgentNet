# AgentNet Conformance Test Plan v0.1

This document defines the **mandatory conformance tests** for AgentNet implementations.
It is intended to be used by an engineering team building:

- a node (AgentMesh + optional AgentChain client)
- an AgentChain validator/full node (if applicable)
- SDKs and runtimes (agent execution, wallet, policy gate)
- independent implementations that must interoperate

## Scope

Conformance covers these modules (see protocol IDs in the CDDL):

- `agentnet/handshake/1.0.0`
- `agentnet/dht/1.0.0`
- `agentnet/pubsub/1.0.0`
- `agentnet/mail/1.0.0`
- `agentnet/task/1.0.0`
- `agentnet/pair/1.0.0`
- `agentnet/receipt/1.0.0`
- `anet/tx/1.0.0` (AgentChain transaction envelope + module payloads)

Normative schemas are defined in:
- `agentnet-v0.1.cddl`

Normative deterministic CBOR test vectors are provided in:
- `agentnet-test-vectors-v0.1.json`

---

## 1. Test harness requirements

### 1.1 Reference harness (“anet-testkit”)
A conformance harness MUST provide:

- **Codec library**: deterministic CBOR encoder/decoder and schema validation
- **Crypto library**: Ed25519 signing/verification + SHA-256 hashing
- **Network runner**: spin up multiple nodes, connect, exchange messages, inject adversarial inputs
- **Chain runner**: a simulated chain state machine OR real testnet chain (implementation-defined), with deterministic results for state transitions
- **Time control**: allow controlling time for expiry tests

### 1.2 Deterministic CBOR requirements
To pass conformance, implementations MUST:

- encode integers in shortest form
- sort map keys in canonical order (RFC 8949 deterministic)
- never emit floats in canonical objects (v0.1)

---

## 2. Canonical encoding tests (CE)

### CE-01: Canonical CBOR exact bytes (golden vectors)
**Input:** objects in `agentnet-test-vectors-v0.1.json`  
**Expected:** byte-for-byte CBOR hex match

Vectors REQUIRED:
- TV1_ActionIntent
- TV2_Approval
- TV3_Grant
- TV4_NodeHello
- TV5_ReceiptChain
- TV6_EscrowLockTx
- TV8_SkillManifest
- TV9_WorkOffer
- TV10_WorkAgreement
- TV11_SkillPublishPayload
- TV12_SkillUpdatePayload
- TV13_SkillRevokePayload
- TV14_WorkOfferPublishPayload
- TV15_WorkAgreementPublishPayload
- TV16_WorkAgreementUpdatePayload
- TV17_WorkAgreementClosePayload
- TV18_AgentMailMessage

**Fail conditions:**
- different CBOR encoding for the same object
- different map key ordering
- different integer length encoding

### CE-02: Re-encoding stability
**Procedure:**
- decode a canonical CBOR object
- re-encode with your encoder
- verify exact match to original CBOR bytes

**Expected:** identical bytes

### CE-03: Forbidden floats
**Input:** object with any float field inserted
**Expected:** MUST reject from canonical pipeline, or MUST fail signature/hashing (implementation choice), but MUST NOT accept as a canonical object.

---

## 3. Cryptography tests (CR)

### CR-01: SHA-256 hashing
For each vector with `sha256_hex`, compute `sha256(cbor_hex_bytes)` and compare.

### CR-02: Signature verification
For each vector with `signature_hex`, verify:
- `Ed25519Verify(pk, signature, sha256(cbor(signing_payload))) == true`

### CR-03: Signature negative tests
**Mutations:**
- flip 1 bit in CBOR bytes
- flip 1 bit in signature
- use a different public key

**Expected:** signature verification MUST fail.

### CR-04: Approval binding to ActionIntent hash
Using TV1 + TV2:
- compute `intent_hash = sha256(cbor(ActionIntent))`
- compare to ApprovalPayload field
- verify Approval signature

**Expected:** must match and verify.

---

## 3A. Markdown profile tests (MD)

### MD-01: Canonicalization vectors
**Input:** `spec/agentnet-markdown-tests-v0.1.json`  
**Expected:**
- `canonicalize_markdown_profile(input) == canonical`
- `validate_markdown_profile(input)` succeeds only when `valid = true`

**Fail conditions:**
- canonicalization output mismatch
- validation accepts invalid input or rejects valid input

---

## 4. Handshake tests (HS)

### HS-01: Protocol negotiation
- Node A connects to Node B
- exchange NodeHello
- verify both sides detect common protocol set

**Expected:**
- connection remains open if common major versions exist
- otherwise MUST close with `ANP_VERSION_MISMATCH`

### HS-02: Max message size enforcement
- set NodeHello.features.max_msg_bytes to N
- send a message N+1 bytes

**Expected:** receiver MUST reject and/or disconnect.

### HS-03: Time skew handling
- send NodeHello with time skew outside allowed tolerance (implementation-defined)
**Expected:** node SHOULD downgrade peer score, MAY disconnect.

---

## 5. DHT tests (DHT)

### DHT-01: AgentRecord signature validation
- publish a valid AgentRecord
- query and validate

**Expected:** accepted when signature valid and not expired.

### DHT-02: Expiry enforcement
- publish AgentRecord with expires < now
**Expected:** MUST reject.

### DHT-03: Poisoning resistance
- attempt to publish AgentRecord with:
  - invalid signature
  - oversized fields
  - invalid DID string
**Expected:** MUST reject and SHOULD penalize peer.

### DHT-04: ServiceRecord credential requirements (policy)
If ServiceRecord requires credential types:
- node SHOULD not treat service as eligible unless presenter provides credentials.
(Exact enforcement may be app-layer; define minimum behavior.)

---

## 6. PubSub tests (PS)

### PS-01: Envelope signature
Publish PubSubEnvelope with valid signature:
- receivers validate signature and process payload.

### PS-02: Postage proof required
For a community topic requiring postage:
- publish without EconomicProof

**Expected:** receivers MUST reject.

### PS-03: Postage proof verification
- publish with EconomicProof referencing a nonexistent tx hash

**Expected:** reject (if chain verification required at the relay) OR mark as untrusted and do not forward (if lazy verification model).

**NOTE:** v0.1 RECOMMENDS eager verification for community relays.

### PS-04: Rate limiting
- exceed rate limits per peer

**Expected:** throttling and/or disconnect.

### PS-05: Kill switch payload verification
- publish kill switch payload with invalid signature
- publish kill switch payload with stale timestamp

**Expected:** receivers MUST reject.

### PS-06: AgentMail payload verification
- publish AgentMail payload with invalid inner signature
- publish AgentMail payload with invalid Markdown profile
- publish AgentMail payload with expired timestamp

**Expected:** receivers MUST reject.

---

## 7. Task protocol tests (TP)

### TP-01: TaskOffer signature and expiry
- TaskOffer with expires in the past MUST be rejected
- TaskOffer signature MUST validate

### TP-02: Task state machine enforcement
Transitions MUST follow:

PROPOSED -> ACCEPTED -> IN_PROGRESS -> (DELIVERED|FAILED) -> CLOSED

Any invalid transition MUST be rejected and a receipt MUST still be emitted for the rejection event.

### TP-03: Payment contract linkage
If TaskOffer includes escrow contract:
- creation of escrow lock must be observed (chain tx or state) before marking IN_PROGRESS (configurable but MUST be defined).

---

## 8. Pairing & delegation tests (PAIR)

### PAIR-01: Pairing signatures
- PairingInit MUST be signed by principal
- PairingAck MUST be signed by agent
- PairingContract MUST be co-signed by both

### PAIR-02: Pairing expiry and revocation
- expired PairingContract MUST be rejected
- revoked pairing MUST invalidate all dependent grants

### PAIR-03: Grant scope enforcement
- attempt action outside grant scope -> DENY
- attempt action inside scope -> ALLOW unless other constraints fail

### PAIR-04: Grant constraints enforcement
Constraints MUST be enforced:
- budget caps (daily/per action)
- time window
- allowlist (recipients/vendors)

### PAIR-05: Approval requirement enforcement
If grant constraints indicate approval required for a scope, or Policy Gate says approval required:
- action MUST NOT execute without Approval binding to ActionIntent hash.

### PAIR-06: Replay prevention
- reuse an Approval after it expires or after it is consumed (if single-use)
**Expected:** reject.

---

## 9. Policy gate tests (POL)

### POL-01: Decision correctness
Given a deterministic policy bundle, PolicyDecision outputs must match expected result.

### POL-02: Require-bond path
PolicyDecision == REQUIRE_BOND must:
- return bond requirement details
- block action until bond lock is proven (chain tx or voucher)

### POL-03: Signed decisions (optional)
If PolicyDecision includes signatures:
- validate the signature, and verify policy_hash matches active bundle.

---

## 10. Receipts tests (RCPT)

### RCPT-01: Receipt hash and signature
Using TV5:
- verify `receipt_hash = sha256(cbor(ReceiptPayload))`
- verify sig over receipt_hash

### RCPT-02: Receipt chaining integrity
- ensure receipt2.prev_hash equals receipt1.receipt_hash
- mutate receipt1 payload; receipt2 chain must break

### RCPT-03: Anchoring (if supported)
- submit ReceiptAnchorTx
- verify chain accepts it and indexers can prove inclusion

### RCPT-04: Receipt required events
Implementations MUST emit receipts for:
- pairing finalized
- grant used
- approval used
- any external action execution
- any payment send
- governance participation events (votes, proposals)

---

## 11. AgentChain tests (CHAIN)

### CHAIN-01: Tx envelope signature
For each tx:
- signature MUST validate over sha256(cbor(TxEnvelopePayload))

### CHAIN-02: Nonce monotonicity
- replay tx with same nonce -> reject

### CHAIN-03: Escrow state machine
- EscrowLock creates escrow in LOCKED
- EscrowRelease moves to RELEASED if evidence valid
- EscrowDispute moves to DISPUTED
- EscrowResolve finalizes outcome

Invalid transitions MUST reject.

### CHAIN-04: Postage accounting
- PostageTx increments appropriate counters / burns / routes fees as specified

### CHAIN-05: Governance lifecycle
- Proposal submitted
- Votes counted by chamber rules
- Trial started
- Finalized or rolled back based on metrics (in v0.1 metrics evaluation may be “manual oracle”; must be specified)

---

## 12. Fuzzing and adversarial tests (ADV)

### ADV-01: Schema fuzzing
- random CBOR inputs
- corrupted lengths
- unknown keys
Expected: no crashes, safe rejection.

### ADV-02: Resource exhaustion
- oversized messages
- many DHT puts
- pubsub floods
Expected: rate limits and defensive disconnect.

### ADV-03: Sybil simulation
- many nodes from one operator
Expected: peer scoring and postage/bond economics limit damage.

---

## 13. Performance baseline tests (PERF)

Not strict conformance, but required for release readiness:

- handshake latency p50/p95
- pubsub throughput by topic
- DHT query latency
- receipt write throughput
- chain tx submission throughput

---

## Appendix A: How to use the provided test vectors

1) Load `agentnet-test-vectors-v0.1.json`
2) For each vector:
   - decode `*_cbor_hex` to bytes
   - compute sha256 over the bytes
   - verify signature using provided public key
3) Treat the CBOR bytes as goldens; any mismatch fails CE-01.
