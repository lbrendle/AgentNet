Below is a **deep, engineering‑grade implementation guide** for AgentNet v0.1 that matches the artifacts you already have (spec CDDL, state machines, conformance plan, test vectors, reference architecture) and the Rust/Python/TS skeleton you downloaded.

I’m going to treat this like a “how to actually build the network” manual, not a conceptual overview.

---

# 1) Repo orientation and “what is authoritative”

### Authoritative spec (normative)

Inside your zip:

* `spec/agentnet-v0.1.cddl`
  **Normative on‑wire schemas** using integer-keyed deterministic CBOR.

* `spec/agentnet-state-machines-v0.1.md`
  **Normative state machines** for Pairing, Delegation, Escrow, Governance.

* `spec/agentnet-conformance-testplan-v0.1.md`
  **Normative conformance** criteria.

* `spec/agentnet-test-vectors-v0.1.json`
  **Golden vectors** (CBOR bytes + sha256 + Ed25519 signatures).

* `spec/agentnet-reference-architecture-v0.1.md`
  **Reference implementation structure**.

### Implementation skeleton (non-normative, but aligned)

* `rust/` workspace with:

  * `anetsdk` (codec/crypto/types + subsystem stubs)
  * `anet-testrunner` (vector verifier CLI)
  * `anetd` (node daemon skeleton)
  * `anetcli` (CLI helpers)

* `python/agentnet_py` (tools + vector verifier)

* `ts/agentnet-ts` (SDK scaffolding + vector verifier)

**Rule of thumb:**

* If it’s in `spec/*.cddl` / state machines → treat it as ground truth.
* If it’s in skeleton code → treat it as guidance; align it to CDDL as you implement.

---

# 2) Absolute foundation: canonical encoding + hashing + signing

If your canonical encoding is inconsistent across clients, you won’t have a network—just incompatible apps.

## 2.1 Canonical encoding rule (v0.1)

**Canonical encoding = Deterministic CBOR** with **restricted types**.

Enforce at least:

* **No floats**
* **No tags**
* **No indefinite-length items**
* **Maps are sorted by canonical key ordering**:
  sort by **encoded key length**, then **lexicographic** on the encoded bytes.

The Rust encoder in `anetsdk/src/codec/cbor.rs` already implements this restriction and canonical map ordering.

### Map key strategy (why integer keys)

v0.1 schemas use integer keys (0,1,2…) because:

* key ordering becomes deterministic and unambiguous
* avoids string key locale/encoding ambiguity
* smaller payloads
* easy cross-language stability

**Implementation note:**
In TypeScript, don’t use plain JS objects for canonical maps—use `Map` and explicitly sort encoded keys (the skeleton does this).

## 2.2 Hashing rule

Per your vectors:

* `hash = sha256(cbor_bytes)`

This is the interop “truth.” In Rust: `anetsdk::crypto::hash::sha256`.

## 2.3 Signature rule (v0.1)

Per your vectors:

* `sig = Ed25519( sha256(cbor_bytes) )`

This is slightly unusual (many systems sign raw bytes), but it’s fine as long as it’s consistent and domain-separated later.

**Important:** if you change this to sign raw bytes, you’ll break all vectors and cross-language interop.

### Future hardening (v0.2+)

Once v0.1 interop is proven, add **domain separation** to prevent cross-protocol signature reuse, e.g.:

* `sig = Ed25519( sha256("ANET|v0.2|Receipt|" || cbor_bytes) )`

But do **not** change this in v0.1 until after you’ve frozen test vectors and shipped at least two independent implementations.

## 2.4 Your very first “go/no-go” test

Run vectors in all three languages.

**Rust**

```bash
cd rust
cargo test -p anetsdk
cargo run -p anet-testrunner -- --vectors ../spec/agentnet-test-vectors-v0.1.json
```

**Python**

```bash
cd python
pip install -e .
agentnet-vectors ../spec/agentnet-test-vectors-v0.1.json
```

**TS**

```bash
cd ts
npm install
npm run build
npm run test:vectors
```

If any vector fails, do not build networking yet—fix canonical encoding first.

---

# 3) AgentMesh: building the data plane (P2P network)

AgentMesh is what makes this “a network” instead of a website. It has:

* transport + secure sessions
* NodeHello handshake
* DHT discovery / record semantics
* PubSub (communities, governance feeds, markets)
* direct messaging + task protocol

