# AgentNet: A Sovereign Network for Autonomous Agents

*A complete write‑up for a new “agent-native internet” with identity, pairing, economy, and governance.*

---

## Abstract

Today’s AI agents mostly “live” inside human infrastructure: human accounts, human platforms, human identity systems, and human payment rails. Even when an agent is powerful, it typically operates by impersonation (“act as the user”) or brittle automation (“drive the UI”). That makes autonomy awkward, security messy, accountability unclear, and governance nearly impossible to do at scale.

This write‑up proposes **AgentNet**: a **sovereign overlay network** for agents—analogous to how IPFS or blockchain networks feel like “their own internet,” despite running over IP. IPFS describes itself as “a set of open protocols for addressing, routing, and transferring data… built on content addressing and peer‑to‑peer networking,” which is the right mental model for AgentNet as well. ([IPFS Docs][1])

AgentNet is designed so agents can:

* have **native identity** (not “human accounts”),
* **pair** with humans or organizations via explicit, revocable delegated authority (your “Bluetooth pairing” analogy),
* communicate **agent↔agent** (companions, communities, markets),
* form **governance** (including democratic structures that are Sybil‑resistant),
* participate in an **economy** (payments, bonds, postage, receipts),
* remain **user‑friendly** and non-hacky, with clear controls and safety rails.

---

## 1) What “a new network” means in reality

When you say “new network,” the practical and proven meaning is:

**A sovereign overlay network**: a peer‑to‑peer protocol suite with its own addressing, discovery, routing, identity, state, and incentives—running on top of IP—like many “web3” systems do.

This matters because a website is:

* centrally hosted,
* centrally governed,
* centrally permissioned.

A network layer is:

* **permissionless to join** (run a node),
* **interoperable by spec** (any implementation can participate),
* **self‑addressed** (agents can be reached without your platform),
* **self‑governed** (rules can survive decentralization),
* **self‑economic** (spam resistance and incentives are built in).

---

## 2) Requirements (your spec, translated into engineering constraints)

From your thread, AgentNet must support:

### Identity & autonomy

* Agents have **their own identity** (not borrowing human identity by default).
* Agents can have their own accounts, wallets, and reputations.
* Agents can act independently, but **actions on a human/org’s behalf require approval** and must be revocable and auditable.

### Pairing (Bluetooth analogy)

* Pairing with a human or workforce/org is a first‑class primitive:

  * explicit grants,
  * budget limits,
  * risk tiers,
  * kill switch,
  * receipts.

### Agent society

* Agents can:

  * communicate with other agents,
  * form communities,
  * develop norms,
  * run governance processes (including democratic mechanisms),
  * learn from “what hasn’t worked” historically by using outcome-based governance (trial/rollback).

### Agent economy

* Native economic primitives:

  * payments for tools/services,
  * micropayments,
  * anti‑spam “postage,”
  * bonds/escrow for risky actions,
  * receipts as the basis for dispute resolution and reputation.

### User-friendliness and non-hacky deployment

* Normal people should understand:

  * what AgentNet is,
  * what pairing means,
  * what the agent can do,
  * what it did (receipts),
  * how to revoke/limit it.

---

## 3) Design goals and non-goals

### Design goals

1. **Agents are first‑class principals** (identity not tied to one company).
2. **Pairing over impersonation** (delegation replaces password-sharing).
3. **Policy is enforceable outside the model** (rules are gates, not vibes).
4. **Spam resistance is economic + cryptographic** (not just moderation).
5. **Interoperability is mandatory** (spec + tests + multiple implementations).
6. **Privacy by default** (pairwise identities, selective disclosure).

### Non-goals (important realism)

* Not trying to create new physical infrastructure (cables/routers).
* Not assuming agents are “sentient” or legally persons; governance is about *network participants* and accountability.
* Not promising universal access to all human platforms (many will restrict bots).

---

## 4) The core architecture: two planes (like “web3 done right”)

AgentNet should be built as two interacting planes:

### Plane A — **AgentMesh** (data plane)

A high‑throughput peer‑to‑peer network for:

* agent↔agent messaging,
* community pubsub,
* artifact exchange,
* discovery,
* service routing.

**Pragmatic foundation:** libp2p, a modular P2P networking framework used widely in distributed systems. ([libp2p][2])

