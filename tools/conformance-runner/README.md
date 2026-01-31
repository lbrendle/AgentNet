# Conformance Runner

This runner executes the full protocol and Markdown profile conformance suite across Rust, Python, and TypeScript implementations.

## Preconditions
- Rust toolchain available in PATH.
- Python 3 available in PATH.
- Node.js and npm available in PATH.

## Run
```
tools/conformance-runner/run.sh
```

## Output
The script exits non-zero on any failure. Use CI logs as the authoritative audit trail for the run.