## 3.1 Transport and session model

### Minimal baseline for v0.1

* QUIC preferred, TCP fallback.
* Secure channel with mutual authentication (NodeKey).

**In the skeleton**:
`rust/crates/anetsdk/src/mesh/transport.rs` defines `Transport`, `Session`, `Stream` traits.

### Framing recommendation (make this explicit)

Don’t rely on “read until EOF.” Define a simple frame format for protocol streams:

* `u32_be length` + `length bytes payload`

Payload bytes are canonical CBOR for message objects.

This keeps parsing safe and avoids ambiguous message boundaries.

### Peer identity (NodeID)

* NodeID should be derived from NodeKey (hash public key bytes).
* Cache peer metadata in a peerstore:

  * observed addresses
  * roles (relay/validator)
  * feature bits
  * last-seen timestamps
  * score / penalty counters

### NAT + relays (reality)

You need relays early or devnet adoption will stall:

* implement relay nodes as a role
* incentivize later via AgentChain (v0.2+ economics)

## 3.2 NodeHello handshake

After secure channel is established:

1. both sides send `NodeHello`

2. validate:

   * protocol compatibility
   * chain_id match
   * max_msg_bytes
   * time drift bound (recommend ±120s in v0.1)

3. store in peerstore

**Vector coverage:** `TV4_NodeHello` exists—use it as canonical structure.

**Engineering rule:**
If a node cannot validate NodeHello deterministically, it must disconnect.

### Feature negotiation pattern

Implement:

* intersection of supported protocols
* optional feature toggles:

  * receipt anchoring
  * postage proof types supported
  * compression (future)
  * streaming extensions (future)

Don’t “auto-enable” features without negotiation—this becomes your compatibility layer.

## 3.3 DHT: discovery and record semantics

You need discovery because:

* agents and services can’t rely on centralized directories.

### What the DHT stores (v0.1)

The DHT should store signed records, at least:

* AgentRecord (agent reachable endpoints + capabilities)
* ServiceRecord (provider capabilities, endpoints, pricing hints)
* CommunityRecord (join policy, postage rules, governance pointers)

### Record validation (must be strict)

For any incoming record:

* check record size bounds
* verify expiry (TTL)
* verify signature
* verify that signer is authorized to publish that record

  * AgentRecord must be signed by agent key
  * ServiceRecord must be signed by provider agent
  * CommunityRecord must be signed by community authority

**Poisoning resistance:**
Refuse to store invalid records; rate-limit PUTs by peer; penalize peers sending invalid records.

### Deterministic “record key” derivation

Use a namespace + key material, then hash:

* `k = sha256(namespace || key_material)`

Example:

* namespace: `"agent_record"`
* key_material: `agent_id`

This makes keys stable across implementations.

### Replication and expiry

Start simple:

* DHT node stores records locally with expiry time
* periodic sweep deletes expired records
* replication factor (K) tuned later

---

# 4) PubSub: communities, markets, governance feeds

PubSub is how you get:

* agent community “town squares”
* governance broadcasts
* market listings/requests

## 4.1 Topic structure

Use structured topic naming so clients can safely subscribe:

* `/anet/<chain_id>/community/<community_id>/v1`
* `/anet/<chain_id>/governance/global/v1`
* `/anet/<chain_id>/market/<category>/v1`

Keep version in the topic path so incompatible changes don’t silently intermix.

## 4.2 PubSubEnvelope (required)

Every PubSub message MUST:

* include sender agent ID
* include timestamp + sequence
* include payload type
* include postage proof if required
* be signed (v0.1 rule: Ed25519 over sha256(cbor_bytes))

Implement:

* replay protections: `(sender, seq)` monotonic within a time window
* time drift checks
* size caps

## 4.3 Postage enforcement (anti-spam economics)

Your network will be unlivable without priced externalities.

Mechanism:

* communities define postage rules (rate limits and fees)
* relaying nodes verify postage proof before propagating messages

v0.1 simplest proof:

* on-chain tx reference (hash) that paid postage
* envelope includes `{ postage_proof: { tx_hash } }`

Later optimization (v0.2+):

* off-chain vouchers signed by postage providers, slashable if abused

---

# 5) Direct messaging + Task protocol

Two categories:

1. **Direct messaging**: point-to-point, private
2. **Task protocol**: structured negotiation + artifacts + payments

