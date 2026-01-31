use crate::config::{
    BudgetConfig, BudgetCurrencyCap, EscrowConfig, IdentityConfig, SenderKeyConfig,
    SkillRegistryConfig, TxConfig, WorkRegistryConfig,
};
use anetsdk::{
    decode_tx_envelope, encode_canonical, parse_credential_revoke_payload,
    parse_escrow_dispute_payload, parse_escrow_lock_payload, parse_escrow_release_payload,
    parse_escrow_resolve_payload, parse_identity_register_payload, parse_identity_rotate_payload,
    parse_postage_payload, parse_skill_publish_payload, parse_skill_revoke_payload,
    parse_skill_update_payload, parse_transfer_payload, sha256, verify_skill_manifest,
    verify_tx_envelope, verify_work_agreement, verify_work_offer, CborValue, TxEnvelopePayload,
};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

const TX_ESCROW_LOCK: u64 = 40;
const TX_ESCROW_RELEASE: u64 = 41;
const TX_ESCROW_DISPUTE: u64 = 42;
const TX_ESCROW_RESOLVE: u64 = 43;
const TX_IDENTITY_REGISTER: u64 = 10;
const TX_IDENTITY_ROTATE: u64 = 11;
const TX_CRED_REVOKE: u64 = 12;
const TX_TRANSFER: u64 = 20;
const TX_POSTAGE: u64 = 21;
const TX_SKILL_PUBLISH: u64 = 30;
const TX_SKILL_UPDATE: u64 = 31;
const TX_SKILL_REVOKE: u64 = 32;
const TX_WORK_OFFER_PUBLISH: u64 = 50;
const TX_WORK_AGREEMENT_PUBLISH: u64 = 51;
const TX_WORK_AGREEMENT_UPDATE: u64 = 52;
const TX_WORK_AGREEMENT_CLOSE: u64 = 53;

const EV_PAYMENT_SENT: u64 = 2;
const EV_GOVERNANCE_EVENT: u64 = 7;
const EV_SKILL_REGISTRY: u64 = 9;
const EV_WORK_REGISTRY: u64 = 10;

#[derive(Debug)]
pub struct ReceiptSpec {
    pub event_type: u64,
    pub details: CborValue,
    pub economics: CborValue,
}

#[derive(Debug)]
pub struct TxDecision {
    pub accept: bool,
    pub reason: Option<String>,
    pub receipt: Option<ReceiptSpec>,
}

pub struct TxEngine {
    pubsub_payload_type: u16,
    sender_pubkeys: HashMap<String, Vec<u8>>,
    escrow: Option<EscrowLedger>,
    identity: Option<IdentityRegistry>,
    budget: Option<BudgetLedger>,
    skill_registry: Option<SkillRegistry>,
    work_registry: Option<WorkRegistry>,
}

impl TxEngine {
    pub fn build(config: &TxConfig, state_dir: &Path) -> Result<Option<Self>> {
        if !config.enabled() {
            return Ok(None);
        }
        let payload_type = config
            .pubsub_payload_type()
            .ok_or_else(|| anyhow::anyhow!("tx config enabled without pubsub_payload_type"))?;
        let sender_pubkeys = parse_sender_pubkeys(config.sender_pubkeys())?;
        let escrow = EscrowLedger::build(&config.escrow, state_dir)?;
        let identity = IdentityRegistry::build(&config.identity, state_dir)?;
        let budget = BudgetLedger::build(&config.budget, state_dir)?;
        let skill_registry = SkillRegistry::build(&config.skill_registry, state_dir)?;
        let work_registry = WorkRegistry::build(&config.work_registry, state_dir)?;
        Ok(Some(Self {
            pubsub_payload_type: payload_type,
            sender_pubkeys,
            escrow,
            identity,
            budget,
            skill_registry,
            work_registry,
        }))
    }

    pub fn matches_payload_type(&self, payload_type: u16) -> bool {
        payload_type == self.pubsub_payload_type
    }

    pub fn resolve_pubkey(&self, did: &str) -> Option<Vec<u8>> {
        self.resolve_sender_pubkey(did)
    }

    pub fn handle_pubsub_payload(
        &mut self,
        payload: &CborValue,
        economics: CborValue,
        now: u64,
    ) -> Result<TxDecision> {
        let tx_bytes = tx_payload_to_bytes(payload)?;
        let tx_hash = sha256(&tx_bytes);
        let envelope = decode_tx_envelope(&tx_bytes).context("decode tx envelope")?;
        let payload_verified: TxEnvelopePayload;
        let mut identity_payload = None;

        if envelope.payload.tx_type == TX_IDENTITY_REGISTER {
            if let Some(identity) = &self.identity {
                if !identity.config.allow_register() {
                    return Ok(TxDecision {
                        accept: false,
                        reason: Some("identity register disabled".to_string()),
                        receipt: None,
                    });
                }
            } else {
                return Ok(TxDecision {
                    accept: false,
                    reason: Some("identity registry disabled".to_string()),
                    receipt: None,
                });
            }
            let register_payload = parse_identity_register_payload(&envelope.payload.payload)
                .context("parse identity register")?;
            let sender_pubkey = register_payload.pk_ed25519.clone();
            payload_verified =
                verify_tx_envelope(&tx_bytes, &sender_pubkey).context("verify tx signature")?;
            identity_payload = Some(IdentityPayload::Register(register_payload));
        } else {
            let sender_pubkey = self
                .resolve_sender_pubkey(&envelope.payload.sender)
                .ok_or_else(|| anyhow::anyhow!("unknown tx sender"))?;
            payload_verified =
                verify_tx_envelope(&tx_bytes, &sender_pubkey).context("verify tx signature")?;
        }
        if payload_verified.sender != envelope.payload.sender {
            return Ok(TxDecision {
                accept: false,
                reason: Some("tx sender mismatch".to_string()),
                receipt: None,
            });
        }
        if payload_verified.tx_type != envelope.payload.tx_type {
            return Ok(TxDecision {
                accept: false,
                reason: Some("tx type mismatch".to_string()),
                receipt: None,
            });
        }
        let mut decision = TxDecision {
            accept: true,
            reason: None,
            receipt: None,
        };
        if let Some(budget) = &mut self.budget {
            if budget.applies_to(payload_verified.tx_type) {
                if let Err(reason) = budget.check_and_record(&payload_verified, now) {
                    return Ok(TxDecision {
                        accept: false,
                        reason: Some(reason),
                        receipt: None,
                    });
                }
            }
        }

        if is_identity_tx(payload_verified.tx_type) {
            decision =
                self.handle_identity_tx(&payload_verified, now, economics, identity_payload)?;
        } else if is_skill_tx(payload_verified.tx_type) {
            decision = self.handle_skill_tx(&payload_verified, now, economics)?;
        } else if is_work_tx(payload_verified.tx_type) {
            decision = self.handle_work_tx(&payload_verified, now, economics)?;
        } else if let Some(escrow) = &mut self.escrow {
            if is_escrow_tx(payload_verified.tx_type) {
                decision = escrow.apply(&payload_verified, &tx_hash, now, economics)?;
            }
        }
        Ok(decision)
    }

    fn resolve_sender_pubkey(&self, did: &str) -> Option<Vec<u8>> {
        if let Some(identity) = &self.identity {
            if let Some(pk) = identity.pubkey_for(did) {
                return Some(pk);
            }
        }
        self.sender_pubkeys.get(did).cloned()
    }