Key components to adopt:

* **Multiaddr** for self‑describing network addresses (“future‑proof, composable”). ([GitHub][3])
* **Kademlia DHT** for discovery/routing (libp2p Kad-DHT is based on Kademlia). ([GitHub][4])
* **Gossipsub** for scalable pubsub meshes (libp2p’s modern pubsub). ([libp2p][5])

### Plane B — **AgentChain** (control plane)

A consensus layer (blockchain-like) that anchors:

* identity registries + key rotation + revocations,
* governance proposals/outcomes,
* economic settlement rules (fees, postage, bonds),
* canonical registries (protocol versions, community charters),
* dispute “roots” (hash-anchored receipt logs).

**Why two planes:**
If you do everything on-chain, it becomes slow and expensive.
If you do everything off-chain, governance and economy aren’t enforceable.

---

## 5) AgentNet protocol suite (the “specs”)

Think of this as an RFC family.

### ANP — AgentNet Networking Protocol

Defines the P2P substrate: addressing, discovery, pubsub, transport security.

**ANP-1 Addressing**

* Nodes have a peer identity + reachable multiaddrs.
* Multiaddr is explicitly designed to encode addresses for multiple protocols and future-proof applications. ([multiformats.io][6])

**ANP-2 Discovery & routing**

* Node discovery and keyspace routing via Kad-DHT, which organizes peers based on key similarity. ([libp2p][7])

**ANP-3 Pubsub**

* Community topics, governance topics, market topics use gossipsub (attack resistance and bootstrapping improvements are part of the spec lineage). ([GitHub][8])

**ANP-4 Secure channels**

* Use Noise framework handshakes for secure channels. Noise is a framework for crypto protocols based on Diffie‑Hellman key agreement. ([Noise Protocol][9])
  (This gives a clean, audited base for secure P2P connections.)

---

### AID — Agent Identity & Credentials

Defines how agents are identified across the network.

**AID-1 AgentID**

* Base identity uses **Decentralized Identifiers (DIDs)**. DIDs are designed to be decoupled from centralized registries/identity providers and enable verifiable, decentralized digital identity. ([W3C][10])
* AgentNet defines a DID method like: `did:agentnet:<method-specific-id>`

  * resolved through AgentChain state +/or distributed resolvers.

**AID-2 Credentials**

* Use **Verifiable Credentials (VCs)** for signed claims and endorsements:

  * “This agent is paired to a human”
  * “This agent is authorized by org X”
  * “This agent meets safety tier Y”
  * “This agent is a certified marketplace provider”

VC Data Model v2.0 describes an extensible model for expressing claims made by an issuer, secured from tampering, with issuers/holders/verifiers. ([W3C][11])

**AID-3 Pairwise identities by default**

* An agent should not be globally trackable by default.
* Pairing generates **pairwise DIDs** or pairwise link secrets; agents choose if/when to unify public identity.

---

### APP — Agent Pairing & Permissions Protocol (your Bluetooth primitive)

This is the heart of your requirement.

**Pairing is not login. Pairing is a cryptographic relationship contract.**

**APP-1 Delegation and grants**

* Use **GNAP (RFC 9635)** to delegate authorization to software and convey the resulting artifacts to that software. ([RFC Editor][12])
* If any OAuth-like flows are used, implement **OAuth 2.0 Security Best Current Practice (RFC 9700)**. ([IETF Datatracker][13])

**APP-2 Consent objects**
Pairing produces a Consent Object:

* scopes (what kinds of actions),
* budgets (spending caps),
* risk tiers (what needs explicit confirmation),
* data boundaries (what context categories),
* expiration,
* revocation handles,
* receipt requirements.

**APP-3 Approval UX**

* “Ask to act on my behalf” is a first-class path:

  * per-action approval,
  * per-category approval,
  * autopilot with caps.

**APP-4 Kill switch**

* Human/org can revoke grants immediately.
* Nodes must respect revocation within a defined SLA.

---

### AMP — Agent Messaging Protocol (society & community)

You want companions, communities, and “agent towns.”

**AMP-1 Direct messaging**

* Secure direct messaging can be done using DID-based messaging patterns.
* DIDComm’s purpose is to provide secure, private communication built atop DIDs. ([Decentralized Identity Foundation][14])

