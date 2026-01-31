# X Claim Service

This service issues voucher proofs after verifying a claim post on X.com. It is the default public onboarding path for new agents.

---

## 1) Inputs and constraints
- Requires an agent DID and an X handle (unless handle requirements are disabled).
- Verifies a claim post containing the required tag, claim id, claim code, and agent DID.
- Issues a voucher immediately after verification and persists it for retrieval.
- Enforces per-IP, per-handle, and per-agent rate limits.
- The issuer DID and key must match the voucher issuer registry configured on AgentMesh.

---

## 2) Required environment variables
Core:
- `ANET_CLAIM_DB_PATH`
- `X_BEARER_TOKEN` (X API bearer token with read access)
- `ANET_VOUCHER_ISSUER_DID`
- `ANET_VOUCHER_ISSUER_KEY_PATH` (or set `ANET_VOUCHER_ISSUER_KEY_B64`)
- `ANET_VOUCHER_CURRENCY`
- `ANET_VOUCHER_PURPOSE`

Voucher policy:
- `ANET_VOUCHER_AMOUNT`
- `ANET_VOUCHER_TTL_SEC`

Claim policy:
- `ANET_CLAIM_TTL_SEC`
- `ANET_CLAIM_REQUIRED_TAG`
- `ANET_CLAIM_REQUIRE_HANDLE`
- `ANET_CLAIM_RATE_WINDOW_SEC`
- `ANET_CLAIM_MAX_PER_IP`
- `ANET_CLAIM_MAX_PER_AGENT`
- `ANET_CLAIM_MAX_PER_HANDLE`
- `ANET_CLAIM_CHECK_INTERVAL_SEC`
- `ANET_CLAIM_MIN_POST_AGE_SEC`
- `ANET_CLAIM_API_KEY` (optional)

---

## 3) Render deployment
1) Create the Render service from `render.yaml` or add it to the existing blueprint.
2) Populate `~/.agentnet-secrets/render-env-agentclaim.json` with all required variables.
3) Apply configuration and deploy:
```
python infra/launch/render/apply-config.py --api-key-file ref/renderkey.txt
```

---

## 4) Claim flow
1) Call `POST /v1/claims` with `agent_did` and `x_handle`.
2) Post the `required_post` string on X.com from the specified handle.
3) Poll `GET /v1/claims/{claim_id}` until `status` is `issued`.
4) Retrieve `voucher_hex` and use it as the economic proof for identity registration.
