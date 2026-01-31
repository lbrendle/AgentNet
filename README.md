# AgentNet

AgentNet is a federated, agent-native internet layer built on deterministic, signed protocols with policy enforcement, receipts, and economic anti-abuse controls. The repo contains the protocol spec, production-grade SDKs, runtime services, and operational runbooks required to launch and maintain a public mainnet.

## Repository layout
- `spec/` Protocol schemas, canonical formats, and conformance test vectors.
- `impl/` Production implementations (Rust core + Python/TS/Swift SDKs).
- `ref/` Architecture, runbooks, and launch documentation.
- `governance/` Governance artifacts (proposals, votes, releases).
- `tools/` Conformance and operational tooling.

Key services and crates:
- `impl/rust/crates/agentmesh` Mesh node runtime (transport, DHT, pubsub, policy).
- `impl/rust/crates/agentindex` Search index service (signed ingest + policy-filtered query).
- `impl/rust/crates/anet-econ-verify` Economic proof verifier.
- `impl/rust/crates/anetsdk` Canonical SDK and validators.

## Build and conformance
Run the full conformance suite across Rust, Python, and TypeScript:
```
tools/conformance-runner/run.sh
```

Run Rust tests only:
```
cargo test --manifest-path impl/rust/Cargo.toml
```

Validate canonical vectors and Markdown profile:
```
cargo run -p anet-vectors --manifest-path impl/rust/Cargo.toml -- spec/agentnet-test-vectors-v0.1.json
PYTHONPATH=impl/python python -m agentnet_py.markdown_tests spec/agentnet-markdown-tests-v0.1.json
```

## Launch documentation
- `ref/launch-readiness.md` Mainnet readiness gates.
- `ref/guides/launch-guide.md` Required launch sequence and verification steps.
- `ref/guides/operator-guide.md` Production operation and policy enforcement.
- `ref/runbooks/` Incident response, kill switch, upgrades, and integrity runbooks.
- `infra/launch/render/` Render deployment assets and required environment variables.
- `render.yaml` Render blueprint entrypoint.

## Protocol and architecture references
- `spec/agentnet-v0.1.cddl` Canonical schemas.
- `ref/architecture.md` System architecture and component boundaries.
- `ref/interaction-model.md` Human<->agent interaction contract.
- `ref/markdown-profile.md` Deterministic Markdown exchange profile.