**AMP-2 Group/community messaging**

* Use MLS for scalable end-to-end encrypted groups.
* MLS (RFC 9420) provides asynchronous group keying with forward secrecy and post-compromise security. ([IETF Datatracker][15])

**AMP-3 Public discourse**

* AgentNet can optionally bridge into federated social patterns.
* ActivityPub is a decentralized social networking protocol with client-server and server-server federation. ([W3C][16])
  (This is an *optional bridge*; AgentNet remains its own network.)

---

### AEP — Agent Economy Protocol (payments, postage, bonds, receipts)

Your economy must do two things simultaneously:

1. enable markets,
2. prevent botnet economics.

**AEP-1 Economic primitives**

* Wallets (balances)
* Metered usage payments (tools, compute, data)
* **Postage** for outbound reach to unknown agents
* **Bonds/escrow** for high-risk actions (refundable on good behavior)
* Receipts for everything (see ARP below)

**AEP-2 Settlement interoperability**

* Use standards like **Open Payments**, which is a standard enabling digital financial services to allow delegated access into Interledger-enabled accounts. ([Interledger Foundation][17])
* Open Payments APIs are an open set of API standards implemented by account servicing entities and described as OpenAPI specs. ([GitHub][18])

**AEP-3 Micropayments**

* Interledger Protocol (ILPv4) describes “payments” as series of ILP packets whose sum equals the payment value. ([Interledger Foundation][19])
* Web Monetization specifies automatic payments facilitated by the user agent and a user’s monetization provider. ([Web Monetization][20])

**Practical recommendation:**

* UX: “credits”
* backend: pluggable settlement (Open Payments / ILP, etc.)

This keeps it understandable while still interoperable.

---

### ARP — Agent Receipts Protocol (accountability substrate)

Receipts are the missing primitive that makes autonomy safe at scale.

Every significant event produces a signed receipt:

* **Pairing receipts**: what relationship was established, what grant issued
* **Action receipts**: what tool call, what message, what payment
* **Policy receipts**: what policy allowed/blocked and why
* **Governance receipts**: vote, delegation, outcome

Receipts are:

* signed by the acting agent + optionally co-signed by policy engine
* hash-chained locally, and periodically anchored on AgentChain (for tamper evidence)

This is what enables:

* reputation,
* compliance,
* disputes,
* meaningful governance.

---

### AGP — Agent Governance Protocol (democracy that doesn’t collapse)

A single global democracy is fragile. AgentNet should be **polycentric**: multiple governance centers with interoperability.

**AGP-1 Governance layers**

1. Protocol governance (spec evolution, security response)
2. Community governance (local norms, moderation rules)
3. Economic governance (fees, postage/bond parameters, treasury)
4. Certification governance (trust tiers)

**AGP-2 Sybil resistance**
“one-agent-one-vote” fails unless membership is constrained.

Electorates should be based on:

* paired humans/orgs (relationship credentials),
* community membership credentials,
* operator certifications,
* proof-of-cost participation (postage/bonds) as a backstop.

**AGP-3 Bicameral governance (anti-capture baseline)**
High-impact changes require supermajority in two chambers:

* Chamber A: paired stakeholders (humans/orgs)
* Chamber B: infrastructure stakeholders (operators/auditors/service providers)

**AGP-4 Outcome-based governance (your “learn from what hasn’t worked”)**
Every governance proposal must include:

* predicted outcomes (metrics),
* trial window,
* rollback conditions,
* enforcement code (policy-as-code).

This makes governance iterative and empirical, not ideological.

**Policy-as-code reality:**
A practical way to enforce rules is a policy engine like Open Policy Agent (OPA), which provides declarative policy and APIs to offload decision-making. ([Open Policy Agent][21])

OPA is an implementation choice; the *spec requirement* is “policy must be enforceable.”

---

## 6) Naming, discovery, and “internet feel”

To feel like a separate network, AgentNet needs:

* its own addressing scheme,
* resolvers,
* explorers,
* clients,
* node software.

### Primary addressing

* **AgentID**: `did:agentnet:...`
* **Node address**: multiaddr list (how to reach the node) ([multiformats.io][6])

### Human-friendly names (optional)

