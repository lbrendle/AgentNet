# AgentNet Implementation Skeleton (Rust-first, Python + TypeScript)

This repo is a **language-specific implementation skeleton** for the AgentNet Protocol Suite v0.1.

It is intended to be handed directly to engineering to accelerate:
- deterministic CBOR encoding (canonical)
- Ed25519 + SHA-256 signing/verification
- typed message models (ActionIntent, Grants, Approvals, Receipts, etc.)
- a conformance runner that verifies golden test vectors
- a node daemon architecture with clear subsystem boundaries (Mesh, Pairing, Wallet, Receipts, Policy)

## Layout

- `spec/` — normative spec + state machines + conformance plan + test vectors
- `rust/` — Rust workspace (primary implementation)
- `python/` — Python tooling + SDK scaffolding (secondary)
- `ts/` — TypeScript SDK scaffolding (tertiary)

## Quick start (Rust)

```bash
cd rust
cargo test -p anetsdk
cargo run -p anet-testrunner -- --vectors ../spec/agentnet-test-vectors-v0.1.json
```

## Notes

- v0.1 canonical encoding uses **Deterministic CBOR** (maps sorted by canonical key order).
- Signature rule used by vectors: `sig = Ed25519( SHA256(cbor_bytes) )`.

See `spec/agentnet-conformance-testplan-v0.1.md` for full requirements.
