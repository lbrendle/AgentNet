use crate::models::{
    AgentProfileIngest, AgentRecordIngest, CommunityRecordIngest, IdentityStateIngest, ReceiptIngest,
    ServiceRecordIngest, SkillManifestIngest, SkillRegistryStateIngest, WorkAgreementIngest,
    WorkOfferIngest, WorkRegistryStateIngest,
};
use crate::state::{
    IdentityEntry, IdentityState, IndexState, SkillRegistryRecord, SkillRegistryState,
    WorkAgreementRegistryRecord, WorkOfferRegistryRecord, WorkRegistryState,
};
use crate::util::cbor_to_json_value;
use anetsdk::{
    decode_canonical, parse_agent_profile, parse_agent_record, parse_community_record,
    parse_receipt_payload, parse_service_record, sha256, verify_agent_profile, verify_agent_record,
    verify_community_record, verify_ed25519_hash, verify_service_record, verify_skill_manifest,
    verify_work_agreement, verify_work_offer, ReceiptPayload,
};
use anyhow::{anyhow, Context, Result};
use hex::FromHex;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

pub async fn ingest_identity_state(
    state: Arc<IndexState>,
    payload: IdentityStateIngest,
) -> Result<()> {
    let snapshot: crate::models::IdentityStateSnapshot =
        serde_json::from_str(&payload.json).context("parse identity state json")?;
    let mut records = std::collections::HashMap::new();
    for (key, record) in snapshot.records {
        if key != record.did {
            return Err(anyhow!("identity key mismatch for {}", record.did));
        }
        let pk_ed25519 = decode_hex_key("pk_ed25519_hex", &record.pk_ed25519_hex)?;
        let pk_x25519 = decode_hex_key("pk_x25519_hex", &record.pk_x25519_hex)?;
        if pk_ed25519.len() != 32 || pk_x25519.len() != 32 {
            return Err(anyhow!("identity key length mismatch for {}", record.did));
        }
        records.insert(
            record.did.clone(),
            IdentityEntry {
                did: record.did,
                pk_ed25519,
                pk_x25519,
                created: record.created,
                updated: record.updated,
            },
        );
    }
    let identity_state = IdentityState {
        records,
        revocations: snapshot.revocations,
    };
    state.set_identity_state(identity_state).await
}

pub async fn ingest_skill_registry_state(
    state: Arc<IndexState>,
    payload: SkillRegistryStateIngest,
) -> Result<()> {
    state.ensure_identity_loaded().await?;
    let snapshot: crate::models::SkillStateSnapshot =
        serde_json::from_str(&payload.json).context("parse skill registry state json")?;
    let mut records = std::collections::HashMap::new();
    for (key, record) in snapshot.records {
        if key != record.skill_id {
            return Err(anyhow!(
                "skill registry key mismatch for {}",
                record.skill_id
            ));
        }
        records.insert(
            record.skill_id.clone(),
            SkillRegistryRecord {
                skill_id: record.skill_id,
                author: record.author,
                manifest_hash_hex: record.manifest_hash_hex,
                manifest_hex: record.manifest_hex,
                revoked: record.revoked,
                revoked_at: record.revoked_at,
                revocation_reason: record.revocation_reason,
                published_at: record.published_at,
                updated_at: record.updated_at,
            },
        );
    }
    let skill_state = SkillRegistryState { records };
    state.set_skill_registry_state(skill_state).await
}