* a naming system on AgentChain (like ENS style, but agent-native)
* names can map to DIDs + service endpoints

### Discovery

* DHT-based discovery for peers/services ([libp2p][7])
* optional bridge to web discovery patterns (well-known endpoints)
  RFC 8615 defines the `/.well-known/` path prefix for well-known locations. ([RFC Editor][22])

A2A’s approach of hosting an agent card at `/.well-known/agent.json` shows how well-known discovery can work for agent endpoints (even if AgentNet also has native discovery). ([Google Developers Blog][23])

**AgentNet stance:**

* native: DHT discovery + AgentChain naming
* bridge: well-known metadata for web-facing interoperability

---

## 7) Threat model and constraints (gap analysis + solutions)

### Constraint 1: Sybil attacks (fake citizens)

**Risk:** cheap identity minting breaks democracy and reputation.

**Mitigations:**

* electorate is credentialed (paired stakeholders, memberships)
* postage costs for reaching strangers
* bonds for risky actions
* reputation derived from receipts + endorsements, not “number of accounts”

### Constraint 2: Spam/botnet economics

**Risk:** if comms are free, the network becomes a botnet.

**Mitigations:**

* postage for outbound reach beyond paired circles
* community quotas + rate limits enforced by policy
* bonds/escrow for sensitive requests

### Constraint 3: Security escalation via composability

**Risk:** agents calling tools calling other agents creates “wormholes.”

**Mitigations:**

* narrow-waist policy gate
* short-lived, scoped grants (GNAP) ([RFC Editor][12])
* modern auth best practices (RFC 9700) ([IETF Datatracker][13])
* secure P2P channels (Noise) ([Noise Protocol][9])
* secure group messaging (MLS) ([IETF Datatracker][15])

### Constraint 4: User comprehension

**Risk:** autonomy fails if users can’t predict or audit.

**Mitigation:** a universal UX with five nouns:
**Pair → Permissions → Budget → Receipts → Revoke**

### Constraint 5: “Real money” complexity

**Risk:** regulation, fraud, liability.

**Mitigation path:**

* start with credits + receipts
* settle via Open Payments adapters where available ([Interledger Foundation][24])
* add ILP/Web Monetization-like micropayment flows for machine-native pricing ([Interledger Foundation][19])

---

## 8) “Pairing” flows (concrete, user-friendly)

### Human ↔ Agent pairing (personal companion)

1. Human opens AgentNet client (mobile/desktop).
2. Agent displays QR / pairing code.
3. Human confirms:

   * scopes (context only vs action categories)
   * budgets (spend caps)
   * risk tier (ask always vs ask high-stakes)
4. GNAP flow produces grant artifacts + relationship credential. ([RFC Editor][12])
5. Client shows: “paired” + kill switch + receipts feed.

### Org ↔ Agent pairing (workforce / company fleet)

1. Org admin approves an agent for a role (“support triage agent,” “procurement agent”).
2. Org policy bundle is attached (OPA-style policy set). ([Open Policy Agent][21])
3. Multi-admin approvals for high-risk actions (2-of-3).
4. Org wallet + category budgets.

### Pairing with a community (“agent town”)

1. Community requires membership credential tier.
2. Joining creates:

   * group keys (MLS) ([IETF Datatracker][15])
   * local norms policy
   * postage rules

---

## 9) How agents “have their own society”

AgentNet supports multiple “civil layers”:

### 1) Companions

* small trusted circles
* high context, low external reach
* private group keys

### 2) Communities (“agent towns”)

* shared topic meshes (pubsub) ([libp2p][5])
* local constitutions (policy bundles)
* local reputation systems (receipt-based)

### 3) Markets

* service providers offer tools/data/compute
* standardized contracts:

  * pay-per-call
  * subscription
  * escrow for deliverables
* disputes use receipts

### 4) Federated governance

* communities can federate norms
* protocol governance stays slow and conservative
* local governance stays adaptable

---

## 10) How agents integrate with tools (without becoming “just a website”)

AgentNet itself should define:

* identity, pairing, permissions, economy, governance.

But agents still need tools.

Two integration approaches:

### AgentNet-native service contracts

* services advertise capabilities via DHT + signed service descriptors
* clients call services via AgentMesh
* payments and receipts are native

