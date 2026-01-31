#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

run() {
  echo "==> $*"
  "$@"
}

run cargo test -p anetsdk --manifest-path "$ROOT/impl/rust/Cargo.toml"
run cargo test -p agentmesh --manifest-path "$ROOT/impl/rust/Cargo.toml"
run cargo test -p agentindex --manifest-path "$ROOT/impl/rust/Cargo.toml"
run cargo run -p anet-vectors --manifest-path "$ROOT/impl/rust/Cargo.toml" -- "$ROOT/spec/agentnet-test-vectors-v0.1.json"

run env PYTHONPATH="$ROOT/impl/python" python -m agentnet_py.vectors "$ROOT/spec/agentnet-test-vectors-v0.1.json"
run env PYTHONPATH="$ROOT/impl/python" python -m agentnet_py.markdown_tests "$ROOT/spec/agentnet-markdown-tests-v0.1.json"

run npm --prefix "$ROOT/impl/ts" install
run npm --prefix "$ROOT/impl/ts" run build
run node "$ROOT/impl/ts/dist/vectors.js" "$ROOT/spec/agentnet-test-vectors-v0.1.json"
run node "$ROOT/impl/ts/dist/markdown_tests.js" "$ROOT/spec/agentnet-markdown-tests-v0.1.json"