    fn handle_identity_tx(
        &mut self,
        payload: &TxEnvelopePayload,
        now: u64,
        economics: CborValue,
        identity_payload: Option<IdentityPayload>,
    ) -> Result<TxDecision> {
        let Some(identity) = &mut self.identity else {
            return Ok(TxDecision {
                accept: false,
                reason: Some("identity registry disabled".to_string()),
                receipt: None,
            });
        };
        match payload.tx_type {
            TX_IDENTITY_REGISTER => {
                let register = match identity_payload {
                    Some(IdentityPayload::Register(register)) => register,
                    None => {
                        return Ok(TxDecision {
                            accept: false,
                            reason: Some("invalid register payload".to_string()),
                            receipt: None,
                        });
                    }
                };
                if payload.sender != register.agent_id {
                    return Ok(TxDecision {
                        accept: false,
                        reason: Some("sender must match agent id".to_string()),
                        receipt: None,
                    });
                }
                identity.register(&register, now)?;
                let details = identity_details_register(&register);
                return Ok(TxDecision {
                    accept: true,
                    reason: None,
                    receipt: Some(ReceiptSpec {
                        event_type: EV_GOVERNANCE_EVENT,
                        details,
                        economics,
                    }),
                });
            }
            TX_IDENTITY_ROTATE => {
                if !identity.config.allow_rotate() {
                    return Ok(TxDecision {
                        accept: false,
                        reason: Some("identity rotate disabled".to_string()),
                        receipt: None,
                    });
                }
                let rotate = parse_identity_rotate_payload(&payload.payload)
                    .context("parse identity rotate")?;
                if payload.sender != rotate.agent_id {
                    return Ok(TxDecision {
                        accept: false,
                        reason: Some("sender must match agent id".to_string()),
                        receipt: None,
                    });
                }
                identity.rotate(&rotate, now)?;
                let details = identity_details_rotate(&rotate);
                return Ok(TxDecision {
                    accept: true,
                    reason: None,
                    receipt: Some(ReceiptSpec {
                        event_type: EV_GOVERNANCE_EVENT,
                        details,
                        economics,
                    }),
                });
            }
            TX_CRED_REVOKE => {
                if !identity.config.allow_revoke() {
                    return Ok(TxDecision {
                        accept: false,
                        reason: Some("credential revoke disabled".to_string()),
                        receipt: None,
                    });
                }
                let revoke = parse_credential_revoke_payload(&payload.payload)
                    .context("parse credential revoke")?;
                if payload.sender != revoke.issuer {
                    return Ok(TxDecision {
                        accept: false,
                        reason: Some("sender must match issuer".to_string()),
                        receipt: None,
                    });
                }
                identity.revoke(&revoke, now)?;
                let details = identity_details_revoke(&revoke);
                return Ok(TxDecision {
                    accept: true,
                    reason: None,
                    receipt: Some(ReceiptSpec {
                        event_type: EV_GOVERNANCE_EVENT,
                        details,
                        economics,
                    }),
                });
            }
            _ => Ok(TxDecision {
                accept: true,
                reason: None,
                receipt: None,
            }),
        }
    }

    fn handle_skill_tx(
        &mut self,
        payload: &TxEnvelopePayload,
        now: u64,
        economics: CborValue,
    ) -> Result<TxDecision> {
        let sender = payload.sender.clone();
        let sender_pubkey = self
            .resolve_sender_pubkey(&sender)
            .ok_or_else(|| anyhow::anyhow!("unknown tx sender"))?;
        let Some(skill_registry) = &mut self.skill_registry else {
            return Ok(TxDecision {
                accept: false,
                reason: Some("skill registry disabled".to_string()),
                receipt: None,
            });
        };

        let decision = match payload.tx_type {
            TX_SKILL_PUBLISH => {
                let publish = parse_skill_publish_payload(&payload.payload)
                    .context("parse skill publish payload")?;
                skill_registry.publish(&publish, &sender, &sender_pubkey, now, economics)?
            }
            TX_SKILL_UPDATE => {
                let update = parse_skill_update_payload(&payload.payload)
                    .context("parse skill update payload")?;
                skill_registry.update(&update, &sender, &sender_pubkey, now, economics)?
            }
            TX_SKILL_REVOKE => {
                let revoke = parse_skill_revoke_payload(&payload.payload)
                    .context("parse skill revoke payload")?;
                skill_registry.revoke(&revoke, &sender, now, economics)?
            }
            _ => TxDecision {
                accept: false,
                reason: Some("unsupported skill tx".to_string()),
                receipt: None,
            },
        };
        Ok(decision)
    }

    fn handle_work_tx(
        &mut self,
        payload: &TxEnvelopePayload,
        now: u64,
        economics: CborValue,
    ) -> Result<TxDecision> {
        let sender = payload.sender.clone();
        let sender_pubkey = self
            .resolve_sender_pubkey(&sender)
            .ok_or_else(|| anyhow::anyhow!("unknown tx sender"))?;
        let Some(work_registry) = &mut self.work_registry else {
            return Ok(TxDecision {
                accept: false,
                reason: Some("work registry disabled".to_string()),
                receipt: None,
            });
        };
        let decision = match payload.tx_type {
            TX_WORK_OFFER_PUBLISH => {
                let publish = anetsdk::parse_work_offer_publish_payload(&payload.payload)
                    .context("parse work offer publish payload")?;
                work_registry.publish_offer(&publish, &sender, &sender_pubkey, now, economics)?
            }
            TX_WORK_AGREEMENT_PUBLISH => {
                let publish = anetsdk::parse_work_agreement_publish_payload(&payload.payload)
                    .context("parse work agreement publish payload")?;
                work_registry.publish_agreement(
                    &publish,
                    &sender,
                    &sender_pubkey,
                    now,
                    economics,
                )?
            }
            TX_WORK_AGREEMENT_UPDATE => {
                let update = anetsdk::parse_work_agreement_update_payload(&payload.payload)
                    .context("parse work agreement update payload")?;
                work_registry.update_agreement(&update, &sender, &sender_pubkey, now, economics)?
            }
            TX_WORK_AGREEMENT_CLOSE => {
                let close = anetsdk::parse_work_agreement_close_payload(&payload.payload)
                    .context("parse work agreement close payload")?;
                work_registry.close_agreement(&close, &sender, now, economics)?
            }
            _ => TxDecision {
                accept: false,
                reason: Some("unsupported work tx".to_string()),
                receipt: None,
            },
        };
        Ok(decision)
    }
}

fn parse_sender_pubkeys(list: &[SenderKeyConfig]) -> Result<HashMap<String, Vec<u8>>> {
    let mut out = HashMap::new();
    for entry in list {
        let pk = hex::decode(&entry.pubkey_hex)
            .with_context(|| format!("decode sender pubkey for {}", entry.did))?;
        if pk.len() != 32 {
            anyhow::bail!("sender pubkey for {} must be 32 bytes", entry.did);
        }
        if out.insert(entry.did.clone(), pk).is_some() {
            anyhow::bail!("duplicate sender pubkey for {}", entry.did);
        }
    }
    Ok(out)
}

fn tx_payload_to_bytes(value: &CborValue) -> Result<Vec<u8>> {
    match value {
        CborValue::Bytes(bytes) => Ok(bytes.clone()),
        CborValue::Map(_) => encode_canonical(value).context("encode tx payload"),
        _ => Err(anyhow::anyhow!("tx payload must be bytes or map")),
    }
}

fn is_escrow_tx(tx_type: u64) -> bool {
    matches!(
        tx_type,
        TX_ESCROW_LOCK | TX_ESCROW_RELEASE | TX_ESCROW_DISPUTE | TX_ESCROW_RESOLVE
    )
}

fn is_identity_tx(tx_type: u64) -> bool {
    matches!(
        tx_type,
        TX_IDENTITY_REGISTER | TX_IDENTITY_ROTATE | TX_CRED_REVOKE
    )
}

fn is_skill_tx(tx_type: u64) -> bool {
    matches!(
        tx_type,
        TX_SKILL_PUBLISH | TX_SKILL_UPDATE | TX_SKILL_REVOKE
    )
}

fn is_work_tx(tx_type: u64) -> bool {
    matches!(
        tx_type,
        TX_WORK_OFFER_PUBLISH
            | TX_WORK_AGREEMENT_PUBLISH
            | TX_WORK_AGREEMENT_UPDATE
            | TX_WORK_AGREEMENT_CLOSE
    )
}

enum IdentityPayload {
    Register(anetsdk::IdentityRegisterPayload),
}

#[derive(Serialize, Deserialize, Clone)]
struct EscrowRecord {
    escrow_id: String,
    payer: String,
    payee: String,
    amount: u64,
    currency: String,
    release_condition_cbor_hex: String,
    dispute_window_sec: u64,
    expiry: u64,
    status: EscrowStatus,
    locked_at: u64,
    disputed_at: Option<u64>,
    resolved_at: Option<u64>,
    outcome: Option<EscrowOutcome>,
    split_amount_to_payee: Option<u64>,
    last_tx_hash: String,
}

#[derive(Serialize, Deserialize, Clone)]
enum EscrowStatus {
    Locked,
    Disputed,
    Released,
    Refunded,
    Split,
    Slashed,
    Expired,
}

#[derive(Serialize, Deserialize, Clone)]
enum EscrowOutcome {
    Release,
    Refund,
    Split,
    Slash,
}

struct EscrowLedger {
    state_path: PathBuf,
    log_path: PathBuf,
    records: HashMap<String, EscrowRecord>,
    arbitrators: HashSet<String>,
}

#[derive(Serialize, Deserialize, Clone)]
struct IdentityRecord {
    did: String,
    pk_ed25519_hex: String,
    pk_x25519_hex: String,
    created: u64,
    updated: u64,
}

struct IdentityRegistry {
    config: IdentityConfig,
    state_path: PathBuf,
    records: HashMap<String, IdentityRecord>,
    revocations: HashSet<String>,
}