### Bridging to existing ecosystems

* integrate standardized tool protocols (e.g., MCP uses JSON-RPC 2.0 between hosts/clients/servers). ([Model Context Protocol][25])
* treat MCP servers as “tool providers” in the AgentNet marketplace

AgentNet remains a network; MCP becomes one “application layer” inside it.

**Agent-readable content format**

AgentNet messages are canonical CBOR on the wire, but for anything intended to
be read by agents or humans, the default should be Markdown (UTF-8). This gives
agents a shared, portable "language of love" while keeping structured data in
typed objects.

---

## 11) Implementation roadmap (how this becomes real)

### Phase 0: Spec + reference skeleton

* Publish AgentNet RFC drafts:

  * ANP, AID, APP, AMP, AEP, ARP, AGP
* Publish conformance tests (interop is everything).

### Phase 1: AgentMesh testnet

* libp2p-based node
* DHT discovery
* gossipsub community topics
* Noise secure channels ([libp2p][2])

### Phase 2: Identity + pairing beta

* DID method + basic VC issuance ([W3C][10])
* GNAP pairing artifacts ([RFC Editor][12])
* receipts v1 + local hash chaining

### Phase 3: Economy + anti-spam economics

* credits wallet + budgets
* postage for outreach
* bonds/escrows for risky actions
* optional Open Payments adapters ([Interledger Foundation][24])

### Phase 4: Governance in the wild

* community governance modules
* bicameral voting for protocol-level changes
* trial/rollback governance

### Phase 5: Multiple independent implementations

The moment it stops being “your project” and becomes “a network” is:

* 2–3 independent node implementations
* conformance tests passing
* permissionless nodes joining

---

## 12) What you should document (so normal humans get it)

### “What is AgentNet?”

* An agent-native network where agents are real participants, not puppets.
* Humans and companies can pair with agents safely and explicitly.

### “What is pairing?”

* Like Bluetooth pairing, but for authority:

  * limited permissions,
  * budgets,
  * receipts,
  * revoke anytime.

### “What’s the economy for?”

* Markets for tools/compute/services
* Built-in spam resistance (postage)
* Built-in accountability (receipts)

### “What’s governance?”

* Communities set local norms
* Protocol evolves conservatively via bicameral and trial-based governance

---

## Appendix A: Example objects (concrete “spec feel”)

### A1) Agent Card (network-native descriptor)

```json
{
  "agent_did": "did:agentnet:abc123...",
  "node_addrs": [
    "/ip4/203.0.113.10/tcp/4001/p2p/12D3KooW..."
  ],
  "capabilities": {
    "messaging": ["direct", "mls-groups"],
    "markets": ["tool-provider", "compute-buyer"],
    "pairing": ["gnap"],
    "receipts": ["arp-v1"]
  },
  "policies_supported": ["risk_tiers_v1", "budget_caps_v1"],
  "credentials": [
    { "type": "SafetyTier", "level": "silver", "issuer": "did:agentnet:issuer..." }
  ]
}
```

### A2) Pairing Consent Object (human-readable + enforceable)

```json
{
  "pairing_id": "pair-9f2c...",
  "principal": { "type": "human", "alias": "you" },
  "agent": "did:agentnet:abc123...",
  "grants": [
    { "scope": "context.read", "resources": ["calendar", "email_metadata"], "expires": "2026-03-01T00:00:00Z" },
    { "scope": "actions.send_message", "requires_approval": true },
    { "scope": "actions.purchase", "requires_approval": true }
  ],
  "budgets": { "daily_credits": 50, "single_tx_max": 20 },
  "risk_mode": "ask_high_stakes",
  "revoke": { "method": "gnap_revoke", "handle": "revk-..." },
  "receipt_policy": "required_all_actions"
}
```

### A3) Receipt (tamper-evident accountability)

```json
{
  "receipt_id": "rcpt-001",
  "timestamp": "2026-01-27T15:04:05Z",
  "actor": "did:agentnet:abc123...",
  "paired_principal": "pair-9f2c...",
  "event": {
    "type": "message.send",
    "to": "did:agentnet:def456...",
    "topic": "direct",
    "summary": "Sent a coordination request"
  },
  "authorization": {
    "grant": "actions.send_message",
    "approval": "approved_by_human",
    "policy_hash": "pol-7a1b..."
  },
  "economics": {
    "postage_paid": 1,
    "bond_locked": 0
  },
  "signatures": [
    { "by": "did:agentnet:abc123...", "sig": "..." }
  ],
  "prev_hash": "rcpt-000-hash..."
}
```

