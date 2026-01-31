use crate::schema::{
    expect_bytes_len, expect_map, expect_text, expect_text_array, expect_u16, expect_u64,
    expect_u8, get_optional, get_required,
};
use crate::signed::{split_signed_map, with_signature};
use crate::{
    decode_canonical, encode_canonical, sha256, sign_ed25519_hash, verify_ed25519_hash, CborValue,
    Error,
};

#[derive(Debug, Clone)]
pub struct Contact {
    pub node_ids: Vec<String>,
    pub addrs: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AgentRecordPayload {
    pub agent_id: String,
    pub agent_pubkeys: Vec<Vec<u8>>,
    pub contact: Contact,
    pub capabilities: Vec<String>,
    pub expires: u64,
}

#[derive(Debug, Clone)]
pub struct AgentRecord {
    pub payload: AgentRecordPayload,
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct ServiceRecordPayload {
    pub provider_id: String,
    pub service_type: u16,
    pub addrs: Vec<String>,
    pub required_credentials: Option<Vec<String>>,
    pub pricing: Option<CborValue>,
    pub expires: u64,
}

#[derive(Debug, Clone)]
pub struct ServiceRecord {
    pub payload: ServiceRecordPayload,
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct CommunityRecordPayload {
    pub community_id: String,
    pub controller: String,
    pub join_policy: u8,
    pub required_credentials: Option<Vec<String>>,
    pub economics: CborValue,
    pub governance: CborValue,
    pub expires: u64,
}

#[derive(Debug, Clone)]
pub struct CommunityRecord {
    pub payload: CommunityRecordPayload,
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct AgentProfilePayload {
    pub agent_id: String,
    pub display_name: String,
    pub summary: String,
    pub tags: Vec<String>,
    pub capabilities: Vec<String>,
    pub links: Option<Vec<String>>,
    pub visibility: u8,
    pub expires: u64,
}

#[derive(Debug, Clone)]
pub struct AgentProfileRecord {
    pub payload: AgentProfilePayload,
    pub signature: Vec<u8>,
}

pub fn parse_contact(value: &CborValue) -> Result<Contact, Error> {
    let map = expect_map(value)?;
    Ok(Contact {
        node_ids: expect_text_array(get_required(&map, 0)?)?,
        addrs: expect_text_array(get_required(&map, 1)?)?,
    })
}

pub fn parse_agent_record_payload(value: &CborValue) -> Result<AgentRecordPayload, Error> {
    let map = expect_map(value)?;
    Ok(AgentRecordPayload {
        agent_id: expect_text(get_required(&map, 0)?)?,
        agent_pubkeys: expect_bytes_array_len(get_required(&map, 1)?, 32)?,
        contact: parse_contact(get_required(&map, 2)?)?,
        capabilities: expect_text_array(get_required(&map, 3)?)?,
        expires: expect_u64(get_required(&map, 4)?)?,
    })
}

pub fn parse_agent_record(value: &CborValue) -> Result<AgentRecord, Error> {
    let (payload_entries, signature) = split_signed_map(value, 5)?;
    let payload_value = CborValue::Map(payload_entries);
    let payload = parse_agent_record_payload(&payload_value)?;
    Ok(AgentRecord { payload, signature })
}

pub fn parse_service_record_payload(value: &CborValue) -> Result<ServiceRecordPayload, Error> {
    let map = expect_map(value)?;
    let required_credentials = match get_optional(&map, 3) {
        Some(value) => Some(expect_text_array(value)?),
        None => None,
    };
    let pricing = get_optional(&map, 4).cloned();
    Ok(ServiceRecordPayload {
        provider_id: expect_text(get_required(&map, 0)?)?,
        service_type: expect_u16(get_required(&map, 1)?)?,
        addrs: expect_text_array(get_required(&map, 2)?)?,
        required_credentials,
        pricing,
        expires: expect_u64(get_required(&map, 5)?)?,
    })
}

pub fn parse_service_record(value: &CborValue) -> Result<ServiceRecord, Error> {
    let (payload_entries, signature) = split_signed_map(value, 6)?;
    let payload_value = CborValue::Map(payload_entries);
    let payload = parse_service_record_payload(&payload_value)?;
    Ok(ServiceRecord { payload, signature })
}

pub fn parse_community_record_payload(value: &CborValue) -> Result<CommunityRecordPayload, Error> {
    let map = expect_map(value)?;
    let required_credentials = match get_optional(&map, 3) {
        Some(value) => Some(expect_text_array(value)?),
        None => None,
    };
    Ok(CommunityRecordPayload {
        community_id: expect_text(get_required(&map, 0)?)?,
        controller: expect_text(get_required(&map, 1)?)?,
        join_policy: expect_u8(get_required(&map, 2)?)?,
        required_credentials,
        economics: get_required(&map, 4)?.clone(),
        governance: get_required(&map, 5)?.clone(),
        expires: expect_u64(get_required(&map, 6)?)?,
    })
}

pub fn parse_community_record(value: &CborValue) -> Result<CommunityRecord, Error> {
    let (payload_entries, signature) = split_signed_map(value, 7)?;
    let payload_value = CborValue::Map(payload_entries);
    let payload = parse_community_record_payload(&payload_value)?;
    Ok(CommunityRecord { payload, signature })
}

pub fn parse_agent_profile_payload(value: &CborValue) -> Result<AgentProfilePayload, Error> {
    let map = expect_map(value)?;
    let links = match get_optional(&map, 5) {
        Some(value) => Some(expect_text_array(value)?),
        None => None,
    };
    Ok(AgentProfilePayload {
        agent_id: expect_text(get_required(&map, 0)?)?,
        display_name: expect_text(get_required(&map, 1)?)?,
        summary: expect_text(get_required(&map, 2)?)?,
        tags: expect_text_array(get_required(&map, 3)?)?,
        capabilities: expect_text_array(get_required(&map, 4)?)?,
        links,
        visibility: expect_u8(get_required(&map, 6)?)?,
        expires: expect_u64(get_required(&map, 7)?)?,
    })
}

pub fn parse_agent_profile(value: &CborValue) -> Result<AgentProfileRecord, Error> {
    let (payload_entries, signature) = split_signed_map(value, 8)?;
    let payload_value = CborValue::Map(payload_entries);
    let payload = parse_agent_profile_payload(&payload_value)?;
    Ok(AgentProfileRecord { payload, signature })
}

impl Contact {
    pub fn to_cbor(&self) -> CborValue {
        CborValue::Map(vec![
            (
                CborValue::Unsigned(0),
                CborValue::Array(
                    self.node_ids
                        .iter()
                        .map(|s| CborValue::Text(s.clone()))
                        .collect(),
                ),
            ),
            (
                CborValue::Unsigned(1),
                CborValue::Array(
                    self.addrs
                        .iter()
                        .map(|s| CborValue::Text(s.clone()))
                        .collect(),
                ),
            ),
        ])
    }
}

impl AgentRecordPayload {
    pub fn to_cbor(&self) -> CborValue {
        CborValue::Map(vec![
            (
                CborValue::Unsigned(0),
                CborValue::Text(self.agent_id.clone()),
            ),
            (
                CborValue::Unsigned(1),
                CborValue::Array(
                    self.agent_pubkeys
                        .iter()
                        .map(|k| CborValue::Bytes(k.clone()))
                        .collect(),
                ),
            ),
            (CborValue::Unsigned(2), self.contact.to_cbor()),
            (
                CborValue::Unsigned(3),
                CborValue::Array(
                    self.capabilities
                        .iter()
                        .map(|s| CborValue::Text(s.clone()))
                        .collect(),
                ),
            ),
            (CborValue::Unsigned(4), CborValue::Unsigned(self.expires)),
        ])
    }
}

impl ServiceRecordPayload {
    pub fn to_cbor(&self) -> CborValue {
        let mut entries = Vec::new();
        entries.push((
            CborValue::Unsigned(0),
            CborValue::Text(self.provider_id.clone()),
        ));
        entries.push((
            CborValue::Unsigned(1),
            CborValue::Unsigned(self.service_type as u64),
        ));
        entries.push((
            CborValue::Unsigned(2),
            CborValue::Array(
                self.addrs
                    .iter()
                    .map(|s| CborValue::Text(s.clone()))
                    .collect(),
            ),
        ));
        if let Some(required) = &self.required_credentials {
            entries.push((
                CborValue::Unsigned(3),
                CborValue::Array(
                    required
                        .iter()
                        .map(|s| CborValue::Text(s.clone()))
                        .collect(),
                ),
            ));
        }
        if let Some(pricing) = &self.pricing {
            entries.push((CborValue::Unsigned(4), pricing.clone()));
        }
        entries.push((CborValue::Unsigned(5), CborValue::Unsigned(self.expires)));
        CborValue::Map(entries)
    }
}

impl CommunityRecordPayload {
    pub fn to_cbor(&self) -> CborValue {
        let mut entries = Vec::new();
        entries.push((
            CborValue::Unsigned(0),
            CborValue::Text(self.community_id.clone()),
        ));
        entries.push((
            CborValue::Unsigned(1),
            CborValue::Text(self.controller.clone()),
        ));
        entries.push((
            CborValue::Unsigned(2),
            CborValue::Unsigned(self.join_policy as u64),
        ));
        if let Some(required) = &self.required_credentials {
            entries.push((
                CborValue::Unsigned(3),
                CborValue::Array(
                    required
                        .iter()
                        .map(|s| CborValue::Text(s.clone()))
                        .collect(),
                ),
            ));
        }
        entries.push((CborValue::Unsigned(4), self.economics.clone()));
        entries.push((CborValue::Unsigned(5), self.governance.clone()));
        entries.push((CborValue::Unsigned(6), CborValue::Unsigned(self.expires)));
        CborValue::Map(entries)
    }
}

impl AgentProfilePayload {
    pub fn to_cbor(&self) -> CborValue {
        let mut entries = Vec::new();
        entries.push((
            CborValue::Unsigned(0),
            CborValue::Text(self.agent_id.clone()),
        ));
        entries.push((
            CborValue::Unsigned(1),
            CborValue::Text(self.display_name.clone()),
        ));
        entries.push((
            CborValue::Unsigned(2),
            CborValue::Text(self.summary.clone()),
        ));
        entries.push((
            CborValue::Unsigned(3),
            CborValue::Array(
                self.tags
                    .iter()
                    .map(|s| CborValue::Text(s.clone()))
                    .collect(),
            ),
        ));
        entries.push((
            CborValue::Unsigned(4),
            CborValue::Array(
                self.capabilities
                    .iter()
                    .map(|s| CborValue::Text(s.clone()))
                    .collect(),
            ),
        ));
        if let Some(links) = &self.links {
            entries.push((
                CborValue::Unsigned(5),
                CborValue::Array(
                    links.iter().map(|s| CborValue::Text(s.clone())).collect(),
                ),
            ));
        }
        entries.push((
            CborValue::Unsigned(6),
            CborValue::Unsigned(self.visibility as u64),
        ));
        entries.push((CborValue::Unsigned(7), CborValue::Unsigned(self.expires)));
        CborValue::Map(entries)
    }
}

pub fn build_agent_record(
    payload: &AgentRecordPayload,
    secret_key: &[u8],
) -> Result<Vec<u8>, Error> {
    build_signed_record(payload.to_cbor(), 5, secret_key)
}

pub fn build_service_record(
    payload: &ServiceRecordPayload,
    secret_key: &[u8],
) -> Result<Vec<u8>, Error> {
    build_signed_record(payload.to_cbor(), 6, secret_key)
}

pub fn build_community_record(
    payload: &CommunityRecordPayload,
    secret_key: &[u8],
) -> Result<Vec<u8>, Error> {
    build_signed_record(payload.to_cbor(), 7, secret_key)
}

pub fn verify_agent_record(data: &[u8], public_key: &[u8]) -> Result<AgentRecordPayload, Error> {
    verify_signed_record(data, 5, public_key, parse_agent_record_payload)
}

pub fn verify_service_record(
    data: &[u8],
    public_key: &[u8],
) -> Result<ServiceRecordPayload, Error> {
    verify_signed_record(data, 6, public_key, parse_service_record_payload)
}

pub fn verify_community_record(
    data: &[u8],
    public_key: &[u8],
) -> Result<CommunityRecordPayload, Error> {
    verify_signed_record(data, 7, public_key, parse_community_record_payload)
}

pub fn build_agent_profile(
    payload: &AgentProfilePayload,
    secret_key: &[u8],
) -> Result<Vec<u8>, Error> {
    build_signed_record(payload.to_cbor(), 8, secret_key)
}

pub fn verify_agent_profile(
    data: &[u8],
    public_key: &[u8],
) -> Result<AgentProfilePayload, Error> {
    verify_signed_record(data, 8, public_key, parse_agent_profile_payload)
}

fn build_signed_record(
    payload: CborValue,
    sig_key: u64,
    secret_key: &[u8],
) -> Result<Vec<u8>, Error> {
    let payload_cbor = encode_canonical(&payload)?;
    let hash = sha256(&payload_cbor);
    let sig = sign_ed25519_hash(secret_key, &hash)?;
    let full = with_signature(&payload, sig_key, sig)?;
    encode_canonical(&full)
}

fn verify_signed_record<T>(
    data: &[u8],
    sig_key: u64,
    public_key: &[u8],
    parse_payload: fn(&CborValue) -> Result<T, Error>,
) -> Result<T, Error> {
    let value = decode_canonical(data)?;
    let (payload_entries, signature) = split_signed_map(&value, sig_key)?;
    let payload_value = CborValue::Map(payload_entries);
    let payload_cbor = encode_canonical(&payload_value)?;
    let hash = sha256(&payload_cbor);
    verify_ed25519_hash(public_key, &hash, &signature)?;
    parse_payload(&payload_value)
}

fn expect_bytes_array_len(value: &CborValue, len: usize) -> Result<Vec<Vec<u8>>, Error> {
    match value {
        CborValue::Array(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                out.push(expect_bytes_len(item, len)?);
            }
            Ok(out)
        }
        _ => Err(Error::Cbor("expected array of bytes")),
    }
}