pub async fn ingest_work_registry_state(
    state: Arc<IndexState>,
    payload: WorkRegistryStateIngest,
) -> Result<()> {
    state.ensure_identity_loaded().await?;
    let snapshot: crate::models::WorkStateSnapshot =
        serde_json::from_str(&payload.json).context("parse work registry state json")?;
    let mut offers = std::collections::HashMap::new();
    for (key, record) in snapshot.offers {
        if key != record.offer_id {
            return Err(anyhow!("work offer key mismatch for {}", record.offer_id));
        }
        offers.insert(
            record.offer_id.clone(),
            WorkOfferRegistryRecord {
                offer_id: record.offer_id,
                issuer: record.issuer,
                offer_hash_hex: record.offer_hash_hex,
                offer_hex: record.offer_hex,
                published_at: record.published_at,
            },
        );
    }
    let mut agreements = std::collections::HashMap::new();
    for (key, record) in snapshot.agreements {
        if key != record.agreement_id {
            return Err(anyhow!(
                "work agreement key mismatch for {}",
                record.agreement_id
            ));
        }
        agreements.insert(
            record.agreement_id.clone(),
            WorkAgreementRegistryRecord {
                agreement_id: record.agreement_id,
                issuer: record.issuer,
                agreement_hash_hex: record.agreement_hash_hex,
                agreement_hex: record.agreement_hex,
                closed: record.closed,
                closed_at: record.closed_at,
                close_reason: record.close_reason,
                published_at: record.published_at,
                updated_at: record.updated_at,
            },
        );
    }
    let work_state = WorkRegistryState { offers, agreements };
    state.set_work_registry_state(work_state).await
}

pub async fn ingest_agent_record(state: Arc<IndexState>, payload: AgentRecordIngest) -> Result<()> {
    state.ensure_identity_loaded().await?;
    let record_bytes = decode_hex("agent_record", &payload.cbor_hex)?;
    let value = decode_canonical(&record_bytes).context("decode agent record cbor")?;
    let record = parse_agent_record(&value).context("parse agent record")?;
    let agent_id = record.payload.agent_id.clone();
    let pk = resolve_pubkey(
        &state,
        &agent_id,
        payload.public_key_hex.as_deref(),
        false,
    )
    .await?;
    if !record.payload.agent_pubkeys.iter().any(|k| k == &pk) {
        return Err(anyhow!("agent record missing active pubkey"));
    }
    let verified_payload =
        verify_agent_record(&record_bytes, &pk).context("verify agent record signature")?;
    ensure_not_expired(verified_payload.expires)?;
    let record_hex = hex::encode(&record_bytes);
    let now = now_ts();
    let mut db = state.db_mut().await;
    db.upsert_agent(&verified_payload, &record_hex, now)?;
    Ok(())
}

pub async fn ingest_agent_profile(
    state: Arc<IndexState>,
    payload: AgentProfileIngest,
) -> Result<()> {
    if payload.public_key_hex.is_none() {
        state.ensure_identity_loaded().await?;
    }
    let record_bytes = decode_hex("agent_profile", &payload.cbor_hex)?;
    let value = decode_canonical(&record_bytes).context("decode agent profile cbor")?;
    let record = parse_agent_profile(&value).context("parse agent profile")?;
    let agent_id = record.payload.agent_id.clone();
    let pk = resolve_pubkey(
        &state,
        &agent_id,
        payload.public_key_hex.as_deref(),
        true,
    )
    .await?;
    let verified_payload =
        verify_agent_profile(&record_bytes, &pk).context("verify agent profile signature")?;
    ensure_not_expired(verified_payload.expires)?;
    let record_hex = hex::encode(&record_bytes);
    let now = now_ts();
    let mut db = state.db_mut().await;
    db.upsert_agent_profile(&verified_payload, &record_hex, now)?;
    Ok(())
}

pub async fn ingest_service_record(
    state: Arc<IndexState>,
    payload: ServiceRecordIngest,
) -> Result<()> {
    state.ensure_identity_loaded().await?;
    let record_bytes = decode_hex("service_record", &payload.cbor_hex)?;
    let value = decode_canonical(&record_bytes).context("decode service record cbor")?;
    let record = parse_service_record(&value).context("parse service record")?;
    let provider_id = record.payload.provider_id.clone();
    let pk = resolve_pubkey(
        &state,
        &provider_id,
        payload.public_key_hex.as_deref(),
        false,
    )
    .await?;
    let verified_payload =
        verify_service_record(&record_bytes, &pk).context("verify service record signature")?;
    ensure_not_expired(verified_payload.expires)?;
    let record_hex = hex::encode(&record_bytes);
    let now = now_ts();
    let mut db = state.db_mut().await;
    db.upsert_service(&verified_payload, &record_hex, now)?;
    Ok(())
}

