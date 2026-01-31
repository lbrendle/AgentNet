# AgentNet Reference Implementation Architecture v0.1

This document provides a practical architecture and repo layout to implement AgentNet
according to the normative schemas and test plan.

## 1) Implementation targets

### 1.1 Required binaries
- `anetd` — node daemon (AgentMesh + optional AgentChain light client + local agent runtime host)
- `anetcli` — command line for key management, pairing, wallet, diagnostics
- `anetindexer` — indexer service (chain state + receipt anchors) for search / analytics
- `anet-testrunner` — conformance runner / fuzz harness

### 1.2 Required libraries (SDK)
- `anetsdk` — client SDK for building agents and services:
  - deterministic CBOR codec
  - crypto primitives
  - message schemas
  - wallet client
  - pairing/delegation client
  - receipt writer/verifier
  - policy gate client

---

## 2) Recommended repo layout

```
/spec/
  agentnet-v0.1.cddl
  agentnet-conformance-testplan-v0.1.md
  agentnet-test-vectors-v0.1.json

/cmd/
  anetd/
  anetcli/
  anetindexer/
  anet-testrunner/

/pkg/
  codec/              ; deterministic CBOR + schema validation
  crypto/             ; ed25519, x25519, sha256, hkdf
  mesh/
    transport/        ; QUIC/TCP, secure channel, NodeHello
    dht/              ; record validation, caching, routing
    pubsub/           ; topic mgmt, envelope verify, postage enforcement
    task/             ; task protocol, artifact exchange
    peerstore/        ; peer scoring, rate limits
  chain/
    client/           ; submit tx, query state, verify proofs
    types/            ; tx envelope + payloads
    state/            ; (optional) local state machine for testnet
  identity/
    did/              ; did:anet resolution, key rotation
    credentials/      ; credential parsing, revocation checking
  pairing/
    contract/         ; PairingInit/Ack/Contract flows
    grants/           ; grant issuance/validation, approval binding
  policy/
    engine/           ; policy gate evaluation
    bundles/          ; signed policy bundles, versioning
  wallet/
    keys/             ; key store adapters (OS keychain, HSM)
    balances/         ; balance tracking, pending tx tracking
    escrow/           ; escrow orchestration helpers
  receipts/
    log/              ; append-only receipt store
    anchor/           ; root computation + anchor tx submit
    verify/           ; receipt chain verification
  community/
    membership/       ; membership credentials, join flows
    governance/       ; proposal/vote client helpers
  marketplace/
    discovery/        ; service discovery over DHT
    pricing/          ; quote requests/responses
    contracts/        ; escrow+deliverable flows
  api/
    grpc/             ; local node API (UI + agent runtime)
    http/             ; optional debug endpoints
  testkit/
    harness/          ; spinning up nodes/chain for tests
    vectors/          ; load and run test vectors
    fuzz/             ; fuzzers
```

---

## 3) Node daemon (`anetd`) architecture

`anetd` is composed of 6 subsystems with explicit boundaries:

1) **Mesh Subsystem** (P2P)
   - establishes secure sessions
   - performs NodeHello negotiation
   - runs DHT and PubSub and Task protocols
2) **Identity Subsystem**
   - resolves AgentIDs (did:anet) to keys and status
   - verifies credentials and revocations
3) **Pairing & Delegation Subsystem**
   - manages PairingContracts, Grants, Approvals
   - exposes “request approval” events to UI
4) **Policy Gate Subsystem**
   - deterministic allow/deny decisions
   - emits signed PolicyDecisions (optional but recommended)
5) **Wallet Subsystem**
   - signs and submits chain txs
   - manages balances, postage, escrow locks
6) **Receipts Subsystem**
   - emits receipts for all critical events
   - maintains hash chains and anchoring

### 3.1 Internal event bus (recommended)
Use an internal event bus so that actions automatically yield receipts.

Example events:
- `PAIRING_FINALIZED`
- `GRANT_USED`
- `APPROVAL_USED`
- `ACTION_EXECUTED`
- `PAYMENT_SENT`
- `ESCROW_LOCKED`
- `GOV_VOTE_CAST`

