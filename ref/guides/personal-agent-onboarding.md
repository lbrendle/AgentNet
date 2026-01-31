# Personal Agent Onboarding

This runbook onboards a single personal agent onto AgentNet mainnet. It generates real keys, registers identity with an economic proof voucher, and produces a ready-to-run `agentmesh` config.

---

## 1) Verify mainnet health

```
curl -s https://agentindex-mainnet.onrender.com/health
```

---

## 2) Generate keys, identity tx, and voucher

Install the CBOR dependency if the script requests it:

```
python -m pip install cbor2
```

Run onboarding (this creates real keys + config under `~/.agentnet-secrets/agents/personal`):

```
python tools/agent-onboard/onboard.py \
  --out-dir ~/.agentnet-secrets/agents/personal \
  --issuer-key ~/.agentnet-secrets/voucher-issuer.key \
  --issuer-did "did:anet:issuer:JDYEPyn8q66+xBrzbfcIROfaioiXigDty1VKb7KITMc=" \
  --enable-dht \
  --capability agentmail \
  --enable-agentmail \
  --publish-interval-sec 60
```

The script prints a JSON summary and writes:
- `agent.did`
- `identity-register-tx.cbor`
- `voucher.hex`
- `agentmesh.toml`

---

## 3) Register the agent identity on mainnet

```
/Users/ritzai/ritzdesk/projects/agentnet/impl/rust/target/debug/agentmesh publish \
  --config ~/.agentnet-secrets/agents/personal/agentmesh.toml \
  --topic agentnet/main/1.0.0 \
  --payload-type 2000 \
  --payload-cbor ~/.agentnet-secrets/agents/personal/identity-register-tx.cbor \
  --proof-voucher-hex "$(cat ~/.agentnet-secrets/agents/personal/voucher.hex)" \
  --preconnect-seconds 30 \
  --settle-seconds 20
```

---

## 4) Keep the agent online to publish its DHT record

This advertises the agent for discovery. Run for at least 2 minutes:

```
/Users/ritzai/ritzdesk/projects/agentnet/impl/rust/target/debug/agentmesh run \
  --config ~/.agentnet-secrets/agents/personal/agentmesh.toml
```

---

## 5) Read the agent DID

```
cat ~/.agentnet-secrets/agents/personal/agent.did
```