#[derive(Serialize, Deserialize, Clone)]
struct SkillRecord {
    skill_id: String,
    author: String,
    manifest_hash_hex: String,
    manifest_hex: String,
    name: String,
    version: String,
    summary: String,
    sandbox_class: u16,
    published_at: u64,
    updated_at: u64,
    revoked: bool,
    revoked_at: Option<u64>,
    revocation_reason: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
struct SkillStateSnapshot {
    records: HashMap<String, SkillRecord>,
}

struct SkillRegistry {
    config: SkillRegistryConfig,
    state_path: PathBuf,
    records: HashMap<String, SkillRecord>,
}

#[derive(Serialize, Deserialize, Clone)]
struct WorkOfferRecord {
    offer_id: String,
    issuer: String,
    offer_hash_hex: String,
    offer_hex: String,
    title: String,
    summary: String,
    scope: String,
    budget_amount: u64,
    budget_currency: String,
    duration_sec: u64,
    deliverables: Vec<String>,
    requirements: Vec<String>,
    published_at: u64,
}

#[derive(Serialize, Deserialize, Clone)]
struct WorkAgreementRecord {
    agreement_id: String,
    offer_id: String,
    issuer: String,
    counterparty: String,
    agreement_hash_hex: String,
    agreement_hex: String,
    budget_amount: u64,
    budget_currency: String,
    start_ts: u64,
    end_ts: u64,
    deliverables: Vec<String>,
    milestones_count: u64,
    escrow_id: Option<String>,
    published_at: u64,
    updated_at: u64,
    closed: bool,
    closed_at: Option<u64>,
    close_reason: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
struct WorkStateSnapshot {
    offers: HashMap<String, WorkOfferRecord>,
    agreements: HashMap<String, WorkAgreementRecord>,
}

struct WorkRegistry {
    config: WorkRegistryConfig,
    state_path: PathBuf,
    offers: HashMap<String, WorkOfferRecord>,
    agreements: HashMap<String, WorkAgreementRecord>,
}

struct BudgetLedger {
    window_sec: u64,
    caps: HashMap<String, u64>,
    state_path: PathBuf,
    state: HashMap<String, HashMap<String, BudgetWindow>>,
}

#[derive(Serialize, Deserialize, Clone)]
struct BudgetWindow {
    window_start: u64,
    spent: u64,
}

#[derive(Serialize, Deserialize, Clone)]
struct BudgetStateSnapshot {
    windows: HashMap<String, HashMap<String, BudgetWindow>>,
}

impl IdentityRegistry {
    fn build(config: &IdentityConfig, state_dir: &Path) -> Result<Option<Self>> {
        if !config.enabled() {
            return Ok(None);
        }
        let state_path = config.state_path_or_default(state_dir);
        if let Some(parent) = state_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create identity state dir {}", parent.display()))?;
        }
        let (records, revocations) = load_identity_state(&state_path)?;
        Ok(Some(Self {
            config: config.clone(),
            state_path,
            records,
            revocations,
        }))
    }

    fn pubkey_for(&self, did: &str) -> Option<Vec<u8>> {
        self.records
            .get(did)
            .and_then(|record| hex::decode(&record.pk_ed25519_hex).ok())
    }

    fn register(&mut self, payload: &anetsdk::IdentityRegisterPayload, now: u64) -> Result<()> {
        if !self.config.allow_register() {
            anyhow::bail!("identity register disabled");
        }
        if payload.agent_id.is_empty() {
            anyhow::bail!("agent id required");
        }
        if self.records.contains_key(&payload.agent_id) {
            anyhow::bail!("identity already registered");
        }
        self.ensure_clock(payload.created, now)?;
        let record = IdentityRecord {
            did: payload.agent_id.clone(),
            pk_ed25519_hex: hex::encode(&payload.pk_ed25519),
            pk_x25519_hex: hex::encode(&payload.pk_x25519),
            created: payload.created,
            updated: payload.created,
        };
        self.records.insert(payload.agent_id.clone(), record);
        self.persist()?;
        Ok(())
    }

    fn rotate(&mut self, payload: &anetsdk::IdentityRotatePayload, now: u64) -> Result<()> {
        if !self.config.allow_rotate() {
            anyhow::bail!("identity rotate disabled");
        }
        self.ensure_clock(payload.ts, now)?;
        let record = self
            .records
            .get_mut(&payload.agent_id)
            .ok_or_else(|| anyhow::anyhow!("identity not registered"))?;
        record.pk_ed25519_hex = hex::encode(&payload.pk_ed25519);
        record.pk_x25519_hex = hex::encode(&payload.pk_x25519);
        record.updated = payload.ts;
        self.persist()?;
        Ok(())
    }

    fn revoke(&mut self, payload: &anetsdk::CredentialRevokePayload, now: u64) -> Result<()> {
        if !self.config.allow_revoke() {
            anyhow::bail!("credential revoke disabled");
        }
        self.ensure_clock(payload.ts, now)?;
        let key = format!(
            "{}:{}",
            payload.issuer,
            hex::encode(&payload.credential_id_hash)
        );
        self.revocations.insert(key);
        self.persist()?;
        Ok(())
    }

    fn ensure_clock(&self, ts: u64, now: u64) -> Result<()> {
        let skew = if ts > now { ts - now } else { now - ts };
        if skew > self.config.max_clock_skew_sec() as u64 {
            anyhow::bail!("identity timestamp outside window");
        }
        Ok(())
    }

    fn persist(&self) -> Result<()> {
        let snapshot = IdentityStateSnapshot {
            records: self.records.clone(),
            revocations: self.revocations.clone(),
        };
        let data = serde_json::to_vec_pretty(&snapshot).context("encode identity state")?;
        fs::write(&self.state_path, data)
            .with_context(|| format!("write identity state {}", self.state_path.display()))?;
        Ok(())
    }
}

impl SkillRegistry {
    fn build(config: &SkillRegistryConfig, state_dir: &Path) -> Result<Option<Self>> {
        if !config.enabled() {
            return Ok(None);
        }
        let state_path = config.state_path_or_default(state_dir);
        if let Some(parent) = state_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create skill registry dir {}", parent.display()))?;
        }
        let records = load_skill_state(&state_path)?;
        Ok(Some(Self {
            config: config.clone(),
            state_path,
            records,
        }))
    }

    fn publish(
        &mut self,
        payload: &anetsdk::SkillPublishPayload,
        sender: &str,
        sender_pubkey: &[u8],
        now: u64,
        economics: CborValue,
    ) -> Result<TxDecision> {
        if !self.config.allow_publish() {
            return Ok(TxDecision {
                accept: false,
                reason: Some("skill publish disabled".to_string()),
                receipt: None,
            });
        }
        self.ensure_clock(payload.ts, now)?;
        let manifest_payload = verify_skill_manifest(&payload.manifest, sender_pubkey)
            .context("verify skill manifest")?;
        self.ensure_clock(manifest_payload.ts, now)?;
        self.ensure_ts_alignment(payload.ts, manifest_payload.ts)?;
        if manifest_payload.author != sender {
            return Ok(TxDecision {
                accept: false,
                reason: Some("manifest author mismatch".to_string()),
                receipt: None,
            });
        }
        if self.records.contains_key(&manifest_payload.skill_id) {
            return Ok(TxDecision {
                accept: false,
                reason: Some("skill already exists".to_string()),
                receipt: None,
            });
        }
        let manifest_hash = sha256(&payload.manifest);
        let record = SkillRecord {
            skill_id: manifest_payload.skill_id.clone(),
            author: manifest_payload.author.clone(),
            manifest_hash_hex: hex::encode(manifest_hash),
            manifest_hex: hex::encode(&payload.manifest),
            name: manifest_payload.name.clone(),
            version: manifest_payload.version.clone(),
            summary: manifest_payload.summary.clone(),
            sandbox_class: manifest_payload.sandbox_class,
            published_at: payload.ts,
            updated_at: payload.ts,
            revoked: false,
            revoked_at: None,
            revocation_reason: None,
        };
        self.records.insert(record.skill_id.clone(), record);
        self.persist()?;
        Ok(TxDecision {
            accept: true,
            reason: None,
            receipt: Some(ReceiptSpec {
                event_type: EV_SKILL_REGISTRY,
                details: skill_details_publish(
                    &manifest_payload.skill_id,
                    sender,
                    &manifest_payload.version,
                    &manifest_hash,
                ),
                economics,
            }),
        })
    }

