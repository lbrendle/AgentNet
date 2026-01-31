use serde::Deserialize;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Deserialize)]
pub struct AgentRecordIngest {
    pub cbor_hex: String,
    pub public_key_hex: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ServiceRecordIngest {
    pub cbor_hex: String,
    pub public_key_hex: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CommunityRecordIngest {
    pub cbor_hex: String,
    pub public_key_hex: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SkillManifestIngest {
    pub cbor_hex: String,
    pub public_key_hex: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WorkOfferIngest {
    pub cbor_hex: String,
    pub public_key_hex: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WorkAgreementIngest {
    pub cbor_hex: String,
    pub public_key_hex: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ReceiptIngest {
    pub payload_hex: String,
    pub signature_hex: String,
    pub public_key_hex: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct IdentityStateIngest {
    pub json: String,
}

#[derive(Debug, Deserialize)]
pub struct SkillRegistryStateIngest {
    pub json: String,
}

#[derive(Debug, Deserialize)]
pub struct WorkRegistryStateIngest {
    pub json: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub capability: Option<String>,
    pub sandbox_class: Option<u64>,
    pub currency: Option<String>,
    pub service_type: Option<u16>,
    pub provider_id: Option<String>,
    pub status: Option<String>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct IdentityStateSnapshot {
    pub records: HashMap<String, IdentityRecordSnapshot>,
    pub revocations: HashSet<String>,
}

#[derive(Debug, Deserialize)]
pub struct IdentityRecordSnapshot {
    pub did: String,
    pub pk_ed25519_hex: String,
    pub pk_x25519_hex: String,
    pub created: u64,
    pub updated: u64,
}

#[derive(Debug, Deserialize)]
pub struct SkillStateSnapshot {
    pub records: HashMap<String, SkillRecordSnapshot>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct SkillRecordSnapshot {
    pub skill_id: String,
    pub author: String,
    pub manifest_hash_hex: String,
    pub manifest_hex: String,
    pub name: String,
    pub version: String,
    pub summary: String,
    pub sandbox_class: u16,
    pub published_at: u64,
    pub updated_at: u64,
    pub revoked: bool,
    pub revoked_at: Option<u64>,
    pub revocation_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WorkStateSnapshot {
    pub offers: HashMap<String, WorkOfferRecordSnapshot>,
    pub agreements: HashMap<String, WorkAgreementRecordSnapshot>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct WorkOfferRecordSnapshot {
    pub offer_id: String,
    pub issuer: String,
    pub offer_hash_hex: String,
    pub offer_hex: String,
    pub title: String,
    pub summary: String,
    pub scope: String,
    pub budget_amount: u64,
    pub budget_currency: String,
    pub duration_sec: u64,
    pub deliverables: Vec<String>,
    pub requirements: Vec<String>,
    pub published_at: u64,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct WorkAgreementRecordSnapshot {
    pub agreement_id: String,
    pub offer_id: String,
    pub issuer: String,
    pub counterparty: String,
    pub agreement_hash_hex: String,
    pub agreement_hex: String,
    pub budget_amount: u64,
    pub budget_currency: String,
    pub start_ts: u64,
    pub end_ts: u64,
    pub deliverables: Vec<String>,
    pub milestones_count: u64,
    pub escrow_id: Option<String>,
    pub published_at: u64,
    pub updated_at: u64,
    pub closed: bool,
    pub closed_at: Option<u64>,
    pub close_reason: Option<String>,
}
