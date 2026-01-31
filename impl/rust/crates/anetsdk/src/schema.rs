use crate::{CborValue, Error};

#[derive(Debug, Clone)]
pub struct ActionIntent {
    pub intent_id: String,
    pub actor: String,
    pub pairing_id: String,
    pub action_type: String,
    pub target: CborValue,
    pub max_cost: u64,
    pub currency: String,
    pub reason: String,
    pub context_refs: Vec<String>,
    pub ts: u64,
}

#[derive(Debug, Clone)]
pub struct Approval {
    pub approval_id: String,
    pub issuer: String,
    pub intent_hash: Vec<u8>,
    pub exp: u64,
}

#[derive(Debug, Clone)]
pub struct Grant {
    pub grant_id: String,
    pub issuer: String,
    pub subject: String,
    pub pairing_id: String,
    pub scopes: Vec<String>,
    pub constraints: CborValue,
    pub revocation_ref: CborValue,
    pub exp: u64,
}

#[derive(Debug, Clone)]
pub struct NodeHello {
    pub protocols: Vec<String>,
    pub chain_id: String,
    pub node_id: String,
    pub node_pubkey: Vec<u8>,
    pub roles: Vec<String>,
    pub features: CborValue,
    pub time: u64,
    pub nonce: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct ReceiptPayload {
    pub receipt_id: String,
    pub ts: u64,
    pub actor: String,
    pub pairing_id: Option<String>,
    pub community_id: Option<String>,
    pub event: CborValue,
    pub auth: CborValue,
    pub economics: CborValue,
    pub prev_hash: Vec<u8>,
    pub seq: u64,
}

#[derive(Debug, Clone)]
pub struct TxEnvelopePayload {
    pub tx_type: u64,
    pub sender: String,
    pub nonce: u64,
    pub fee: u64,
    pub payload: CborValue,
}

pub fn parse_action_intent(value: &CborValue) -> Result<ActionIntent, Error> {
    let map = expect_map(value)?;
    Ok(ActionIntent {
        intent_id: expect_text(get_required(&map, 0)?)?,
        actor: expect_text(get_required(&map, 1)?)?,
        pairing_id: expect_text(get_required(&map, 2)?)?,
        action_type: expect_text(get_required(&map, 3)?)?,
        target: get_required(&map, 4)?.clone(),
        max_cost: expect_u64(get_required(&map, 5)?)?,
        currency: expect_text(get_required(&map, 6)?)?,
        reason: expect_text(get_required(&map, 7)?)?,
        context_refs: expect_text_array(get_required(&map, 8)?)?,
        ts: expect_u64(get_required(&map, 9)?)?,
    })
}

pub fn parse_approval_payload(value: &CborValue) -> Result<Approval, Error> {
    let map = expect_map(value)?;
    Ok(Approval {
        approval_id: expect_text(get_required(&map, 0)?)?,
        issuer: expect_text(get_required(&map, 1)?)?,
        intent_hash: expect_bytes_len(get_required(&map, 2)?, 32)?,
        exp: expect_u64(get_required(&map, 3)?)?,
    })
}

pub fn parse_grant_payload(value: &CborValue) -> Result<Grant, Error> {
    let map = expect_map(value)?;
    Ok(Grant {
        grant_id: expect_text(get_required(&map, 0)?)?,
        issuer: expect_text(get_required(&map, 1)?)?,
        subject: expect_text(get_required(&map, 2)?)?,
        pairing_id: expect_text(get_required(&map, 3)?)?,
        scopes: expect_text_array(get_required(&map, 4)?)?,
        constraints: get_required(&map, 5)?.clone(),
        revocation_ref: get_required(&map, 6)?.clone(),
        exp: expect_u64(get_required(&map, 7)?)?,
    })
}

pub fn parse_nodehello_payload(value: &CborValue) -> Result<NodeHello, Error> {
    let map = expect_map(value)?;
    Ok(NodeHello {
        protocols: expect_text_array(get_required(&map, 0)?)?,
        chain_id: expect_text(get_required(&map, 1)?)?,
        node_id: expect_text(get_required(&map, 2)?)?,
        node_pubkey: expect_bytes_len(get_required(&map, 3)?, 32)?,
        roles: expect_text_array(get_required(&map, 4)?)?,
        features: get_required(&map, 5)?.clone(),
        time: expect_u64(get_required(&map, 6)?)?,
        nonce: expect_bytes_len(get_required(&map, 7)?, 16)?,
    })
}

pub fn parse_receipt_payload(value: &CborValue) -> Result<ReceiptPayload, Error> {
    let map = expect_map(value)?;
    Ok(ReceiptPayload {
        receipt_id: expect_text(get_required(&map, 0)?)?,
        ts: expect_u64(get_required(&map, 1)?)?,
        actor: expect_text(get_required(&map, 2)?)?,
        pairing_id: expect_optional_text(get_required(&map, 3)?)?,
        community_id: expect_optional_text(get_required(&map, 4)?)?,
        event: get_required(&map, 5)?.clone(),
        auth: get_required(&map, 6)?.clone(),
        economics: get_required(&map, 7)?.clone(),
        prev_hash: expect_bytes_len(get_required(&map, 8)?, 32)?,
        seq: expect_u64(get_required(&map, 9)?)?,
    })
}

pub fn parse_tx_envelope_payload(value: &CborValue) -> Result<TxEnvelopePayload, Error> {
    let map = expect_map(value)?;
    Ok(TxEnvelopePayload {
        tx_type: expect_u64(get_required(&map, 0)?)?,
        sender: expect_text(get_required(&map, 1)?)?,
        nonce: expect_u64(get_required(&map, 2)?)?,
        fee: expect_u64(get_required(&map, 3)?)?,
        payload: get_required(&map, 4)?.clone(),
    })
}

pub(crate) fn expect_map(value: &CborValue) -> Result<Vec<(CborValue, CborValue)>, Error> {
    match value {
        CborValue::Map(entries) => Ok(entries.clone()),
        _ => Err(Error::Cbor("expected map")),
    }
}

pub(crate) fn get_required(entries: &[(CborValue, CborValue)], key: u64) -> Result<&CborValue, Error> {
    for (k, v) in entries {
        if let CborValue::Unsigned(n) = k {
            if *n == key {
                return Ok(v);
            }
        }
    }
    Err(Error::Cbor("missing required key"))
}

pub(crate) fn get_optional(entries: &[(CborValue, CborValue)], key: u64) -> Option<&CborValue> {
    for (k, v) in entries {
        if let CborValue::Unsigned(n) = k {
            if *n == key {
                return Some(v);
            }
        }
    }
    None
}

pub(crate) fn expect_u64(value: &CborValue) -> Result<u64, Error> {
    match value {
        CborValue::Unsigned(n) => Ok(*n),
        _ => Err(Error::Cbor("expected unsigned")),
    }
}

pub(crate) fn expect_u16(value: &CborValue) -> Result<u16, Error> {
    match value {
        CborValue::Unsigned(n) if *n <= u16::MAX as u64 => Ok(*n as u16),
        _ => Err(Error::Cbor("expected u16")),
    }
}

pub(crate) fn expect_u8(value: &CborValue) -> Result<u8, Error> {
    match value {
        CborValue::Unsigned(n) if *n <= u8::MAX as u64 => Ok(*n as u8),
        _ => Err(Error::Cbor("expected u8")),
    }
}

pub(crate) fn expect_text(value: &CborValue) -> Result<String, Error> {
    match value {
        CborValue::Text(s) => Ok(s.clone()),
        _ => Err(Error::Cbor("expected text")),
    }
}

pub(crate) fn expect_optional_text(value: &CborValue) -> Result<Option<String>, Error> {
    match value {
        CborValue::Text(s) => Ok(Some(s.clone())),
        CborValue::Null => Ok(None),
        _ => Err(Error::Cbor("expected text or null")),
    }
}

pub(crate) fn expect_bytes(value: &CborValue) -> Result<Vec<u8>, Error> {
    match value {
        CborValue::Bytes(b) => Ok(b.clone()),
        _ => Err(Error::Cbor("expected bytes")),
    }
}

pub(crate) fn expect_bytes_len(value: &CborValue, len: usize) -> Result<Vec<u8>, Error> {
    let bytes = expect_bytes(value)?;
    if bytes.len() != len {
        return Err(Error::Cbor("invalid length"));
    }
    Ok(bytes)
}

pub(crate) fn expect_text_array(value: &CborValue) -> Result<Vec<String>, Error> {
    match value {
        CborValue::Array(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                out.push(expect_text(item)?);
            }
            Ok(out)
        }
        _ => Err(Error::Cbor("expected array of text")),
    }
}