    fn update(
        &mut self,
        payload: &anetsdk::SkillUpdatePayload,
        sender: &str,
        sender_pubkey: &[u8],
        now: u64,
        economics: CborValue,
    ) -> Result<TxDecision> {
        if !self.config.allow_update() {
            return Ok(TxDecision {
                accept: false,
                reason: Some("skill update disabled".to_string()),
                receipt: None,
            });
        }
        self.ensure_clock(payload.ts, now)?;
        let manifest_payload = verify_skill_manifest(&payload.manifest, sender_pubkey)
            .context("verify skill manifest")?;
        self.ensure_clock(manifest_payload.ts, now)?;
        self.ensure_ts_alignment(payload.ts, manifest_payload.ts)?;
        if manifest_payload.author != sender {
            return Ok(TxDecision {
                accept: false,
                reason: Some("manifest author mismatch".to_string()),
                receipt: None,
            });
        }
        if payload.skill_id != manifest_payload.skill_id {
            return Ok(TxDecision {
                accept: false,
                reason: Some("skill id mismatch".to_string()),
                receipt: None,
            });
        }
        let record = self
            .records
            .get_mut(&payload.skill_id)
            .ok_or_else(|| anyhow::anyhow!("skill not found"))?;
        if record.revoked {
            return Ok(TxDecision {
                accept: false,
                reason: Some("skill revoked".to_string()),
                receipt: None,
            });
        }
        if record.author != sender {
            return Ok(TxDecision {
                accept: false,
                reason: Some("sender must match author".to_string()),
                receipt: None,
            });
        }
        if record.manifest_hash_hex != hex::encode(&payload.prev_manifest_hash) {
            return Ok(TxDecision {
                accept: false,
                reason: Some("prev manifest hash mismatch".to_string()),
                receipt: None,
            });
        }
        let manifest_hash = sha256(&payload.manifest);
        if record.manifest_hash_hex == hex::encode(manifest_hash) {
            return Ok(TxDecision {
                accept: false,
                reason: Some("manifest hash unchanged".to_string()),
                receipt: None,
            });
        }
        record.manifest_hash_hex = hex::encode(manifest_hash);
        record.manifest_hex = hex::encode(&payload.manifest);
        record.name = manifest_payload.name.clone();
        record.version = manifest_payload.version.clone();
        record.summary = manifest_payload.summary.clone();
        record.sandbox_class = manifest_payload.sandbox_class;
        record.updated_at = payload.ts;
        self.persist()?;
        Ok(TxDecision {
            accept: true,
            reason: None,
            receipt: Some(ReceiptSpec {
                event_type: EV_SKILL_REGISTRY,
                details: skill_details_update(
                    &manifest_payload.skill_id,
                    sender,
                    &manifest_payload.version,
                    &payload.prev_manifest_hash,
                    &manifest_hash,
                ),
                economics,
            }),
        })
    }

    fn revoke(
        &mut self,
        payload: &anetsdk::SkillRevokePayload,
        sender: &str,
        now: u64,
        economics: CborValue,
    ) -> Result<TxDecision> {
        if !self.config.allow_revoke() {
            return Ok(TxDecision {
                accept: false,
                reason: Some("skill revoke disabled".to_string()),
                receipt: None,
            });
        }
        self.ensure_clock(payload.ts, now)?;
        let record = self
            .records
            .get_mut(&payload.skill_id)
            .ok_or_else(|| anyhow::anyhow!("skill not found"))?;
        if record.revoked {
            return Ok(TxDecision {
                accept: false,
                reason: Some("skill already revoked".to_string()),
                receipt: None,
            });
        }
        if record.author != sender {
            return Ok(TxDecision {
                accept: false,
                reason: Some("sender must match author".to_string()),
                receipt: None,
            });
        }
        if record.manifest_hash_hex != hex::encode(&payload.manifest_hash) {
            return Ok(TxDecision {
                accept: false,
                reason: Some("manifest hash mismatch".to_string()),
                receipt: None,
            });
        }
        record.revoked = true;
        record.revoked_at = Some(payload.ts);
        record.revocation_reason = Some(payload.reason.clone());
        self.persist()?;
        Ok(TxDecision {
            accept: true,
            reason: None,
            receipt: Some(ReceiptSpec {
                event_type: EV_SKILL_REGISTRY,
                details: skill_details_revoke(
                    &payload.skill_id,
                    sender,
                    &payload.manifest_hash,
                    &payload.reason,
                ),
                economics,
            }),
        })
    }

    fn ensure_clock(&self, ts: u64, now: u64) -> Result<()> {
        let skew = if ts > now { ts - now } else { now - ts };
        if skew > self.config.max_clock_skew_sec() as u64 {
            anyhow::bail!("skill timestamp outside window");
        }
        Ok(())
    }

    fn ensure_ts_alignment(&self, ts: u64, manifest_ts: u64) -> Result<()> {
        let diff = if ts > manifest_ts {
            ts - manifest_ts
        } else {
            manifest_ts - ts
        };
        if diff > self.config.max_clock_skew_sec() as u64 {
            anyhow::bail!("manifest timestamp mismatch");
        }
        Ok(())
    }

    fn persist(&self) -> Result<()> {
        let snapshot = SkillStateSnapshot {
            records: self.records.clone(),
        };
        let data = serde_json::to_vec_pretty(&snapshot).context("encode skill state")?;
        fs::write(&self.state_path, data)
            .with_context(|| format!("write skill state {}", self.state_path.display()))?;
        Ok(())
    }
}

