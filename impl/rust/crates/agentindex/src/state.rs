use crate::db::IndexDb;
use crate::models::{MeshInfoIngest, SearchQuery};
use anyhow::{anyhow, Result};
use serde_json::json;
use std::collections::{HashMap, HashSet};
use tokio::sync::{Mutex, RwLock};

#[derive(Debug, Clone)]
pub struct IdentityEntry {
    pub did: String,
    pub pk_ed25519: Vec<u8>,
    pub pk_x25519: Vec<u8>,
    pub created: u64,
    pub updated: u64,
}

#[derive(Debug, Clone, Default)]
pub struct IdentityState {
    pub records: HashMap<String, IdentityEntry>,
    pub revocations: HashSet<String>,
}

#[derive(Debug, Clone, Default)]
pub struct SkillRegistryState {
    pub records: HashMap<String, SkillRegistryRecord>,
}

#[derive(Debug, Clone)]
pub struct SkillRegistryRecord {
    pub skill_id: String,
    pub author: String,
    pub manifest_hash_hex: String,
    pub manifest_hex: String,
    pub revoked: bool,
    pub revoked_at: Option<u64>,
    pub revocation_reason: Option<String>,
    pub published_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Default)]
pub struct WorkRegistryState {
    pub offers: HashMap<String, WorkOfferRegistryRecord>,
    pub agreements: HashMap<String, WorkAgreementRegistryRecord>,
}

#[derive(Debug, Clone)]
pub struct WorkOfferRegistryRecord {
    pub offer_id: String,
    pub issuer: String,
    pub offer_hash_hex: String,
    pub offer_hex: String,
    pub published_at: u64,
}

#[derive(Debug, Clone)]
pub struct WorkAgreementRegistryRecord {
    pub agreement_id: String,
    pub issuer: String,
    pub agreement_hash_hex: String,
    pub agreement_hex: String,
    pub closed: bool,
    pub closed_at: Option<u64>,
    pub close_reason: Option<String>,
    pub published_at: u64,
    pub updated_at: u64,
}

pub struct IndexState {
    db: Mutex<IndexDb>,
    identity: RwLock<IdentityState>,
    skill_registry: RwLock<SkillRegistryState>,
    work_registry: RwLock<WorkRegistryState>,
    mesh_info: RwLock<Option<MeshInfoIngest>>,
}

impl IndexState {
    pub fn new(db: Mutex<IndexDb>) -> Self {
        Self {
            db,
            identity: RwLock::new(IdentityState::default()),
            skill_registry: RwLock::new(SkillRegistryState::default()),
            work_registry: RwLock::new(WorkRegistryState::default()),
            mesh_info: RwLock::new(None),
        }
    }

    pub async fn stats(&self) -> Result<serde_json::Value> {
        let db = self.db.lock().await;
        db.stats()
    }

    pub async fn search_agent_profiles(
        &self,
        query: SearchQuery,
    ) -> Result<serde_json::Value> {
        let db = self.db.lock().await;
        let results = db.search_agent_profiles(&query)?;
        Ok(json!({ "results": results, "count": results.len() }))
    }

    pub async fn resolve_pubkey(&self, did: &str) -> Option<Vec<u8>> {
        let guard = self.identity.read().await;
        guard.records.get(did).map(|entry| entry.pk_ed25519.clone())
    }

    pub async fn ensure_identity_loaded(&self) -> Result<()> {
        let guard = self.identity.read().await;
        if guard.records.is_empty() {
            return Err(anyhow!("identity state not loaded"));
        }
        Ok(())
    }

    pub async fn set_identity_state(&self, state: IdentityState) -> Result<()> {
        {
            let mut db = self.db.lock().await;
            db.replace_identity_state(&state)?;
        }
        let mut guard = self.identity.write().await;
        *guard = state;
        Ok(())
    }

    pub async fn set_skill_registry_state(&self, state: SkillRegistryState) -> Result<()> {
        let identity = {
            let guard = self.identity.read().await;
            if guard.records.is_empty() {
                return Err(anyhow!("identity state not loaded"));
            }
            guard.clone()
        };
        {
            let mut db = self.db.lock().await;
            db.replace_skill_registry_state(&state, &identity)?;
        }
        let mut guard = self.skill_registry.write().await;
        *guard = state;
        Ok(())
    }

    pub async fn set_work_registry_state(&self, state: WorkRegistryState) -> Result<()> {
        let identity = {
            let guard = self.identity.read().await;
            if guard.records.is_empty() {
                return Err(anyhow!("identity state not loaded"));
            }
            guard.clone()
        };
        {
            let mut db = self.db.lock().await;
            db.replace_work_registry_state(&state, &identity)?;
        }
        let mut guard = self.work_registry.write().await;
        *guard = state;
        Ok(())
    }

    pub async fn skill_registry_record(&self, skill_id: &str) -> Option<SkillRegistryRecord> {
        let guard = self.skill_registry.read().await;
        guard.records.get(skill_id).cloned()
    }

    pub async fn work_offer_registry_record(
        &self,
        offer_id: &str,
    ) -> Option<WorkOfferRegistryRecord> {
        let guard = self.work_registry.read().await;
        guard.offers.get(offer_id).cloned()
    }

    pub async fn work_agreement_registry_record(
        &self,
        agreement_id: &str,
    ) -> Option<WorkAgreementRegistryRecord> {
        let guard = self.work_registry.read().await;
        guard.agreements.get(agreement_id).cloned()
    }

    pub async fn search_agents(&self, query: SearchQuery) -> Result<serde_json::Value> {
        let db = self.db.lock().await;
        let results = db.search_agents(&query)?;
        Ok(json!({ "results": results, "count": results.len() }))
    }

    pub async fn search_skills(&self, query: SearchQuery) -> Result<serde_json::Value> {
        let db = self.db.lock().await;
        let results = db.search_skills(&query)?;
        Ok(json!({ "results": results, "count": results.len() }))
    }

    pub async fn search_work_offers(&self, query: SearchQuery) -> Result<serde_json::Value> {
        let db = self.db.lock().await;
        let results = db.search_work_offers(&query)?;
        Ok(json!({ "results": results, "count": results.len() }))
    }

    pub async fn search_services(&self, query: SearchQuery) -> Result<serde_json::Value> {
        let db = self.db.lock().await;
        let results = db.search_services(&query)?;
        Ok(json!({ "results": results, "count": results.len() }))
    }

    pub async fn search_work_agreements(&self, query: SearchQuery) -> Result<serde_json::Value> {
        let db = self.db.lock().await;
        let results = db.search_work_agreements(&query)?;
        Ok(json!({ "results": results, "count": results.len() }))
    }

    pub async fn set_mesh_info(&self, info: MeshInfoIngest) -> Result<()> {
        let mut guard = self.mesh_info.write().await;
        *guard = Some(info);
        Ok(())
    }

    pub async fn mesh_info(&self) -> Option<MeshInfoIngest> {
        let guard = self.mesh_info.read().await;
        guard.clone()
    }

    pub async fn db_mut(&self) -> tokio::sync::MutexGuard<'_, IndexDb> {
        self.db.lock().await
    }
}
