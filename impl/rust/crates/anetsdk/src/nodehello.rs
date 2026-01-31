use crate::{
    decode_canonical, encode_canonical, parse_nodehello_payload, sha256, sign_ed25519_hash,
    verify_ed25519_hash, CborValue, Error, NodeHello,
};
use crate::signed::{split_signed_map, with_signature};

#[derive(Debug, Clone)]
pub struct NodeHelloPayload {
    pub protocols: Vec<String>,
    pub chain_id: String,
    pub node_id: String,
    pub node_pubkey: Vec<u8>,
    pub roles: Vec<String>,
    pub features: CborValue,
    pub time: u64,
    pub nonce: Vec<u8>,
}

impl NodeHelloPayload {
    pub fn to_cbor(&self) -> CborValue {
        CborValue::Map(vec![
            (CborValue::Unsigned(0), CborValue::Array(self.protocols.iter().map(|s| CborValue::Text(s.clone())).collect())),
            (CborValue::Unsigned(1), CborValue::Text(self.chain_id.clone())),
            (CborValue::Unsigned(2), CborValue::Text(self.node_id.clone())),
            (CborValue::Unsigned(3), CborValue::Bytes(self.node_pubkey.clone())),
            (CborValue::Unsigned(4), CborValue::Array(self.roles.iter().map(|s| CborValue::Text(s.clone())).collect())),
            (CborValue::Unsigned(5), self.features.clone()),
            (CborValue::Unsigned(6), CborValue::Unsigned(self.time)),
            (CborValue::Unsigned(7), CborValue::Bytes(self.nonce.clone())),
        ])
    }

    pub fn sign(&self, secret_key: &[u8]) -> Result<Vec<u8>, Error> {
        let payload_cbor = encode_canonical(&self.to_cbor())?;
        let hash = crate::sha256(&payload_cbor);
        sign_ed25519_hash(secret_key, &hash)
    }
}

pub fn payload_from_parsed(node: &NodeHello) -> NodeHelloPayload {
    NodeHelloPayload {
        protocols: node.protocols.clone(),
        chain_id: node.chain_id.clone(),
        node_id: node.node_id.clone(),
        node_pubkey: node.node_pubkey.clone(),
        roles: node.roles.clone(),
        features: node.features.clone(),
        time: node.time,
        nonce: node.nonce.clone(),
    }
}

pub fn build_nodehello(payload: &NodeHelloPayload, secret_key: &[u8]) -> Result<Vec<u8>, Error> {
    let sig = payload.sign(secret_key)?;
    let payload_cbor = payload.to_cbor();
    let full = with_signature(&payload_cbor, 8, sig)?;
    encode_canonical(&full)
}

pub fn decode_nodehello(data: &[u8]) -> Result<(NodeHelloPayload, Vec<u8>), Error> {
    let value = decode_canonical(data)?;
    let (payload_entries, signature) = split_signed_map(&value, 8)?;
    let payload_value = CborValue::Map(payload_entries);
    let parsed = parse_nodehello_payload(&payload_value)?;
    Ok((payload_from_parsed(&parsed), signature))
}

pub fn verify_nodehello(data: &[u8]) -> Result<NodeHelloPayload, Error> {
    let value = decode_canonical(data)?;
    let (payload_entries, signature) = split_signed_map(&value, 8)?;
    let payload_value = CborValue::Map(payload_entries);
    let payload_cbor = encode_canonical(&payload_value)?;
    let hash = sha256(&payload_cbor);
    let parsed = parse_nodehello_payload(&payload_value)?;
    verify_ed25519_hash(&parsed.node_pubkey, &hash, &signature)?;
    Ok(payload_from_parsed(&parsed))
}