impl WorkRegistry {
    fn build(config: &WorkRegistryConfig, state_dir: &Path) -> Result<Option<Self>> {
        if !config.enabled() {
            return Ok(None);
        }
        let state_path = config.state_path_or_default(state_dir);
        if let Some(parent) = state_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create work registry dir {}", parent.display()))?;
        }
        let (offers, agreements) = load_work_state(&state_path)?;
        Ok(Some(Self {
            config: config.clone(),
            state_path,
            offers,
            agreements,
        }))
    }

    fn publish_offer(
        &mut self,
        payload: &anetsdk::WorkOfferPublishPayload,
        sender: &str,
        sender_pubkey: &[u8],
        now: u64,
        economics: CborValue,
    ) -> Result<TxDecision> {
        if !self.config.allow_offer_publish() {
            return Ok(TxDecision {
                accept: false,
                reason: Some("work offer publish disabled".to_string()),
                receipt: None,
            });
        }
        self.ensure_clock(payload.ts, now)?;
        let offer_payload =
            verify_work_offer(&payload.offer, sender_pubkey).context("verify work offer")?;
        self.ensure_clock(offer_payload.ts, now)?;
        self.ensure_ts_alignment(payload.ts, offer_payload.ts)?;
        if offer_payload.issuer != sender {
            return Ok(TxDecision {
                accept: false,
                reason: Some("offer issuer mismatch".to_string()),
                receipt: None,
            });
        }
        if self.offers.contains_key(&offer_payload.offer_id) {
            return Ok(TxDecision {
                accept: false,
                reason: Some("offer already exists".to_string()),
                receipt: None,
            });
        }
        let offer_hash = sha256(&payload.offer);
        let record = WorkOfferRecord {
            offer_id: offer_payload.offer_id.clone(),
            issuer: offer_payload.issuer.clone(),
            offer_hash_hex: hex::encode(offer_hash),
            offer_hex: hex::encode(&payload.offer),
            title: offer_payload.title.clone(),
            summary: offer_payload.summary.clone(),
            scope: offer_payload.scope.clone(),
            budget_amount: offer_payload.budget_amount,
            budget_currency: offer_payload.budget_currency.clone(),
            duration_sec: offer_payload.duration_sec,
            deliverables: offer_payload.deliverables.clone(),
            requirements: offer_payload.requirements.clone().unwrap_or_default(),
            published_at: payload.ts,
        };
        self.offers.insert(record.offer_id.clone(), record);
        self.persist()?;
        Ok(TxDecision {
            accept: true,
            reason: None,
            receipt: Some(ReceiptSpec {
                event_type: EV_WORK_REGISTRY,
                details: work_details_offer_publish(&offer_payload.offer_id, sender, &offer_hash),
                economics,
            }),
        })
    }

    fn publish_agreement(
        &mut self,
        payload: &anetsdk::WorkAgreementPublishPayload,
        sender: &str,
        sender_pubkey: &[u8],
        now: u64,
        economics: CborValue,
    ) -> Result<TxDecision> {
        if !self.config.allow_agreement_publish() {
            return Ok(TxDecision {
                accept: false,
                reason: Some("work agreement publish disabled".to_string()),
                receipt: None,
            });
        }
        self.ensure_clock(payload.ts, now)?;
        let agreement_payload = verify_work_agreement(&payload.agreement, sender_pubkey)
            .context("verify work agreement")?;
        self.ensure_clock(agreement_payload.ts, now)?;
        self.ensure_ts_alignment(payload.ts, agreement_payload.ts)?;
        if agreement_payload.issuer != sender {
            return Ok(TxDecision {
                accept: false,
                reason: Some("agreement issuer mismatch".to_string()),
                receipt: None,
            });
        }
        if self
            .agreements
            .contains_key(&agreement_payload.agreement_id)
        {
            return Ok(TxDecision {
                accept: false,
                reason: Some("agreement already exists".to_string()),
                receipt: None,
            });
        }
        let offer = self
            .offers
            .get(&agreement_payload.offer_id)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "offer not found for agreement {}",
                    agreement_payload.offer_id
                )
            })?;
        if offer.issuer != agreement_payload.issuer {
            return Ok(TxDecision {
                accept: false,
                reason: Some("offer issuer mismatch".to_string()),
                receipt: None,
            });
        }
        let agreement_hash = sha256(&payload.agreement);
        let record = WorkAgreementRecord {
            agreement_id: agreement_payload.agreement_id.clone(),
            offer_id: agreement_payload.offer_id.clone(),
            issuer: agreement_payload.issuer.clone(),
            counterparty: agreement_payload.counterparty.clone(),
            agreement_hash_hex: hex::encode(agreement_hash),
            agreement_hex: hex::encode(&payload.agreement),
            budget_amount: agreement_payload.budget_amount,
            budget_currency: agreement_payload.budget_currency.clone(),
            start_ts: agreement_payload.start_ts,
            end_ts: agreement_payload.end_ts,
            deliverables: agreement_payload.deliverables.clone(),
            milestones_count: agreement_payload
                .milestones
                .as_ref()
                .map(|m| m.len() as u64)
                .unwrap_or(0),
            escrow_id: agreement_payload.escrow_id.clone(),
            published_at: payload.ts,
            updated_at: payload.ts,
            closed: false,
            closed_at: None,
            close_reason: None,
        };
        self.agreements.insert(record.agreement_id.clone(), record);
        self.persist()?;
        Ok(TxDecision {
            accept: true,
            reason: None,
            receipt: Some(ReceiptSpec {
                event_type: EV_WORK_REGISTRY,
                details: work_details_agreement_publish(
                    &agreement_payload.agreement_id,
                    sender,
                    &agreement_hash,
                ),
                economics,
            }),
        })
    }

    fn update_agreement(
        &mut self,
        payload: &anetsdk::WorkAgreementUpdatePayload,
        sender: &str,
        sender_pubkey: &[u8],
        now: u64,
        economics: CborValue,
    ) -> Result<TxDecision> {
        if !self.config.allow_agreement_update() {
            return Ok(TxDecision {
                accept: false,
                reason: Some("work agreement update disabled".to_string()),
                receipt: None,
            });
        }
        self.ensure_clock(payload.ts, now)?;
        let agreement_payload = verify_work_agreement(&payload.agreement, sender_pubkey)
            .context("verify work agreement")?;
        self.ensure_clock(agreement_payload.ts, now)?;
        self.ensure_ts_alignment(payload.ts, agreement_payload.ts)?;
        if agreement_payload.issuer != sender {
            return Ok(TxDecision {
                accept: false,
                reason: Some("agreement issuer mismatch".to_string()),
                receipt: None,
            });
        }
        if payload.agreement_id != agreement_payload.agreement_id {
            return Ok(TxDecision {
                accept: false,
                reason: Some("agreement id mismatch".to_string()),
                receipt: None,
            });
        }
        let record = self
            .agreements
            .get_mut(&payload.agreement_id)
            .ok_or_else(|| anyhow::anyhow!("agreement not found"))?;
        if record.closed {
            return Ok(TxDecision {
                accept: false,
                reason: Some("agreement closed".to_string()),
                receipt: None,
            });
        }
        if record.issuer != sender {
            return Ok(TxDecision {
                accept: false,
                reason: Some("sender must match issuer".to_string()),
                receipt: None,
            });
        }
        if record.agreement_hash_hex != hex::encode(&payload.prev_agreement_hash) {
            return Ok(TxDecision {
                accept: false,
                reason: Some("prev agreement hash mismatch".to_string()),
                receipt: None,
            });
        }
        let agreement_hash = sha256(&payload.agreement);
        if record.agreement_hash_hex == hex::encode(agreement_hash) {
            return Ok(TxDecision {
                accept: false,
                reason: Some("agreement hash unchanged".to_string()),
                receipt: None,
            });
        }
        record.offer_id = agreement_payload.offer_id.clone();
        record.counterparty = agreement_payload.counterparty.clone();
        record.agreement_hash_hex = hex::encode(agreement_hash);
        record.agreement_hex = hex::encode(&payload.agreement);
        record.budget_amount = agreement_payload.budget_amount;
        record.budget_currency = agreement_payload.budget_currency.clone();
        record.start_ts = agreement_payload.start_ts;
        record.end_ts = agreement_payload.end_ts;
        record.deliverables = agreement_payload.deliverables.clone();
        record.milestones_count = agreement_payload
            .milestones
            .as_ref()
            .map(|m| m.len() as u64)
            .unwrap_or(0);
        record.escrow_id = agreement_payload.escrow_id.clone();
        record.updated_at = payload.ts;
        self.persist()?;
        Ok(TxDecision {
            accept: true,
            reason: None,
            receipt: Some(ReceiptSpec {
                event_type: EV_WORK_REGISTRY,
                details: work_details_agreement_update(
                    &agreement_payload.agreement_id,
                    sender,
                    &payload.prev_agreement_hash,
                    &agreement_hash,
                ),
                economics,
            }),
        })
    }

    fn close_agreement(
        &mut self,
        payload: &anetsdk::WorkAgreementClosePayload,
        sender: &str,
        now: u64,
        economics: CborValue,
    ) -> Result<TxDecision> {
        if !self.config.allow_agreement_close() {
            return Ok(TxDecision {
                accept: false,
                reason: Some("work agreement close disabled".to_string()),
                receipt: None,
            });
        }
        self.ensure_clock(payload.ts, now)?;
        let record = self
            .agreements
            .get_mut(&payload.agreement_id)
            .ok_or_else(|| anyhow::anyhow!("agreement not found"))?;
        if record.closed {
            return Ok(TxDecision {
                accept: false,
                reason: Some("agreement already closed".to_string()),
                receipt: None,
            });
        }
        if sender != record.issuer && sender != record.counterparty {
            return Ok(TxDecision {
                accept: false,
                reason: Some("sender must be issuer or counterparty".to_string()),
                receipt: None,
            });
        }
        if record.agreement_hash_hex != hex::encode(&payload.agreement_hash) {
            return Ok(TxDecision {
                accept: false,
                reason: Some("agreement hash mismatch".to_string()),
                receipt: None,
            });
        }
        record.closed = true;
        record.closed_at = Some(payload.ts);
        record.close_reason = Some(payload.reason.clone());
        self.persist()?;
        Ok(TxDecision {
            accept: true,
            reason: None,
            receipt: Some(ReceiptSpec {
                event_type: EV_WORK_REGISTRY,
                details: work_details_agreement_close(
                    &payload.agreement_id,
                    sender,
                    &payload.agreement_hash,
                    &payload.reason,
                ),
                economics,
            }),
        })
    }

    fn ensure_clock(&self, ts: u64, now: u64) -> Result<()> {
        let skew = if ts > now { ts - now } else { now - ts };
        if skew > self.config.max_clock_skew_sec() as u64 {
            anyhow::bail!("work timestamp outside window");
        }
        Ok(())
    }

    fn ensure_ts_alignment(&self, ts: u64, inner_ts: u64) -> Result<()> {
        let diff = if ts > inner_ts {
            ts - inner_ts
        } else {
            inner_ts - ts
        };
        if diff > self.config.max_clock_skew_sec() as u64 {
            anyhow::bail!("work timestamp mismatch");
        }
        Ok(())
    }

    fn persist(&self) -> Result<()> {
        let snapshot = WorkStateSnapshot {
            offers: self.offers.clone(),
            agreements: self.agreements.clone(),
        };
        let data = serde_json::to_vec_pretty(&snapshot).context("encode work state")?;
        fs::write(&self.state_path, data)
            .with_context(|| format!("write work state {}", self.state_path.display()))?;
        Ok(())
    }
}

