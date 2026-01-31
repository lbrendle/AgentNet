use crate::schema::{
    expect_bytes_len, expect_map, expect_text, expect_u64, expect_u8, get_optional, get_required,
};
use crate::{CborValue, Error};

#[derive(Debug, Clone)]
pub struct EscrowLockPayload {
    pub escrow_id: String,
    pub payer: String,
    pub payee: String,
    pub amount: u64,
    pub currency: String,
    pub release_condition: CborValue,
    pub dispute_window_sec: u64,
    pub expiry: u64,
}

#[derive(Debug, Clone)]
pub struct EscrowReleasePayload {
    pub escrow_id: String,
    pub evidence_receipt_hash: Vec<u8>,
    pub ts: u64,
}

#[derive(Debug, Clone)]
pub struct EscrowDisputePayload {
    pub escrow_id: String,
    pub reason: String,
    pub evidence_anchor_or_receipt: Vec<u8>,
    pub ts: u64,
}

#[derive(Debug, Clone)]
pub struct EscrowResolvePayload {
    pub escrow_id: String,
    pub outcome: u8,
    pub split_amount_to_payee: Option<u64>,
    pub ts: u64,
}

pub fn parse_escrow_lock_payload(value: &CborValue) -> Result<EscrowLockPayload, Error> {
    let map = expect_map(value)?;
    Ok(EscrowLockPayload {
        escrow_id: expect_text(get_required(&map, 0)?)?,
        payer: expect_text(get_required(&map, 1)?)?,
        payee: expect_text(get_required(&map, 2)?)?,
        amount: expect_u64(get_required(&map, 3)?)?,
        currency: expect_text(get_required(&map, 4)?)?,
        release_condition: get_required(&map, 5)?.clone(),
        dispute_window_sec: expect_u64(get_required(&map, 6)?)?,
        expiry: expect_u64(get_required(&map, 7)?)?,
    })
}

pub fn parse_escrow_release_payload(value: &CborValue) -> Result<EscrowReleasePayload, Error> {
    let map = expect_map(value)?;
    Ok(EscrowReleasePayload {
        escrow_id: expect_text(get_required(&map, 0)?)?,
        evidence_receipt_hash: expect_bytes_len(get_required(&map, 1)?, 32)?,
        ts: expect_u64(get_required(&map, 2)?)?,
    })
}

pub fn parse_escrow_dispute_payload(value: &CborValue) -> Result<EscrowDisputePayload, Error> {
    let map = expect_map(value)?;
    Ok(EscrowDisputePayload {
        escrow_id: expect_text(get_required(&map, 0)?)?,
        reason: expect_text(get_required(&map, 1)?)?,
        evidence_anchor_or_receipt: expect_bytes_len(get_required(&map, 2)?, 32)?,
        ts: expect_u64(get_required(&map, 3)?)?,
    })
}

pub fn parse_escrow_resolve_payload(value: &CborValue) -> Result<EscrowResolvePayload, Error> {
    let map = expect_map(value)?;
    let split_amount_to_payee = match get_optional(&map, 2) {
        Some(value) => Some(expect_u64(value)?),
        None => None,
    };
    Ok(EscrowResolvePayload {
        escrow_id: expect_text(get_required(&map, 0)?)?,
        outcome: expect_u8(get_required(&map, 1)?)?,
        split_amount_to_payee,
        ts: expect_u64(get_required(&map, 3)?)?,
    })
}

pub fn escrow_lock_payload_to_cbor(payload: &EscrowLockPayload) -> CborValue {
    CborValue::Map(vec![
        (
            CborValue::Unsigned(0),
            CborValue::Text(payload.escrow_id.clone()),
        ),
        (
            CborValue::Unsigned(1),
            CborValue::Text(payload.payer.clone()),
        ),
        (
            CborValue::Unsigned(2),
            CborValue::Text(payload.payee.clone()),
        ),
        (CborValue::Unsigned(3), CborValue::Unsigned(payload.amount)),
        (
            CborValue::Unsigned(4),
            CborValue::Text(payload.currency.clone()),
        ),
        (CborValue::Unsigned(5), payload.release_condition.clone()),
        (
            CborValue::Unsigned(6),
            CborValue::Unsigned(payload.dispute_window_sec),
        ),
        (CborValue::Unsigned(7), CborValue::Unsigned(payload.expiry)),
    ])
}

pub fn escrow_release_payload_to_cbor(payload: &EscrowReleasePayload) -> CborValue {
    CborValue::Map(vec![
        (
            CborValue::Unsigned(0),
            CborValue::Text(payload.escrow_id.clone()),
        ),
        (
            CborValue::Unsigned(1),
            CborValue::Bytes(payload.evidence_receipt_hash.clone()),
        ),
        (CborValue::Unsigned(2), CborValue::Unsigned(payload.ts)),
    ])
}

pub fn escrow_dispute_payload_to_cbor(payload: &EscrowDisputePayload) -> CborValue {
    CborValue::Map(vec![
        (
            CborValue::Unsigned(0),
            CborValue::Text(payload.escrow_id.clone()),
        ),
        (
            CborValue::Unsigned(1),
            CborValue::Text(payload.reason.clone()),
        ),
        (
            CborValue::Unsigned(2),
            CborValue::Bytes(payload.evidence_anchor_or_receipt.clone()),
        ),
        (CborValue::Unsigned(3), CborValue::Unsigned(payload.ts)),
    ])
}

pub fn escrow_resolve_payload_to_cbor(payload: &EscrowResolvePayload) -> CborValue {
    let mut entries = vec![
        (
            CborValue::Unsigned(0),
            CborValue::Text(payload.escrow_id.clone()),
        ),
        (
            CborValue::Unsigned(1),
            CborValue::Unsigned(payload.outcome as u64),
        ),
        (CborValue::Unsigned(3), CborValue::Unsigned(payload.ts)),
    ];
    if let Some(amount) = payload.split_amount_to_payee {
        entries.push((CborValue::Unsigned(2), CborValue::Unsigned(amount)));
    }
    CborValue::Map(entries)
}
