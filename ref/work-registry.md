# Work Registry (Offers and Agreements)

This document defines registry transactions for publishing and managing work offers and agreements.

---

## 1) Transaction types

- `50` WorkOfferPublish
- `51` WorkAgreementPublish
- `52` WorkAgreementUpdate
- `53` WorkAgreementClose

Tx payloads are canonical CBOR maps carried inside `TxEnvelope.payload`.

---

## 2) WorkOfferPublish payload

CBOR map:

- `0` offer (bytes) — canonical, signed WorkOffer bytes.
- `1` ts (u64) — publish timestamp.

Rules:
- Offer must verify with sender’s registered public key.
- Offer `issuer` MUST equal `TxEnvelope.sender`.
- Offer must not already exist.
- Timestamp must be within the configured clock skew window.

---

## 3) WorkAgreementPublish payload

CBOR map:

- `0` agreement (bytes) — canonical, signed WorkAgreement bytes.
- `1` ts (u64) — publish timestamp.

Rules:
- Agreement must verify with sender’s registered public key.
- Agreement `issuer` MUST equal `TxEnvelope.sender`.
- Agreement must not already exist.
- Agreement must reference an existing offer.
- Timestamp must be within the configured clock skew window.

---

## 4) WorkAgreementUpdate payload

CBOR map:

- `0` agreement_id (tstr)
- `1` prev_agreement_hash (bytes[32])
- `2` agreement (bytes)
- `3` ts (u64)

Rules:
- Agreement must exist and not be closed.
- Sender MUST match the registered issuer.
- `prev_agreement_hash` must match current registry record.
- Agreement must verify with sender key and keep the same `agreement_id`.
- Timestamp must be within the configured clock skew window.

---

## 5) WorkAgreementClose payload

CBOR map:

- `0` agreement_id (tstr)
- `1` agreement_hash (bytes[32])
- `2` reason (tstr)
- `3` ts (u64)

Rules:
- Agreement must exist and not be closed.
- Sender MUST be issuer or counterparty.
- `agreement_hash` must match current registry record.
- Timestamp must be within the configured clock skew window.

---

## 6) Receipt details

Receipts emit event type `EV_WORK_REGISTRY` with details including:
- event string (`work.offer.publish`, `work.agreement.publish`, `work.agreement.update`, `work.agreement.close`)
- offer_id or agreement_id
- actor (issuer or closing party)
- agreement/offer hash
- reason (for closes)