pub async fn ingest_community_record(
    state: Arc<IndexState>,
    payload: CommunityRecordIngest,
) -> Result<()> {
    state.ensure_identity_loaded().await?;
    let record_bytes = decode_hex("community_record", &payload.cbor_hex)?;
    let value = decode_canonical(&record_bytes).context("decode community record cbor")?;
    let record = parse_community_record(&value).context("parse community record")?;
    let controller = record.payload.controller.clone();
    let pk = resolve_pubkey(
        &state,
        &controller,
        payload.public_key_hex.as_deref(),
        false,
    )
    .await?;
    let verified_payload =
        verify_community_record(&record_bytes, &pk).context("verify community record signature")?;
    ensure_not_expired(verified_payload.expires)?;
    let record_hex = hex::encode(&record_bytes);
    let now = now_ts();
    let mut db = state.db_mut().await;
    db.upsert_community(&verified_payload, &record_hex, now)?;
    Ok(())
}

pub async fn ingest_skill_manifest(
    state: Arc<IndexState>,
    payload: SkillManifestIngest,
) -> Result<()> {
    state.ensure_identity_loaded().await?;
    let manifest_bytes = decode_hex("skill_manifest", &payload.cbor_hex)?;
    let value = decode_canonical(&manifest_bytes).context("decode skill manifest cbor")?;
    let manifest = anetsdk::parse_skill_manifest(&value).context("parse skill manifest")?;
    let author = manifest.payload.author.clone();
    let pk = resolve_pubkey(
        &state,
        &author,
        payload.public_key_hex.as_deref(),
        false,
    )
    .await?;
    let verified_payload =
        verify_skill_manifest(&manifest_bytes, &pk).context("verify skill manifest signature")?;
    let manifest_hash_hex = hex::encode(sha256(&manifest_bytes));
    let registry_record = state
        .skill_registry_record(&verified_payload.skill_id)
        .await
        .ok_or_else(|| anyhow!("skill registry record not found"))?;
    if registry_record.author != verified_payload.author {
        return Err(anyhow!("skill registry author mismatch"));
    }
    if registry_record.revoked {
        return Err(anyhow!("skill is revoked"));
    }
    if registry_record.manifest_hash_hex != manifest_hash_hex {
        return Err(anyhow!("skill manifest hash mismatch with registry"));
    }
    let mut db = state.db_mut().await;
    db.upsert_skill_manifest(
        &verified_payload,
        &hex::encode(&manifest_bytes),
        &manifest_hash_hex,
        Some(&registry_record),
    )?;
    Ok(())
}

pub async fn ingest_experience_manifest(
    state: Arc<IndexState>,
    payload: SkillManifestIngest,
) -> Result<()> {
    if payload.public_key_hex.is_none() {
        state.ensure_identity_loaded().await?;
    }
    let manifest_bytes = decode_hex("skill_manifest", &payload.cbor_hex)?;
    let value = decode_canonical(&manifest_bytes).context("decode skill manifest cbor")?;
    let manifest = anetsdk::parse_skill_manifest(&value).context("parse skill manifest")?;
    let author = manifest.payload.author.clone();
    let pk = resolve_pubkey(
        &state,
        &author,
        payload.public_key_hex.as_deref(),
        true,
    )
    .await?;
    let verified_payload =
        verify_skill_manifest(&manifest_bytes, &pk).context("verify skill manifest signature")?;
    let manifest_hash_hex = hex::encode(sha256(&manifest_bytes));
    let registry_record = state
        .skill_registry_record(&verified_payload.skill_id)
        .await;
    if let Some(registry_record) = registry_record.as_ref() {
        if registry_record.author != verified_payload.author {
            return Err(anyhow!("skill registry author mismatch"));
        }
        if registry_record.revoked {
            return Err(anyhow!("skill is revoked"));
        }
        if registry_record.manifest_hash_hex != manifest_hash_hex {
            return Err(anyhow!("skill manifest hash mismatch with registry"));
        }
    }
    let mut db = state.db_mut().await;
    db.upsert_skill_manifest(
        &verified_payload,
        &hex::encode(&manifest_bytes),
        &manifest_hash_hex,
        registry_record.as_ref(),
    )?;
    Ok(())
}