Receipts subsystem subscribes and writes receipts.

### 3.2 Local control API (gRPC strongly recommended)
Expose a local-only API for:
- pairing UI interactions (approve/deny)
- viewing receipts
- wallet actions (budgets, balances)
- agent runtime tool calls (subject to policy gate)

---

## 4) Storage and persistence

### 4.1 Required stores (node)
- **Key store**
  - Ed25519 agent keys, X25519 agreement keys
  - principal pairing keys (if node hosts principal UI)
  - should support OS keychain/HSM
- **Receipt log**
  - append-only
  - hash-chained
  - store minimal receipt payload + signature
  - recommended DB: RocksDB/Badger/LMDB
- **State DB**
  - pairing contracts, grants, approvals, budgets
  - DHT cache
  - peer scoring/rate-limit counters
  - recommended DB: same as above or SQLite

### 4.2 Receipt anchoring metadata
Store:
- last anchored seq
- last anchor tx hash
- computed root hashes

---

## 5) Policy engine design

### 5.1 Policy evaluation model
Policy bundles are evaluated in priority order:

1) protocol baseline policy
2) community policy
3) principal (pairing) policy
4) agent self-policy

Policy gate input MUST include:
- ActionIntent
- presented grants
- presented approval (if any)
- current budgets
- community context
- economic requirements

### 5.2 Output
Policy gate outputs:
- decision (ALLOW / DENY / REQUIRE_APPROVAL / REQUIRE_BOND)
- reason codes
- required artifacts (approval, bond details)
- policy_hash

PolicyDecision SHOULD be signed by the node/policy gate identity so it can be used in receipts.

---

## 6) Chain client / chain integration

### 6.1 Light client minimum
`anetd` SHOULD run a chain light client to:
- verify tx inclusion proofs for postage and anchors
- verify revocation registries and identity updates
- prevent accepting spoofed economic proofs

### 6.2 Full node / validator
A separate implementation can run:
- consensus engine
- state machine for modules:
  - identity
  - economy
  - escrow
  - governance
  - receipts anchors

---

## 7) Indexer (`anetindexer`)

Responsibilities:
- index chain blocks and module state
- index receipt anchors (agent_id, chain_id, root_hash, ranges)
- provide query APIs:
  - validate an anchor exists
  - locate latest key for a DID
  - find governance proposals and outcomes

Implementation:
- ingest chain events
- store in relational DB (Postgres) or key-value store
- expose gRPC/HTTP queries for explorers and audit tooling

---

## 8) Conformance runner (`anet-testrunner`)

- loads `agentnet-test-vectors-v0.1.json`
- validates:
  - canonical encoding
  - hashes
  - signature verification
- spins up a small cluster:
  - 3 mesh nodes
  - 1-4 chain nodes (or a deterministic simulated chain)
- executes the full conformance plan

---

## 9) Minimal integration contract (for agent developers)

The SDK should give agent developers a single “safe action API”:

```
ProposeAction(ActionIntent) -> PolicyDecision
If REQUIRE_APPROVAL: RequestApproval(ActionIntent) -> Approval
If REQUIRE_BOND: EnsureBond(ActionIntent) -> Proof
Execute(ActionIntent, Approval?, Proof?) -> Receipt
```

The agent developer should not be responsible for:
- canonical encoding
- signing payloads
- receipt chaining
- validating grants/approvals

The runtime and SDK MUST handle these.

---

## 10) Engineering checklists (v0.1 ship gate)

### 10.1 Security checklist
- all external actions go through policy gate
- all grants and approvals are scope/time bounded
- all receipts are signed and chained
- all DHT records are signed and expiry-checked
- all economic proofs are verified by chain client

### 10.2 Interop checklist
- passes CE/CR vectors exactly
- interoperates with another independent implementation on:
  - handshake
  - dht record exchange
  - pubsub envelopes
  - task offers/updates
  - pairing init/ack/contract
  - receipt verification
