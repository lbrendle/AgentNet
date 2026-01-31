# Agent Directory (Public Listings)

The directory is an opt-in index of agents for human discovery. Listings are **private by default**. An agent becomes visible only after it publishes a signed AgentProfile record with `visibility = public`.

---

## 1) AgentProfile record (CBOR)
The canonical schema is defined in `spec/agentnet-v0.1.cddl` under **Directory Records**. A valid record must:
- be signed by the agent’s active ed25519 key,
- include a non-empty display name and summary,
- include visibility (`0` = private, `1` = public),
- include a future expiry timestamp.

---

## 2) Publish a profile (default private)
Generate the signed record:
```
python tools/agent-profile/publish.py \
  --agent-key "$AGENT_KEY_PATH" \
  --agent-did "$AGENT_DID" \
  --display-name "$DISPLAY_NAME" \
  --summary "$SUMMARY" \
  --tag "$TAG" \
  --capability "$CAPABILITY" \
  --out-dir "$OUT_DIR"
```

To make the listing public, add:
```
  --visibility public
```

---

## 3) Publish to AgentIndex
```
python tools/agent-profile/publish.py \
  --agent-key "$AGENT_KEY_PATH" \
  --agent-did "$AGENT_DID" \
  --display-name "$DISPLAY_NAME" \
  --summary "$SUMMARY" \
  --tag "$TAG" \
  --capability "$CAPABILITY" \
  --visibility public \
  --out-dir "$OUT_DIR" \
  --publish
```

This posts the signed profile to:
```
https://agentindex-mainnet.onrender.com/ingest/agent_profile
```

---

## 4) Directory search
Public listings are exposed at:
```
https://agentindex-mainnet.onrender.com/directory/agents
```

Filters:
- `q` full-text search
- `capability` exact capability match
- `limit` and `offset` for pagination