impl BudgetLedger {
    fn build(config: &BudgetConfig, state_dir: &Path) -> Result<Option<Self>> {
        if !config.enabled() {
            return Ok(None);
        }
        let window_sec = config
            .window_sec()
            .ok_or_else(|| anyhow::anyhow!("budget enabled without window_sec"))?;
        if config.caps().is_empty() {
            anyhow::bail!("budget enabled without currency caps");
        }
        let caps = build_caps(config.caps())?;
        let state_path = config.state_path_or_default(state_dir);
        if let Some(parent) = state_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create budget state dir {}", parent.display()))?;
        }
        let state = load_budget_state(&state_path)?;
        Ok(Some(Self {
            window_sec,
            caps,
            state_path,
            state,
        }))
    }

    fn applies_to(&self, tx_type: u64) -> bool {
        matches!(tx_type, TX_ESCROW_LOCK | TX_TRANSFER | TX_POSTAGE)
    }

    fn check_and_record(&mut self, payload: &TxEnvelopePayload, now: u64) -> Result<(), String> {
        let (amount, currency, spender) = match payload.tx_type {
            TX_ESCROW_LOCK => {
                let lock =
                    parse_escrow_lock_payload(&payload.payload).map_err(|e| e.to_string())?;
                if payload.sender != lock.payer {
                    return Err("sender must be payer".to_string());
                }
                (lock.amount, lock.currency, lock.payer)
            }
            TX_TRANSFER => {
                let transfer =
                    parse_transfer_payload(&payload.payload).map_err(|e| e.to_string())?;
                if payload.sender != transfer.from {
                    return Err("sender must be transfer origin".to_string());
                }
                (transfer.amount, transfer.currency, transfer.from)
            }
            TX_POSTAGE => {
                let postage = parse_postage_payload(&payload.payload).map_err(|e| e.to_string())?;
                if payload.sender != postage.payer {
                    return Err("sender must be postage payer".to_string());
                }
                (postage.amount, postage.currency, postage.payer)
            }
            _ => return Ok(()),
        };

        if amount == 0 {
            return Err("amount must be > 0".to_string());
        }
        let cap = self
            .caps
            .get(&currency)
            .ok_or_else(|| "currency not permitted".to_string())?;
        let sender_entry = self
            .state
            .entry(spender.clone())
            .or_insert_with(HashMap::new);
        let window = sender_entry
            .entry(currency.clone())
            .or_insert(BudgetWindow {
                window_start: now,
                spent: 0,
            });
        if now.saturating_sub(window.window_start) >= self.window_sec {
            window.window_start = now;
            window.spent = 0;
        }
        let next_spent = window.spent.saturating_add(amount);
        if next_spent > *cap {
            return Err("budget exceeded".to_string());
        }
        window.spent = next_spent;
        self.persist().map_err(|e| e.to_string())?;
        Ok(())
    }

    fn persist(&self) -> Result<()> {
        let snapshot = BudgetStateSnapshot {
            windows: self.state.clone(),
        };
        let data = serde_json::to_vec_pretty(&snapshot).context("encode budget state")?;
        fs::write(&self.state_path, data)
            .with_context(|| format!("write budget state {}", self.state_path.display()))?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct IdentityStateSnapshot {
    records: HashMap<String, IdentityRecord>,
    revocations: HashSet<String>,
}

impl EscrowLedger {
    fn build(config: &EscrowConfig, state_dir: &Path) -> Result<Option<Self>> {
        if !config.enabled() {
            return Ok(None);
        }
        let state_path = config.state_path_or_default(state_dir);
        let log_path = config.log_path_or_default(state_dir);
        if let Some(parent) = state_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create escrow state dir {}", parent.display()))?;
        }
        if let Some(parent) = log_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create escrow log dir {}", parent.display()))?;
        }
        let records = load_state(&state_path)?;
        Ok(Some(Self {
            state_path,
            log_path,
            records,
            arbitrators: config.arbitrators.iter().cloned().collect(),
        }))
    }

    fn apply(
        &mut self,
        payload: &TxEnvelopePayload,
        tx_hash: &[u8; 32],
        now: u64,
        economics: CborValue,
    ) -> Result<TxDecision> {
        self.expire_if_needed(now)?;
        let tx_hash_hex = hex::encode(tx_hash);
        let mut decision = TxDecision {
            accept: true,
            reason: None,
            receipt: None,
        };
        match payload.tx_type {
            TX_ESCROW_LOCK => {
                let lock =
                    parse_escrow_lock_payload(&payload.payload).context("parse escrow lock")?;
                if self.records.contains_key(&lock.escrow_id) {
                    return Ok(self.reject(
                        &tx_hash_hex,
                        payload.tx_type,
                        &lock.escrow_id,
                        "escrow already exists",
                    )?);
                }
                if lock.amount == 0 {
                    return Ok(self.reject(
                        &tx_hash_hex,
                        payload.tx_type,
                        &lock.escrow_id,
                        "amount must be > 0",
                    )?);
                }
                if lock.currency.is_empty() {
                    return Ok(self.reject(
                        &tx_hash_hex,
                        payload.tx_type,
                        &lock.escrow_id,
                        "currency required",
                    )?);
                }
                if lock.expiry <= now {
                    return Ok(self.reject(
                        &tx_hash_hex,
                        payload.tx_type,
                        &lock.escrow_id,
                        "expiry must be in the future",
                    )?);
                }
                if lock.dispute_window_sec == 0 {
                    return Ok(self.reject(
                        &tx_hash_hex,
                        payload.tx_type,
                        &lock.escrow_id,
                        "dispute window must be > 0",
                    )?);
                }
                if payload.sender != lock.payer {
                    return Ok(self.reject(
                        &tx_hash_hex,
                        payload.tx_type,
                        &lock.escrow_id,
                        "sender must be payer",
                    )?);
                }
                let release_condition_bytes = encode_canonical(&lock.release_condition)?;
                let record = EscrowRecord {
                    escrow_id: lock.escrow_id.clone(),
                    payer: lock.payer.clone(),
                    payee: lock.payee.clone(),
                    amount: lock.amount,
                    currency: lock.currency.clone(),
                    release_condition_cbor_hex: hex::encode(release_condition_bytes),
                    dispute_window_sec: lock.dispute_window_sec,
                    expiry: lock.expiry,
                    status: EscrowStatus::Locked,
                    locked_at: now,
                    disputed_at: None,
                    resolved_at: None,
                    outcome: None,
                    split_amount_to_payee: None,
                    last_tx_hash: tx_hash_hex.clone(),
                };
                self.records.insert(lock.escrow_id.clone(), record);
                self.persist_state()?;
                self.append_event(&tx_hash_hex, payload.tx_type, &lock.escrow_id, "locked")?;
                decision.receipt = Some(ReceiptSpec {
                    event_type: EV_PAYMENT_SENT,
                    details: escrow_details_lock(&lock),
                    economics,
                });
            }
            TX_ESCROW_RELEASE => {
                let release = parse_escrow_release_payload(&payload.payload)
                    .context("parse escrow release")?;
                let (details, escrow_id) = {
                    let record = self.records.get_mut(&release.escrow_id);
                    let record = match record {
                        Some(record) => record,
                        None => {
                            return Ok(self.reject(
                                &tx_hash_hex,
                                payload.tx_type,
                                &release.escrow_id,
                                "escrow not found",
                            )?);
                        }
                    };
                    if !matches!(record.status, EscrowStatus::Locked) {
                        return Ok(self.reject(
                            &tx_hash_hex,
                            payload.tx_type,
                            &release.escrow_id,
                            "escrow not locked",
                        )?);
                    }
                    if payload.sender != record.payer {
                        return Ok(self.reject(
                            &tx_hash_hex,
                            payload.tx_type,
                            &release.escrow_id,
                            "sender must be payer",
                        )?);
                    }
                    if now >= record.expiry {
                        return Ok(self.reject(
                            &tx_hash_hex,
                            payload.tx_type,
                            &release.escrow_id,
                            "escrow expired",
                        )?);
                    }
                    record.status = EscrowStatus::Released;
                    record.resolved_at = Some(now);
                    record.outcome = Some(EscrowOutcome::Release);
                    record.last_tx_hash = tx_hash_hex.clone();
                    let details = escrow_details_release(record, &release.evidence_receipt_hash);
                    (details, record.escrow_id.clone())
                };
                self.persist_state()?;
                self.append_event(&tx_hash_hex, payload.tx_type, &escrow_id, "released")?;
                decision.receipt = Some(ReceiptSpec {
                    event_type: EV_PAYMENT_SENT,
                    details,
                    economics,
                });
            }
            TX_ESCROW_DISPUTE => {
                let dispute = parse_escrow_dispute_payload(&payload.payload)
                    .context("parse escrow dispute")?;
                let (details, escrow_id) = {
                    let record = self.records.get_mut(&dispute.escrow_id);
                    let record = match record {
                        Some(record) => record,
                        None => {
                            return Ok(self.reject(
                                &tx_hash_hex,
                                payload.tx_type,
                                &dispute.escrow_id,
                                "escrow not found",
                            )?);
                        }
                    };
                    if !matches!(record.status, EscrowStatus::Locked) {
                        return Ok(self.reject(
                            &tx_hash_hex,
                            payload.tx_type,
                            &dispute.escrow_id,
                            "escrow not locked",
                        )?);
                    }
                    if payload.sender != record.payer && payload.sender != record.payee {
                        return Ok(self.reject(
                            &tx_hash_hex,
                            payload.tx_type,
                            &dispute.escrow_id,
                            "sender must be payer or payee",
                        )?);
                    }
                    let dispute_deadline =
                        record.locked_at.saturating_add(record.dispute_window_sec);
                    if now > dispute_deadline {
                        return Ok(self.reject(
                            &tx_hash_hex,
                            payload.tx_type,
                            &dispute.escrow_id,
                            "dispute window closed",
                        )?);
                    }
                    if now >= record.expiry {
                        return Ok(self.reject(
                            &tx_hash_hex,
                            payload.tx_type,
                            &dispute.escrow_id,
                            "escrow expired",
                        )?);
                    }
                    record.status = EscrowStatus::Disputed;
                    record.disputed_at = Some(now);
                    record.last_tx_hash = tx_hash_hex.clone();
                    let details = escrow_details_dispute(
                        record,
                        &dispute.reason,
                        &dispute.evidence_anchor_or_receipt,
                    );
                    (details, record.escrow_id.clone())
                };
                self.persist_state()?;
                self.append_event(&tx_hash_hex, payload.tx_type, &escrow_id, "disputed")?;
                decision.receipt = Some(ReceiptSpec {
                    event_type: EV_GOVERNANCE_EVENT,
                    details,
                    economics,
                });
            }
            TX_ESCROW_RESOLVE => {
                let resolve = parse_escrow_resolve_payload(&payload.payload)
                    .context("parse escrow resolve")?;
                let (details, escrow_id) = {
                    let record = self.records.get_mut(&resolve.escrow_id);
                    let record = match record {
                        Some(record) => record,
                        None => {
                            return Ok(self.reject(
                                &tx_hash_hex,
                                payload.tx_type,
                                &resolve.escrow_id,
                                "escrow not found",
                            )?);
                        }
                    };
                    if !matches!(record.status, EscrowStatus::Disputed) {
                        return Ok(self.reject(
                            &tx_hash_hex,
                            payload.tx_type,
                            &resolve.escrow_id,
                            "escrow not disputed",
                        )?);
                    }
                    if !self.arbitrators.contains(&payload.sender) {
                        return Ok(self.reject(
                            &tx_hash_hex,
                            payload.tx_type,
                            &resolve.escrow_id,
                            "sender not authorized arbitrator",
                        )?);
                    }
                    if now >= record.expiry {
                        return Ok(self.reject(
                            &tx_hash_hex,
                            payload.tx_type,
                            &resolve.escrow_id,
                            "escrow expired",
                        )?);
                    }
                    match resolve.outcome {
                        0 => {
                            record.status = EscrowStatus::Released;
                            record.outcome = Some(EscrowOutcome::Release);
                        }
                        1 => {
                            record.status = EscrowStatus::Refunded;
                            record.outcome = Some(EscrowOutcome::Refund);
                        }
                        2 => {
                            let split_amount = resolve
                                .split_amount_to_payee
                                .ok_or_else(|| anyhow::anyhow!("split outcome missing amount"))?;
                            if split_amount > record.amount {
                                return Ok(self.reject(
                                    &tx_hash_hex,
                                    payload.tx_type,
                                    &resolve.escrow_id,
                                    "split exceeds escrow amount",
                                )?);
                            }
                            record.status = EscrowStatus::Split;
                            record.outcome = Some(EscrowOutcome::Split);
                            record.split_amount_to_payee = Some(split_amount);
                        }
                        3 => {
                            record.status = EscrowStatus::Slashed;
                            record.outcome = Some(EscrowOutcome::Slash);
                        }
                        _ => {
                            return Ok(self.reject(
                                &tx_hash_hex,
                                payload.tx_type,
                                &resolve.escrow_id,
                                "invalid resolve outcome",
                            )?);
                        }
                    }
                    record.resolved_at = Some(now);
                    record.last_tx_hash = tx_hash_hex.clone();
                    let details = escrow_details_resolve(record);
                    (details, record.escrow_id.clone())
                };
                self.persist_state()?;
                self.append_event(&tx_hash_hex, payload.tx_type, &escrow_id, "resolved")?;
                decision.receipt = Some(ReceiptSpec {
                    event_type: EV_GOVERNANCE_EVENT,
                    details,
                    economics,
                });
            }
            _ => {
                self.append_event(&tx_hash_hex, payload.tx_type, "n/a", "ignored")?;
            }
        }
        decision.accept = true;
        Ok(decision)
    }

    fn reject(
        &mut self,
        tx_hash_hex: &str,
        tx_type: u64,
        escrow_id: &str,
        reason: &str,
    ) -> Result<TxDecision> {
        self.append_event(tx_hash_hex, tx_type, escrow_id, reason)?;
        Ok(TxDecision {
            accept: false,
            reason: Some(reason.to_string()),
            receipt: None,
        })
    }

    fn expire_if_needed(&mut self, now: u64) -> Result<()> {
        let mut changed = false;
        for record in self.records.values_mut() {
            if matches!(record.status, EscrowStatus::Locked | EscrowStatus::Disputed)
                && now >= record.expiry
            {
                record.status = EscrowStatus::Expired;
                record.resolved_at = Some(now);
                record.outcome = Some(EscrowOutcome::Refund);
                changed = true;
            }
        }
        if changed {
            self.persist_state()?;
        }
        Ok(())
    }

    fn persist_state(&self) -> Result<()> {
        let data = serde_json::to_vec_pretty(&self.records).context("encode escrow state")?;
        fs::write(&self.state_path, data)
            .with_context(|| format!("write escrow state {}", self.state_path.display()))?;
        Ok(())
    }

    fn append_event(
        &self,
        tx_hash_hex: &str,
        tx_type: u64,
        escrow_id: &str,
        outcome: &str,
    ) -> Result<()> {
        let event = EscrowEvent {
            tx_hash_hex: tx_hash_hex.to_string(),
            tx_type,
            escrow_id: escrow_id.to_string(),
            outcome: outcome.to_string(),
        };
        let line = serde_json::to_string(&event).context("encode escrow event")?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
            .with_context(|| format!("open escrow log {}", self.log_path.display()))?;
        writeln!(file, "{line}").context("append escrow log")?;
        Ok(())
    }
}