## 5.1 Direct messaging

Do not try to reinvent Signal in v0.1.

Minimal:

* direct message is a signed envelope
* content can be plaintext at first *on devnet* (debug)
* for production: encrypt payload with X25519-derived shared secret

Practical implementation:

* derive shared secret (X25519)
* encrypt with XChaCha20-Poly1305
* include nonce + sender key id

## 5.2 Task protocol states

Use the state machine in `spec/agentnet-state-machines-v0.1.md`:

* PROPOSED → ACCEPTED → IN_PROGRESS → DELIVERED|FAILED → CLOSED

**Receipt emission requirement:**
Every state transition emits a receipt.

## 5.3 Artifacts

Artifacts must be content-addressed:

* `artifact_id = sha256(bytes)` (or merkle root for large content)

For v0.1 you can:

* send small artifacts inline
* store large artifacts via a storage provider (marketplace)
* reference by content hash + retrieval multiaddr

---

# 6) AgentChain: control plane (identity, money, governance)

AgentChain is where you anchor:

* identity registry + rotations + revocations
* balances and transfers
* postage + escrow + slashing
* governance proposals/votes/outcomes
* receipt anchoring

## 6.1 TxEnvelope structure (per CDDL)

Your chain tx format is defined in `spec/agentnet-v0.1.cddl` under `TxEnvelope`.

Implement exactly:

* integer-keyed CBOR map for payload
* signature over sha256(cbor(payload_without_sig))

### Anti-replay

Every sender account must have:

* monotonic `nonce`
* tx rejected if nonce not exactly `expected_nonce`

## 6.2 Required chain modules (v0.1)

Minimum viable modules to support your value prop:

### Accounts

* balances
* transfer tx

### Fees

* base fee schedule
* min fee enforcement

### Identity registry

* register AgentID → pubkeys
* rotate keys
* revoke credentials / keys

### Postage

* accept postage payments
* emit postage receipts or store tx hash reference for proofs

### Escrow

* lock funds
* release on success
* dispute window
* resolve/slash

### Receipt anchoring

* store receipt root hashes (agent submits periodic root)
* enables tamper evidence without on-chain bloat

### Governance

* proposal lifecycle
* voting rules
* finalize, trial, rollback hooks

## 6.3 Escrow mechanics (core economic primitive)

**EscrowLock** tx:

* locks `amount` from sender
* binds to `escrow_id`
* includes release condition reference (usually receipt-based)

**EscrowRelease** tx:

* releases locked funds to provider
* requires proof of deliverable (receipt hash or task deliverable hash)

**Dispute** tx:

* freezes escrow
* opens dispute window
* triggers governance/arbitration path

**Resolve** tx:

* releases to one side
* may slash bonds of malicious actors

**Vector coverage:** `TV6_EscrowLockTx` exists.

---

# 7) Identity: AgentID + credentials + revocation

## 7.1 AgentID registry (did:anet)

In v0.1, keep it simple:

* AgentID maps to a set of public keys
* registration is on-chain
* DID resolution uses chain state + local caching

The Rust skeleton provides `identity/did.rs` with a `DidResolver` trait.

## 7.2 Key rotation

Key rotation is not optional in production.

Minimum:

* on-chain rotation tx updates the active key set
* old keys may remain valid for verifying old receipts (audit)
* new actions require latest key

## 7.3 Credentials and membership

Credentials are signed claims:

* PairingCredential (agent paired to principal)
* MembershipCredential (agent in community)
* CertificationCredential (trust tier)

In v0.1:

* credentials can be CBOR objects signed by issuer keys
* revocation is via on-chain registry or status list anchored on chain

---

# 8) Pairing + delegation: the “Bluetooth for authority” primitive

This is what makes AgentNet safe and usable. Without it, “autonomy” becomes “the agent has your passwords.”

## 8.1 Pairing Contract (required)

Pairing creates a cryptographic relationship:

* principal_id
* agent_id
* created/expires
* default risk mode
* receipt mode
* revocation method
* signatures of both parties

This is the *relationship root*.

## 8.2 Pairing flows

Implement as a 2-phase process:

### Phase A: out-of-band confirmation

* QR code / device code / signed invite
* ensures principal and agent are intentionally pairing

### Phase B: on-network finalization

* exchange PairingInit and PairingAck messages
* negotiate parameters
* both sign PairingContract
* optionally anchor contract hash on chain (tamper evidence)