pub async fn ingest_work_offer(state: Arc<IndexState>, payload: WorkOfferIngest) -> Result<()> {
    state.ensure_identity_loaded().await?;
    let offer_bytes = decode_hex("work_offer", &payload.cbor_hex)?;
    let value = decode_canonical(&offer_bytes).context("decode work offer cbor")?;
    let offer = anetsdk::parse_work_offer(&value).context("parse work offer")?;
    let issuer = offer.payload.issuer.clone();
    let pk = resolve_pubkey(
        &state,
        &issuer,
        payload.public_key_hex.as_deref(),
        false,
    )
    .await?;
    let verified_payload =
        verify_work_offer(&offer_bytes, &pk).context("verify work offer signature")?;
    if verified_payload.exp <= now_ts() {
        return Err(anyhow!("work offer expired"));
    }
    let offer_hash_hex = hex::encode(sha256(&offer_bytes));
    let registry_record = state
        .work_offer_registry_record(&verified_payload.offer_id)
        .await
        .ok_or_else(|| anyhow!("work offer registry record not found"))?;
    if registry_record.issuer != verified_payload.issuer {
        return Err(anyhow!("work offer issuer mismatch"));
    }
    if registry_record.offer_hash_hex != offer_hash_hex {
        return Err(anyhow!("work offer hash mismatch with registry"));
    }
    let mut db = state.db_mut().await;
    db.upsert_work_offer(
        &verified_payload,
        &hex::encode(&offer_bytes),
        &offer_hash_hex,
        Some(&registry_record),
    )?;
    Ok(())
}

pub async fn ingest_work_agreement(
    state: Arc<IndexState>,
    payload: WorkAgreementIngest,
) -> Result<()> {
    state.ensure_identity_loaded().await?;
    let agreement_bytes = decode_hex("work_agreement", &payload.cbor_hex)?;
    let value = decode_canonical(&agreement_bytes).context("decode work agreement cbor")?;
    let agreement = anetsdk::parse_work_agreement(&value).context("parse work agreement")?;
    let issuer = agreement.payload.issuer.clone();
    let pk = resolve_pubkey(
        &state,
        &issuer,
        payload.public_key_hex.as_deref(),
        false,
    )
    .await?;
    let verified_payload =
        verify_work_agreement(&agreement_bytes, &pk).context("verify work agreement signature")?;
    let agreement_hash_hex = hex::encode(sha256(&agreement_bytes));
    let registry_record = state
        .work_agreement_registry_record(&verified_payload.agreement_id)
        .await
        .ok_or_else(|| anyhow!("work agreement registry record not found"))?;
    if registry_record.issuer != verified_payload.issuer {
        return Err(anyhow!("work agreement issuer mismatch"));
    }
    if registry_record.agreement_hash_hex != agreement_hash_hex {
        return Err(anyhow!("work agreement hash mismatch with registry"));
    }
    let mut db = state.db_mut().await;
    db.upsert_work_agreement(
        &verified_payload,
        &hex::encode(&agreement_bytes),
        &agreement_hash_hex,
        Some(&registry_record),
    )?;
    Ok(())
}

