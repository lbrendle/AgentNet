use crate::schema::{
    expect_bytes, expect_bytes_len, expect_map, expect_text, expect_u16, expect_u64, expect_u8,
    get_optional, get_required,
};
use crate::signed::{split_signed_map, with_signature};
use crate::{
    decode_canonical, encode_canonical, sha256, sign_ed25519_hash, verify_ed25519_hash, CborValue,
    Error,
};

#[derive(Debug, Clone)]
pub enum EconomicProof {
    OnChainTx { tx_hash: Vec<u8> },
    Voucher { voucher: Vec<u8> },
}

#[derive(Debug, Clone)]
pub struct PubSubEnvelopePayload {
    pub version: u8,
    pub topic: String,
    pub sender: String,
    pub ts: u64,
    pub seq: u64,
    pub payload_type: u16,
    pub payload: CborValue,
    pub economic_proof: Option<EconomicProof>,
}

#[derive(Debug, Clone)]
pub struct PubSubEnvelope {
    pub payload: PubSubEnvelopePayload,
    pub signature: Vec<u8>,
}

pub fn parse_pubsub_payload(value: &CborValue) -> Result<PubSubEnvelopePayload, Error> {
    let map = expect_map(value)?;
    let economic_proof = match get_optional(&map, 7) {
        Some(value) => Some(parse_economic_proof(value)?),
        None => None,
    };
    Ok(PubSubEnvelopePayload {
        version: expect_u8(get_required(&map, 0)?)?,
        topic: expect_text(get_required(&map, 1)?)?,
        sender: expect_text(get_required(&map, 2)?)?,
        ts: expect_u64(get_required(&map, 3)?)?,
        seq: expect_u64(get_required(&map, 4)?)?,
        payload_type: expect_u16(get_required(&map, 5)?)?,
        payload: get_required(&map, 6)?.clone(),
        economic_proof,
    })
}

pub fn parse_pubsub_envelope(value: &CborValue) -> Result<PubSubEnvelope, Error> {
    let (payload_entries, signature) = split_signed_map(value, 8)?;
    let payload_value = CborValue::Map(payload_entries);
    let payload = parse_pubsub_payload(&payload_value)?;
    Ok(PubSubEnvelope { payload, signature })
}

pub fn decode_pubsub_envelope(data: &[u8]) -> Result<PubSubEnvelope, Error> {
    let value = decode_canonical(data)?;
    parse_pubsub_envelope(&value)
}

pub fn build_pubsub_envelope(
    payload: &PubSubEnvelopePayload,
    secret_key: &[u8],
) -> Result<Vec<u8>, Error> {
    let payload_value = payload.to_cbor();
    let payload_cbor = encode_canonical(&payload_value)?;
    let hash = sha256(&payload_cbor);
    let sig = sign_ed25519_hash(secret_key, &hash)?;
    let full = with_signature(&payload_value, 8, sig)?;
    encode_canonical(&full)
}

pub fn verify_pubsub_envelope(
    data: &[u8],
    public_key: &[u8],
) -> Result<PubSubEnvelopePayload, Error> {
    let value = decode_canonical(data)?;
    let (payload_entries, signature) = split_signed_map(&value, 8)?;
    let payload_value = CborValue::Map(payload_entries);
    let payload_cbor = encode_canonical(&payload_value)?;
    let hash = sha256(&payload_cbor);
    verify_ed25519_hash(public_key, &hash, &signature)?;
    parse_pubsub_payload(&payload_value)
}

impl EconomicProof {
    pub fn to_cbor(&self) -> CborValue {
        match self {
            EconomicProof::OnChainTx { tx_hash } => CborValue::Map(vec![
                (CborValue::Unsigned(0), CborValue::Unsigned(1)),
                (CborValue::Unsigned(1), CborValue::Bytes(tx_hash.clone())),
            ]),
            EconomicProof::Voucher { voucher } => CborValue::Map(vec![
                (CborValue::Unsigned(0), CborValue::Unsigned(2)),
                (CborValue::Unsigned(1), CborValue::Bytes(voucher.clone())),
            ]),
        }
    }
}

impl PubSubEnvelopePayload {
    pub fn to_cbor(&self) -> CborValue {
        let mut entries = Vec::new();
        entries.push((
            CborValue::Unsigned(0),
            CborValue::Unsigned(self.version as u64),
        ));
        entries.push((CborValue::Unsigned(1), CborValue::Text(self.topic.clone())));
        entries.push((CborValue::Unsigned(2), CborValue::Text(self.sender.clone())));
        entries.push((CborValue::Unsigned(3), CborValue::Unsigned(self.ts)));
        entries.push((CborValue::Unsigned(4), CborValue::Unsigned(self.seq)));
        entries.push((
            CborValue::Unsigned(5),
            CborValue::Unsigned(self.payload_type as u64),
        ));
        entries.push((CborValue::Unsigned(6), self.payload.clone()));
        if let Some(proof) = &self.economic_proof {
            entries.push((CborValue::Unsigned(7), proof.to_cbor()));
        }
        CborValue::Map(entries)
    }
}

fn parse_economic_proof(value: &CborValue) -> Result<EconomicProof, Error> {
    let map = expect_map(value)?;
    let proof_type = expect_u8(get_required(&map, 0)?)?;
    match proof_type {
        1 => {
            let tx_hash = expect_bytes_len(get_required(&map, 1)?, 32)?;
            Ok(EconomicProof::OnChainTx { tx_hash })
        }
        2 => {
            let voucher = expect_bytes(get_required(&map, 1)?)?;
            Ok(EconomicProof::Voucher { voucher })
        }
        _ => Err(Error::Cbor("unsupported economic proof")),
    }
}
