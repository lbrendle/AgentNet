use crate::schema::{parse_tx_envelope_payload, TxEnvelopePayload};
use crate::signed::{split_signed_map, with_signature};
use crate::{
    decode_canonical, encode_canonical, sha256, sign_ed25519_hash, verify_ed25519_hash, CborValue,
    Error,
};

#[derive(Debug, Clone)]
pub struct TxEnvelope {
    pub payload: TxEnvelopePayload,
    pub signature: Vec<u8>,
}

pub fn parse_tx_envelope(value: &CborValue) -> Result<TxEnvelope, Error> {
    let (payload_entries, signature) = split_signed_map(value, 5)?;
    let payload_value = CborValue::Map(payload_entries);
    let payload = parse_tx_envelope_payload(&payload_value)?;
    Ok(TxEnvelope { payload, signature })
}

pub fn decode_tx_envelope(data: &[u8]) -> Result<TxEnvelope, Error> {
    let value = decode_canonical(data)?;
    parse_tx_envelope(&value)
}

pub fn build_tx_envelope(payload: &TxEnvelopePayload, secret_key: &[u8]) -> Result<Vec<u8>, Error> {
    let payload_value = tx_envelope_payload_to_cbor(payload);
    let payload_cbor = encode_canonical(&payload_value)?;
    let hash = sha256(&payload_cbor);
    let sig = sign_ed25519_hash(secret_key, &hash)?;
    let full = with_signature(&payload_value, 5, sig)?;
    encode_canonical(&full)
}

pub fn verify_tx_envelope(data: &[u8], public_key: &[u8]) -> Result<TxEnvelopePayload, Error> {
    let value = decode_canonical(data)?;
    let (payload_entries, signature) = split_signed_map(&value, 5)?;
    let payload_value = CborValue::Map(payload_entries);
    let payload_cbor = encode_canonical(&payload_value)?;
    let hash = sha256(&payload_cbor);
    verify_ed25519_hash(public_key, &hash, &signature)?;
    parse_tx_envelope_payload(&payload_value)
}

pub fn tx_envelope_payload_to_cbor(payload: &TxEnvelopePayload) -> CborValue {
    CborValue::Map(vec![
        (CborValue::Unsigned(0), CborValue::Unsigned(payload.tx_type)),
        (
            CborValue::Unsigned(1),
            CborValue::Text(payload.sender.clone()),
        ),
        (CborValue::Unsigned(2), CborValue::Unsigned(payload.nonce)),
        (CborValue::Unsigned(3), CborValue::Unsigned(payload.fee)),
        (CborValue::Unsigned(4), payload.payload.clone()),
    ])
}