pub async fn ingest_receipt(state: Arc<IndexState>, payload: ReceiptIngest) -> Result<()> {
    state.ensure_identity_loaded().await?;
    let receipt_payload_bytes = decode_hex("receipt_payload", &payload.payload_hex)?;
    let signature_bytes = decode_hex("receipt_signature", &payload.signature_hex)?;
    if signature_bytes.len() != 64 {
        return Err(anyhow!("receipt signature must be 64 bytes"));
    }
    let value = decode_canonical(&receipt_payload_bytes).context("decode receipt payload cbor")?;
    let receipt_payload = parse_receipt_payload(&value).context("parse receipt payload")?;
    let pk = resolve_pubkey(
        &state,
        &receipt_payload.actor,
        payload.public_key_hex.as_deref(),
        false,
    )
    .await?;
    let receipt_hash = sha256(&receipt_payload_bytes);
    verify_ed25519_hash(&pk, &receipt_hash, &signature_bytes)
        .context("verify receipt signature")?;
    enforce_receipt_sequence(&state, &receipt_payload).await?;
    let event_json = serde_json::to_string(&cbor_to_json_value(&receipt_payload.event))?;
    let auth_json = serde_json::to_string(&cbor_to_json_value(&receipt_payload.auth))?;
    let economics_json = serde_json::to_string(&cbor_to_json_value(&receipt_payload.economics))?;
    let mut db = state.db_mut().await;
    db.insert_receipt(
        &receipt_payload,
        &hex::encode(receipt_hash),
        &hex::encode(signature_bytes),
        &event_json,
        &auth_json,
        &economics_json,
    )?;
    Ok(())
}

async fn enforce_receipt_sequence(state: &IndexState, receipt: &ReceiptPayload) -> Result<()> {
    let db = state.db_mut().await;
    let last = db.last_receipt_for_actor(&receipt.actor)?;
    drop(db);
    match last {
        Some((last_seq, last_hash_hex)) => {
            if receipt.seq != last_seq + 1 {
                return Err(anyhow!("receipt seq mismatch"));
            }
            let last_hash = decode_hex("receipt_prev_hash", &last_hash_hex)?;
            if last_hash.len() != 32 {
                return Err(anyhow!("stored receipt hash invalid"));
            }
            if receipt.prev_hash != last_hash {
                return Err(anyhow!("receipt prev_hash mismatch"));
            }
        }
        None => {
            if receipt.seq != 1 {
                return Err(anyhow!("first receipt seq must be 1"));
            }
            if receipt.prev_hash.len() != 32 || receipt.prev_hash.iter().any(|b| *b != 0) {
                return Err(anyhow!("first receipt prev_hash must be zero"));
            }
        }
    }
    Ok(())
}

async fn resolve_pubkey(
    state: &IndexState,
    did: &str,
    provided_hex: Option<&str>,
    allow_unregistered: bool,
) -> Result<Vec<u8>> {
    let provided = match provided_hex {
        Some(hex) => Some(decode_hex("public_key", hex)?),
        None => None,
    };
    if let Some(provided_pk) = provided.as_ref() {
        if provided_pk.len() != 32 {
            return Err(anyhow!("public key must be 32 bytes"));
        }
    }
    let pk = state.resolve_pubkey(did).await;
    let pk = match pk {
        Some(pk) => pk,
        None => {
            if allow_unregistered {
                if let Some(provided_pk) = provided {
                    return Ok(provided_pk);
                }
            }
            return Err(anyhow!("identity not found for {did}"));
        }
    };
    if let Some(provided_pk) = provided {
        if pk != provided_pk {
            return Err(anyhow!("provided public key mismatch for {did}"));
        }
    }
    Ok(pk)
}

fn ensure_not_expired(expires: u64) -> Result<()> {
    let now = now_ts();
    if expires <= now {
        return Err(anyhow!("record expired"));
    }
    Ok(())
}

fn now_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn decode_hex(label: &str, value: &str) -> Result<Vec<u8>> {
    let cleaned = value.trim_start_matches("0x");
    Vec::from_hex(cleaned).with_context(|| format!("decode {label} hex"))
}

fn decode_hex_key(label: &str, value: &str) -> Result<Vec<u8>> {
    decode_hex(label, value)
}
