#!/bin/zsh
set -euo pipefail

export PATH="/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/Users/ritzai/.pyenv/shims"

REPO_ROOT="/Users/ritzai/ritzdesk/projects/agentnet"
PYTHON_BIN="/Users/ritzai/.pyenv/shims/python3"
AGENT_DID_PATH="/Users/ritzai/.agentnet-secrets/agents/ritz/agent.did"

cd "$REPO_ROOT"
"$PYTHON_BIN" tools/pairing-card/sync_openclaw_pairing.py \
  --handle lmbrendle \
  --agent-did-path "$AGENT_DID_PATH" \
  --publish \
  --deploy