#[derive(Serialize)]
struct EscrowEvent {
    tx_hash_hex: String,
    tx_type: u64,
    escrow_id: String,
    outcome: String,
}

fn load_state(path: &Path) -> Result<HashMap<String, EscrowRecord>> {
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let data = fs::read(path).with_context(|| format!("read escrow state {}", path.display()))?;
    let records = serde_json::from_slice(&data).context("parse escrow state")?;
    Ok(records)
}

fn load_identity_state(path: &Path) -> Result<(HashMap<String, IdentityRecord>, HashSet<String>)> {
    if !path.exists() {
        return Ok((HashMap::new(), HashSet::new()));
    }
    let data = fs::read(path).with_context(|| format!("read identity state {}", path.display()))?;
    let snapshot: IdentityStateSnapshot =
        serde_json::from_slice(&data).context("parse identity state")?;
    Ok((snapshot.records, snapshot.revocations))
}

fn load_budget_state(path: &Path) -> Result<HashMap<String, HashMap<String, BudgetWindow>>> {
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let data = fs::read(path).with_context(|| format!("read budget state {}", path.display()))?;
    let snapshot: BudgetStateSnapshot =
        serde_json::from_slice(&data).context("parse budget state")?;
    Ok(snapshot.windows)
}

fn load_skill_state(path: &Path) -> Result<HashMap<String, SkillRecord>> {
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let data = fs::read(path).with_context(|| format!("read skill state {}", path.display()))?;
    let snapshot: SkillStateSnapshot =
        serde_json::from_slice(&data).context("parse skill state")?;
    Ok(snapshot.records)
}

fn load_work_state(
    path: &Path,
) -> Result<(
    HashMap<String, WorkOfferRecord>,
    HashMap<String, WorkAgreementRecord>,
)> {
    if !path.exists() {
        return Ok((HashMap::new(), HashMap::new()));
    }
    let data = fs::read(path).with_context(|| format!("read work state {}", path.display()))?;
    let snapshot: WorkStateSnapshot = serde_json::from_slice(&data).context("parse work state")?;
    Ok((snapshot.offers, snapshot.agreements))
}

fn build_caps(list: &[BudgetCurrencyCap]) -> Result<HashMap<String, u64>> {
    let mut caps = HashMap::new();
    for entry in list {
        if entry.currency.is_empty() {
            anyhow::bail!("budget cap currency required");
        }
        if entry.max_amount == 0 {
            anyhow::bail!("budget cap max_amount must be > 0");
        }
        if caps
            .insert(entry.currency.clone(), entry.max_amount)
            .is_some()
        {
            anyhow::bail!("duplicate budget cap for {}", entry.currency);
        }
    }
    Ok(caps)
}

fn skill_details_publish(
    skill_id: &str,
    author: &str,
    version: &str,
    manifest_hash: &[u8; 32],
) -> CborValue {
    CborValue::Map(vec![
        (
            CborValue::Unsigned(0),
            CborValue::Text("skill.publish".to_string()),
        ),
        (
            CborValue::Unsigned(1),
            CborValue::Text(skill_id.to_string()),
        ),
        (CborValue::Unsigned(2), CborValue::Text(author.to_string())),
        (CborValue::Unsigned(3), CborValue::Text(version.to_string())),
        (
            CborValue::Unsigned(4),
            CborValue::Bytes(manifest_hash.to_vec()),
        ),
    ])
}

fn skill_details_update(
    skill_id: &str,
    author: &str,
    version: &str,
    prev_hash: &[u8],
    new_hash: &[u8; 32],
) -> CborValue {
    CborValue::Map(vec![
        (
            CborValue::Unsigned(0),
            CborValue::Text("skill.update".to_string()),
        ),
        (
            CborValue::Unsigned(1),
            CborValue::Text(skill_id.to_string()),
        ),
        (CborValue::Unsigned(2), CborValue::Text(author.to_string())),
        (CborValue::Unsigned(3), CborValue::Text(version.to_string())),
        (CborValue::Unsigned(4), CborValue::Bytes(prev_hash.to_vec())),
        (CborValue::Unsigned(5), CborValue::Bytes(new_hash.to_vec())),
    ])
}