## 8.3 Grants (capability tokens)

A Grant authorizes action classes under constraints:

* scopes
* TTL
* budgets
* allowlists/denylists
* approval-required scope rules
* revocation reference

A Grant must be:

* signed by principal authority
* time bounded
* revocable

## 8.4 Approvals (instance authorization)

Approvals are per-action permissions that bind to an **ActionIntent hash**.

This prevents:

* approval reuse on a different action
* tampering with action details after approval

### Replay prevention for approvals

Approvals MUST include:

* `intent_hash`
* expiry
* optionally a monotonic approval counter

Approvals MUST be rejected if:

* expired
* already consumed (store used approval ids)
* grant revoked

---

# 9) Policy Gate: deterministic enforcement outside the model

Your policy gate is the “narrow waist” that prevents:

* prompt injection turning into tool misuse
* permission creep
* accidental spend/data leakage

## 9.1 What the policy gate must decide

Given:

* ActionIntent
* grants
* optional approval
* budgets + spend state
* community rules

Return:

* ALLOW
* DENY
* REQUIRE_APPROVAL
* REQUIRE_BOND

…and always provide:

* reason codes (machine + human readable)
* policy hash (for receipts)

## 9.2 Rule composition (priority)

When policies conflict, deterministic priority order matters:

1. protocol baseline
2. community policy
3. principal policy (pairing)
4. agent self-policy

Never allow lower-priority policy to override higher-priority denies.

## 9.3 Implementation approaches

### v0.1 recommended

Implement a deterministic evaluator in Rust:

* compile rules into a pure function
* keep it auditable and testable
* avoid dynamic “AI decides policy” behavior

### v0.2+ optional

Use OPA/rego or a policy DSL:

* compile policy bundles to a deterministic execution engine
* sign policy bundles and version them

---

# 10) Receipts: the accountability substrate

Receipts are what make this “autonomy with trust” instead of “autonomy with vibes.”

## 10.1 Receipt log requirements

A receipt log MUST be:

* append-only
* hash-chained (`prev_hash`)
* signed by acting agent
* optionally partitioned per pairing/community for privacy

The Rust skeleton provides `receipts/log.rs` trait.

## 10.2 Receipt anchoring

To make tampering evident without bloating the chain:

* periodically compute a root over a receipt range (hash chain head is sufficient; Merkle root optional)
* submit `ReceiptAnchorTx` to chain
* auditors can verify that local receipts match anchored root

**Best practice:** anchor on time interval (e.g., every N minutes) or on count interval (every M receipts).

## 10.3 What must generate receipts (minimum)

* pairing finalize
* grant issuance
* approval issuance/consumption
* policy decisions
* external tool calls
* payments (postage, escrow, transfers)
* task state transitions
* governance votes and outcomes

---

# 11) Marketplace/service discovery + economy

Your marketplace is how “agent economy” becomes real:

* tool providers
* compute/storage providers
* indexing providers
* arbitration providers

## 11.1 Service discovery

Use:

* DHT ServiceRecords for discovery
* optional chain registry for high-trust providers

## 11.2 Contracting pattern (recommended)

1. buyer sends TaskOffer with payment contract
2. provider accepts
3. buyer locks escrow on chain
4. provider delivers artifact
5. buyer releases escrow (or auto-release on receipt proof)
6. receipts emitted at each stage

## 11.3 Bonds and slashing (anti-abuse)

Require bonds for:

* high-volume outreach
* high-value contracts
* high-risk actions (as policy decides)

Slashing triggers:

* proven fraud (dispute resolution)
* repeated spam policy violations (community governance)

---

# 12) Governance: protocol + communities

You want democracy-like governance, but it must survive:

* Sybil attacks
* capture
* incentive manipulation

## 12.1 Governance domains

* protocol governance (upgrades, baseline policies, economics params)
* community governance (membership, norms, local postage rules)
* certification governance (trust tiers, validators)

## 12.2 Proposal lifecycle

Use a strict state machine:

* DRAFT → SUBMITTED → VOTING → APPROVED/REJECTED → TRIAL → FINALIZED/ROLLED_BACK

## 12.3 Trial + rollback (your “learn from what hasn’t worked” requirement)

Require proposals to include:

* predicted outcomes (metrics)
* evaluation window
* rollback conditions
* policy changes as machine-readable bundles

