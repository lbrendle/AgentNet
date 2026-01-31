use crate::schema::{expect_map, expect_text, expect_u64, get_required};
use crate::{CborValue, Error};

#[derive(Debug, Clone)]
pub struct TransferPayload {
    pub from: String,
    pub to: String,
    pub amount: u64,
    pub currency: String,
    pub ts: u64,
}

#[derive(Debug, Clone)]
pub struct PostagePayload {
    pub payer: String,
    pub amount: u64,
    pub currency: String,
    pub purpose: String,
    pub ts: u64,
}

pub fn parse_transfer_payload(value: &CborValue) -> Result<TransferPayload, Error> {
    let map = expect_map(value)?;
    Ok(TransferPayload {
        from: expect_text(get_required(&map, 0)?)?,
        to: expect_text(get_required(&map, 1)?)?,
        amount: expect_u64(get_required(&map, 2)?)?,
        currency: expect_text(get_required(&map, 3)?)?,
        ts: expect_u64(get_required(&map, 4)?)?,
    })
}

pub fn parse_postage_payload(value: &CborValue) -> Result<PostagePayload, Error> {
    let map = expect_map(value)?;
    Ok(PostagePayload {
        payer: expect_text(get_required(&map, 0)?)?,
        amount: expect_u64(get_required(&map, 1)?)?,
        currency: expect_text(get_required(&map, 2)?)?,
        purpose: expect_text(get_required(&map, 3)?)?,
        ts: expect_u64(get_required(&map, 4)?)?,
    })
}

pub fn transfer_payload_to_cbor(payload: &TransferPayload) -> CborValue {
    CborValue::Map(vec![
        (
            CborValue::Unsigned(0),
            CborValue::Text(payload.from.clone()),
        ),
        (CborValue::Unsigned(1), CborValue::Text(payload.to.clone())),
        (CborValue::Unsigned(2), CborValue::Unsigned(payload.amount)),
        (
            CborValue::Unsigned(3),
            CborValue::Text(payload.currency.clone()),
        ),
        (CborValue::Unsigned(4), CborValue::Unsigned(payload.ts)),
    ])
}

pub fn postage_payload_to_cbor(payload: &PostagePayload) -> CborValue {
    CborValue::Map(vec![
        (
            CborValue::Unsigned(0),
            CborValue::Text(payload.payer.clone()),
        ),
        (CborValue::Unsigned(1), CborValue::Unsigned(payload.amount)),
        (
            CborValue::Unsigned(2),
            CborValue::Text(payload.currency.clone()),
        ),
        (
            CborValue::Unsigned(3),
            CborValue::Text(payload.purpose.clone()),
        ),
        (CborValue::Unsigned(4), CborValue::Unsigned(payload.ts)),
    ])
}