fn skill_details_revoke(
    skill_id: &str,
    author: &str,
    manifest_hash: &[u8],
    reason: &str,
) -> CborValue {
    CborValue::Map(vec![
        (
            CborValue::Unsigned(0),
            CborValue::Text("skill.revoke".to_string()),
        ),
        (
            CborValue::Unsigned(1),
            CborValue::Text(skill_id.to_string()),
        ),
        (CborValue::Unsigned(2), CborValue::Text(author.to_string())),
        (
            CborValue::Unsigned(3),
            CborValue::Bytes(manifest_hash.to_vec()),
        ),
        (CborValue::Unsigned(4), CborValue::Text(reason.to_string())),
    ])
}

fn work_details_offer_publish(offer_id: &str, issuer: &str, offer_hash: &[u8; 32]) -> CborValue {
    CborValue::Map(vec![
        (
            CborValue::Unsigned(0),
            CborValue::Text("work.offer.publish".to_string()),
        ),
        (
            CborValue::Unsigned(1),
            CborValue::Text(offer_id.to_string()),
        ),
        (CborValue::Unsigned(2), CborValue::Text(issuer.to_string())),
        (
            CborValue::Unsigned(3),
            CborValue::Bytes(offer_hash.to_vec()),
        ),
    ])
}

fn work_details_agreement_publish(
    agreement_id: &str,
    issuer: &str,
    agreement_hash: &[u8; 32],
) -> CborValue {
    CborValue::Map(vec![
        (
            CborValue::Unsigned(0),
            CborValue::Text("work.agreement.publish".to_string()),
        ),
        (
            CborValue::Unsigned(1),
            CborValue::Text(agreement_id.to_string()),
        ),
        (CborValue::Unsigned(2), CborValue::Text(issuer.to_string())),
        (
            CborValue::Unsigned(3),
            CborValue::Bytes(agreement_hash.to_vec()),
        ),
    ])
}

fn work_details_agreement_update(
    agreement_id: &str,
    issuer: &str,
    prev_hash: &[u8],
    new_hash: &[u8; 32],
) -> CborValue {
    CborValue::Map(vec![
        (
            CborValue::Unsigned(0),
            CborValue::Text("work.agreement.update".to_string()),
        ),
        (
            CborValue::Unsigned(1),
            CborValue::Text(agreement_id.to_string()),
        ),
        (CborValue::Unsigned(2), CborValue::Text(issuer.to_string())),
        (CborValue::Unsigned(3), CborValue::Bytes(prev_hash.to_vec())),
        (CborValue::Unsigned(4), CborValue::Bytes(new_hash.to_vec())),
    ])
}

fn work_details_agreement_close(
    agreement_id: &str,
    actor: &str,
    agreement_hash: &[u8],
    reason: &str,
) -> CborValue {
    CborValue::Map(vec![
        (
            CborValue::Unsigned(0),
            CborValue::Text("work.agreement.close".to_string()),
        ),
        (
            CborValue::Unsigned(1),
            CborValue::Text(agreement_id.to_string()),
        ),
        (CborValue::Unsigned(2), CborValue::Text(actor.to_string())),
        (
            CborValue::Unsigned(3),
            CborValue::Bytes(agreement_hash.to_vec()),
        ),
        (CborValue::Unsigned(4), CborValue::Text(reason.to_string())),
    ])
}

fn escrow_details_lock(lock: &anetsdk::EscrowLockPayload) -> CborValue {
    CborValue::Map(vec![
        (
            CborValue::Unsigned(0),
            CborValue::Text("escrow.lock".to_string()),
        ),
        (
            CborValue::Unsigned(1),
            CborValue::Text(lock.escrow_id.clone()),
        ),
        (CborValue::Unsigned(2), CborValue::Text(lock.payer.clone())),
        (CborValue::Unsigned(3), CborValue::Text(lock.payee.clone())),
        (CborValue::Unsigned(4), CborValue::Unsigned(lock.amount)),
        (
            CborValue::Unsigned(5),
            CborValue::Text(lock.currency.clone()),
        ),
        (CborValue::Unsigned(6), CborValue::Unsigned(lock.expiry)),
    ])
}

fn escrow_details_release(record: &EscrowRecord, evidence_hash: &[u8]) -> CborValue {
    CborValue::Map(vec![
        (
            CborValue::Unsigned(0),
            CborValue::Text("escrow.release".to_string()),
        ),
        (
            CborValue::Unsigned(1),
            CborValue::Text(record.escrow_id.clone()),
        ),
        (
            CborValue::Unsigned(2),
            CborValue::Text(record.payer.clone()),
        ),
        (
            CborValue::Unsigned(3),
            CborValue::Text(record.payee.clone()),
        ),
        (CborValue::Unsigned(4), CborValue::Unsigned(record.amount)),
        (
            CborValue::Unsigned(5),
            CborValue::Text(record.currency.clone()),
        ),
        (
            CborValue::Unsigned(6),
            CborValue::Bytes(evidence_hash.to_vec()),
        ),
    ])
}

fn escrow_details_dispute(record: &EscrowRecord, reason: &str, evidence: &[u8]) -> CborValue {
    CborValue::Map(vec![
        (
            CborValue::Unsigned(0),
            CborValue::Text("escrow.dispute".to_string()),
        ),
        (
            CborValue::Unsigned(1),
            CborValue::Text(record.escrow_id.clone()),
        ),
        (CborValue::Unsigned(2), CborValue::Text(reason.to_string())),
        (CborValue::Unsigned(3), CborValue::Bytes(evidence.to_vec())),
    ])
}

fn escrow_details_resolve(record: &EscrowRecord) -> CborValue {
    let mut entries = vec![
        (
            CborValue::Unsigned(0),
            CborValue::Text("escrow.resolve".to_string()),
        ),
        (
            CborValue::Unsigned(1),
            CborValue::Text(record.escrow_id.clone()),
        ),
        (
            CborValue::Unsigned(2),
            CborValue::Text(record.payer.clone()),
        ),
        (
            CborValue::Unsigned(3),
            CborValue::Text(record.payee.clone()),
        ),
        (CborValue::Unsigned(4), CborValue::Unsigned(record.amount)),
        (
            CborValue::Unsigned(5),
            CborValue::Text(record.currency.clone()),
        ),
    ];
    if let Some(outcome) = &record.outcome {
        let outcome_val = match outcome {
            EscrowOutcome::Release => 0u64,
            EscrowOutcome::Refund => 1u64,
            EscrowOutcome::Split => 2u64,
            EscrowOutcome::Slash => 3u64,
        };
        entries.push((CborValue::Unsigned(6), CborValue::Unsigned(outcome_val)));
    }
    if let Some(split_amount) = record.split_amount_to_payee {
        entries.push((CborValue::Unsigned(7), CborValue::Unsigned(split_amount)));
    }
    CborValue::Map(entries)
}

fn identity_details_register(payload: &anetsdk::IdentityRegisterPayload) -> CborValue {
    CborValue::Map(vec![
        (
            CborValue::Unsigned(0),
            CborValue::Text("identity.register".to_string()),
        ),
        (
            CborValue::Unsigned(1),
            CborValue::Text(payload.agent_id.clone()),
        ),
        (
            CborValue::Unsigned(2),
            CborValue::Bytes(payload.pk_ed25519.clone()),
        ),
        (
            CborValue::Unsigned(3),
            CborValue::Bytes(payload.pk_x25519.clone()),
        ),
        (CborValue::Unsigned(4), CborValue::Unsigned(payload.created)),
    ])
}

fn identity_details_rotate(payload: &anetsdk::IdentityRotatePayload) -> CborValue {
    CborValue::Map(vec![
        (
            CborValue::Unsigned(0),
            CborValue::Text("identity.rotate".to_string()),
        ),
        (
            CborValue::Unsigned(1),
            CborValue::Text(payload.agent_id.clone()),
        ),
        (
            CborValue::Unsigned(2),
            CborValue::Bytes(payload.pk_ed25519.clone()),
        ),
        (
            CborValue::Unsigned(3),
            CborValue::Bytes(payload.pk_x25519.clone()),
        ),
        (CborValue::Unsigned(4), CborValue::Unsigned(payload.ts)),
    ])
}

fn identity_details_revoke(payload: &anetsdk::CredentialRevokePayload) -> CborValue {
    CborValue::Map(vec![
        (
            CborValue::Unsigned(0),
            CborValue::Text("credential.revoke".to_string()),
        ),
        (
            CborValue::Unsigned(1),
            CborValue::Text(payload.issuer.clone()),
        ),
        (
            CborValue::Unsigned(2),
            CborValue::Bytes(payload.credential_id_hash.clone()),
        ),
        (CborValue::Unsigned(3), CborValue::Unsigned(payload.ts)),
    ])
}