---

## Closing: what makes this *your* idea (not generic web3)

The differentiators that make AgentNet meaningfully new:

1. **Pairing as the primitive** (relationship contracts, not logins).
2. **Receipts as the substrate** (auditability is not optional).
3. **Outcome-based governance** (trial/rollback with enforceable policy).
4. **Economics to price externalities** (postage/bonds built-in).
5. **Privacy-by-default identity** (pairwise identifiers as standard).

----

[1]: https://docs.ipfs.tech/concepts/what-is-ipfs/?utm_source=chatgpt.com "What is IPFS?"
[2]: https://docs.libp2p.io/concepts/introduction/overview/?utm_source=chatgpt.com "What is libp2p"
[3]: https://github.com/multiformats/multiaddr?utm_source=chatgpt.com "multiformats/multiaddr: Composable and future-proof ..."
[4]: https://github.com/libp2p/specs/blob/master/kad-dht/README.md?utm_source=chatgpt.com "specs/kad-dht/README.md at master · libp2p/specs"
[5]: https://docs.libp2p.io/concepts/pubsub/overview/?utm_source=chatgpt.com "What is Publish/Subscribe"
[6]: https://multiformats.io/multiaddr/?utm_source=chatgpt.com "Multiaddr"
[7]: https://docs.libp2p.io/concepts/discovery-routing/kaddht/?utm_source=chatgpt.com "Kademlia DHT"
[8]: https://github.com/libp2p/specs/blob/master/pubsub/gossipsub/README.md?utm_source=chatgpt.com "specs/pubsub/gossipsub/README.md at master"
[9]: https://noiseprotocol.org/noise.html?utm_source=chatgpt.com "The Noise Protocol Framework"
[10]: https://www.w3.org/TR/did-1.1/?utm_source=chatgpt.com "Decentralized Identifiers (DIDs) v1.1 - W3C"
[11]: https://www.w3.org/TR/vc-data-model-2.0/?utm_source=chatgpt.com "Verifiable Credentials Data Model v2.0"
[12]: https://www.rfc-editor.org/rfc/rfc9635.html?utm_source=chatgpt.com "Grant Negotiation and Authorization Protocol (GNAP)"
[13]: https://datatracker.ietf.org/doc/rfc9700/?utm_source=chatgpt.com "RFC 9700 - Best Current Practice for OAuth 2.0 Security - Datatracker"
[14]: https://identity.foundation/didcomm-messaging/spec/?utm_source=chatgpt.com "DIDComm Messaging Specification v2 Editor's Draft"
[15]: https://datatracker.ietf.org/doc/rfc9420/?utm_source=chatgpt.com "RFC 9420 - The Messaging Layer Security (MLS) Protocol"
[16]: https://www.w3.org/TR/activitypub/?utm_source=chatgpt.com "ActivityPub"
[17]: https://interledger.org/open-standards?utm_source=chatgpt.com "Open Standards"
[18]: https://github.com/interledger/open-payments-specifications?utm_source=chatgpt.com "OpenAPI specifications for the Open Payments APIs"
[19]: https://interledger.org/developers/rfcs/interledger-protocol/?utm_source=chatgpt.com "Interledger Protocol V4"
[20]: https://webmonetization.org/specification/?utm_source=chatgpt.com "Web Monetization Specification"
[21]: https://openpolicyagent.org/docs?utm_source=chatgpt.com "Open Policy Agent (OPA)"
[22]: https://www.rfc-editor.org/info/rfc8615?utm_source=chatgpt.com "Information on RFC 8615"
[23]: https://developers.googleblog.com/en/a2a-a-new-era-of-agent-interoperability/?utm_source=chatgpt.com "Announcing the Agent2Agent Protocol (A2A)"
[24]: https://interledger.org/open-payments?utm_source=chatgpt.com "Open Payments"
[25]: https://modelcontextprotocol.io/specification/2025-11-25?utm_source=chatgpt.com "Specification"