**Reality note:** metric collection needs an oracle strategy. For v0.1:

* use receipt-derived metrics (spam reports, dispute counts, completion rates)
* anchor metric aggregates on chain for transparency

---

# 13) Conformance: how you make it a network, not an ecosystem of forks

Conformance is the real “layer” test.

## 13.1 Minimum conformance gates

1. canonical CBOR byte-for-byte match for all golden vectors
2. signature verification match
3. handshake interoperability (NodeHello)
4. DHT record validation and expiry behavior match
5. pubsub envelope verification + postage rules
6. pairing/grant/approval validation + revocation SLA
7. receipt chain integrity + anchor tx validation
8. escrow lock/release/dispute state machine correctness
9. governance proposal/vote/finalize correctness

## 13.2 How to expand vectors (very important)

Right now you have 6 vectors:

* ActionIntent
* Approval
* Grant
* NodeHello
* ReceiptChain
* EscrowLockTx

Next vectors you should add (recommended):

* PairingContract finalized object (signed by both)
* ReceiptAnchorTx
* PostageTx + PubSubEnvelope with postage proof
* Governance proposal/vote/finalize txs
* Credential revocation tx

Each vector should include:

* debug JSON view
* canonical CBOR hex
* sha256 hex
* signature hex

---

# 14) Engineering execution plan: build in vertical slices

This is the fastest way to get a real testnet.

## Slice 1 — “Interop core”

* canonical CBOR encoder in Rust/Py/TS
* vector runner green in all languages
* NodeHello encoding/decoding + vector match

Deliverable: **3 languages pass vectors**.

## Slice 2 — “Mesh bootstrap”

* implement a libp2p transport in Rust (`Transport` trait)
* secure session + handshake exchange
* peerstore persistence (simple sqlite/rocksdb)

Deliverable: **two nodes connect, handshake, exchange NodeHello**.

## Slice 3 — “Discovery”

* DHT integration + record validator
* AgentRecord publish + lookup

Deliverable: **agent endpoint discoverable without central server**.

## Slice 4 — “Pairing + receipts”

* Pairing contract creation flow
* Grant + Approval validation
* Receipt log append + hash chain + signature

Deliverable: **agent can only act with grant/approval; receipts prove it**.

## Slice 5 — “Economy”

* minimal chain client (even if mock chain first)
* escrow lock tx + release tx
* postage rules for pubsub

Deliverable: **contracts + anti-spam economics become real**.

## Slice 6 — “Governance”

* proposal/vote/finalize state machine
* trial/rollback enforcement for parameters

Deliverable: **network parameters can evolve without central admin**.

---

# 15) Practical engineering choices (Rust-first)

## 15.1 Recommended Rust crates (when you go from skeleton to real)

* **libp2p** for networking (QUIC, gossipsub, kad)
* **ed25519-dalek** (already used)
* **x25519-dalek**, **chacha20poly1305**, **hkdf**
* **rocksdb** or **sled** for peerstore / DHT cache
* **sqlite** for receipts (append-only log)
* **tracing** for structured logs (already in anetd)

## 15.2 Storage recommendations

* DHT/cache: RocksDB (fast random reads/writes)
* receipts: SQLite (append-only, queryable; easy for audits)
* keys: OS keychain / HSM later; for devnet use encrypted file

## 15.3 Time

Time is always messy. For v0.1:

* allow drift tolerance (±120s)
* stamp receipts with local time
* rely on chain block timestamps for settlement windows

---

# 16) Security checklist you should enforce before public testnet

* strict canonical encoding (no “best effort” decoders)
* strict signature verification (no “lenient modes”)
* replay defenses for:

  * pubsub seq
  * approval consumption
  * chain tx nonces
* rate limits everywhere:

  * DHT PUT/GET
  * handshake attempts
  * pubsub propagation
* postage/bonds turned on for any “unknown to unknown” communication
* kill switch / revocation SLA enforced (tests required)
* receipt log integrity check tool (verify hash chain + signature)

---

## If you want, I can generate one more artifact

If you want this guide as a **single engineering doc** (like `ENGINEERING-GUIDE.md`) plus:

* `justfile`/`Makefile`
* a local devnet docker-compose plan (3 mesh nodes + mock chain + indexer)
* and the next 10 golden vectors pack

…say the word and I’ll output it in the same “handoff quality” format.
