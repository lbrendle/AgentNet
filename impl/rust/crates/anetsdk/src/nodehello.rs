use crate::{encode_canonical, sign_ed25519_hash, CborValue, Error, NodeHello};

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
