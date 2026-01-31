# Economic Proof Verification

This document defines the required validation for economic proofs used as postage or escrow evidence.

---

## 1) Proof types

- **On-chain transaction proofs**: must be verified against the configured chain.
- **Voucher proofs**: must be verified against approved issuer keys and strict policy checks.

The verifier must fail closed if the configured proof type is unsupported or misconfigured.

---

## 2) Voucher envelope (CBOR)

Voucher payloads are canonical CBOR maps with the following fields:

- `0`: issuer (did)
- `1`: payer (did)
- `2`: amount (u64)
- `3`: currency (tstr)
- `4`: purpose (id64)
- `5`: timestamp (unix_time)
- `6`: expiry (unix_time)
- `7`: nonce (nonce16)
- `8`: signature (sig64) over the canonical payload map of fields `0..7`

Verification requirements:

- Issuer must be in the configured issuer registry.
- Signature must verify against issuer public key.
- Payer must match the message sender.
- Purpose must match policy (topic-based by default).
- Amount > 0, currency non-empty.
- Timestamp and expiry must be within allowed clock skew.
- Nonce must be unique and stored persistently to prevent replay.

---

## 3) Verifier configuration requirements

The verifier must be configured with:

- Issuer registry (DID + ed25519 public key).
- Nonce state storage path.
- Max clock skew window.
- Purpose policy (topic match and/or allowed purpose list).

On-chain verification requires:

- Chain RPC endpoint and chain identifier.
- Minimum confirmation depth and success status requirement.
- Optional sender/recipient address constraints.
- Optional maximum transaction age threshold.
- Confirmation thresholds and finality rules.

---

## 4) Failure handling

- Any parsing, signature, replay, or policy error must reject the proof.
- All rejections must be surfaced to policy receipts.
