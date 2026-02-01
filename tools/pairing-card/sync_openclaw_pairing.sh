#!/bin/zsh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
PYTHON_BIN="${PYTHON_BIN:-python3}"

HANDLE="${HANDLE:-}"
MODE="${MODE:-pairing}"
AGENT_HANDLE="${AGENT_HANDLE:-}"
PAIRED_HANDLE="${PAIRED_HANDLE:-}"
AGENT_DID_PATH="${AGENT_DID_PATH:-}"
PUBLISH="${PUBLISH:-1}"
DEPLOY="${DEPLOY:-1}"

if [[ -z "$HANDLE" || -z "$AGENT_DID_PATH" ]]; then
  echo "usage: HANDLE=<handle> AGENT_DID_PATH=<path> [MODE=pairing|agent] [AGENT_HANDLE=<handle>] [PAIRED_HANDLE=<handle>] $0" >&2
  exit 1
fi

cd "$REPO_ROOT"
cmd=("$PYTHON_BIN" tools/pairing-card/sync_openclaw_pairing.py --handle "$HANDLE" --mode "$MODE" --agent-did-path "$AGENT_DID_PATH")
if [[ -n "$AGENT_HANDLE" ]]; then
  cmd+=(--agent-handle "$AGENT_HANDLE")
fi
if [[ -n "$PAIRED_HANDLE" ]]; then
  cmd+=(--paired-handle "$PAIRED_HANDLE")
fi
if [[ "$PUBLISH" == "1" ]]; then
  cmd+=(--publish)
fi
if [[ "$DEPLOY" == "1" ]]; then
  cmd+=(--deploy)
fi

"${cmd[@]}"
