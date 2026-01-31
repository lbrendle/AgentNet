use crate::schema::{expect_bytes_len, expect_map, expect_text, expect_u64, get_required};
use crate::{CborValue, Error};

#[derive(Debug, Clone)]
pub struct IdentityRegisterPayload {
    pub agent_id: String,
    pub pk_ed25519: Vec<u8>,
    pub pk_x25519: Vec<u8>,
    pub created: u64,
}

#[derive(Debug, Clone)]
pub struct IdentityRotatePayload {
    pub agent_id: String,
    pub pk_ed25519: Vec<u8>,
    pub pk_x25519: Vec<u8>,
    pub ts: u64,
}

#[derive(Debug, Clone)]
pub struct CredentialRevokePayload {
    pub issuer: String,
    pub credential_id_hash: Vec<u8>,
    pub ts: u64,
}

pub fn parse_identity_register_payload(value: &CborValue) -> Result<IdentityRegisterPayload, Error> {
    let map = expect_map(value)?;
    Ok(IdentityRegisterPayload {
        agent_id: expect_text(get_required(&map, 0)?)?,
        pk_ed25519: expect_bytes_len(get_required(&map, 1)?, 32)?,
        pk_x25519: expect_bytes_len(get_required(&map, 2)?, 32)?,
        created: expect_u64(get_required(&map, 3)?)?,
    })
}

pub fn parse_identity_rotate_payload(value: &CborValue) -> Result<IdentityRotatePayload, Error> {
    let map = expect_map(value)?;
    Ok(IdentityRotatePayload {
        agent_id: expect_text(get_required(&map, 0)?)?,
        pk_ed25519: expect_bytes_len(get_required(&map, 1)?, 32)?,
        pk_x25519: expect_bytes_len(get_required(&map, 2)?, 32)?,
        ts: expect_u64(get_required(&map, 3)?)?,
    })
}

pub fn parse_credential_revoke_payload(value: &CborValue) -> Result<CredentialRevokePayload, Error> {
    let map = expect_map(value)?;
    Ok(CredentialRevokePayload {
        issuer: expect_text(get_required(&map, 0)?)?,
        credential_id_hash: expect_bytes_len(get_required(&map, 1)?, 32)?,
        ts: expect_u64(get_required(&map, 2)?)?,
    })
}

pub fn identity_register_payload_to_cbor(payload: &IdentityRegisterPayload) -> CborValue {
    CborValue::Map(vec![
        (CborValue::Unsigned(0), CborValue::Text(payload.agent_id.clone())),
        (CborValue::Unsigned(1), CborValue::Bytes(payload.pk_ed25519.clone())),
        (CborValue::Unsigned(2), CborValue::Bytes(payload.pk_x25519.clone())),
        (CborValue::Unsigned(3), CborValue::Unsigned(payload.created)),
    ])
}

pub fn identity_rotate_payload_to_cbor(payload: &IdentityRotatePayload) -> CborValue {
    CborValue::Map(vec![
        (CborValue::Unsigned(0), CborValue::Text(payload.agent_id.clone())),
        (CborValue::Unsigned(1), CborValue::Bytes(payload.pk_ed25519.clone())),
        (CborValue::Unsigned(2), CborValue::Bytes(payload.pk_x25519.clone())),
        (CborValue::Unsigned(3), CborValue::Unsigned(payload.ts)),
    ])
}

pub fn credential_revoke_payload_to_cbor(payload: &CredentialRevokePayload) -> CborValue {
    CborValue::Map(vec![
        (CborValue::Unsigned(0), CborValue::Text(payload.issuer.clone())),
        (CborValue::Unsigned(1), CborValue::Bytes(payload.credential_id_hash.clone())),
        (CborValue::Unsigned(2), CborValue::Unsigned(payload.ts)),
    ])
}
