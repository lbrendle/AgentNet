use crate::models::SearchQuery;
use crate::state::{
    IdentityState, SkillRegistryRecord, SkillRegistryState, WorkAgreementRegistryRecord,
    WorkOfferRegistryRecord, WorkRegistryState,
};
use crate::util::cbor_to_json_value;
use anetsdk::{
    sha256, verify_skill_manifest, verify_work_agreement, verify_work_offer, AgentRecordPayload,
    CommunityRecordPayload, ReceiptPayload, ServiceRecordPayload, SkillManifestPayload,
    WorkAgreementPayload, WorkOfferPayload,
};
use anyhow::{anyhow, Context, Result};
use rusqlite::{params, params_from_iter, Connection, OptionalExtension};
use serde_json::{json, Value};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct IndexDb {
    conn: Connection,
}

impl IndexDb {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path).context("open sqlite db")?;
        conn.pragma_update(None, "journal_mode", "WAL").ok();
        conn.pragma_update(None, "synchronous", "NORMAL").ok();
        conn.pragma_update(None, "foreign_keys", "ON").ok();
        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS agents (
                agent_id TEXT PRIMARY KEY,
                pubkeys_json TEXT NOT NULL,
                node_ids_json TEXT NOT NULL,
                addrs_json TEXT NOT NULL,
                capabilities_json TEXT NOT NULL,
                expires INTEGER NOT NULL,
                record_hex TEXT NOT NULL,
                updated_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS agent_caps (
                agent_id TEXT NOT NULL,
                cap TEXT NOT NULL,
                UNIQUE(agent_id, cap)
            );
            CREATE INDEX IF NOT EXISTS idx_agent_caps_cap ON agent_caps(cap);

            CREATE VIRTUAL TABLE IF NOT EXISTS agents_fts
            USING fts5(agent_id, capabilities);

            CREATE TABLE IF NOT EXISTS services (
                service_key TEXT NOT NULL UNIQUE,
                provider_id TEXT NOT NULL,
                service_type INTEGER NOT NULL,
                addrs_json TEXT NOT NULL,
                required_credentials_json TEXT,
                pricing_json TEXT,
                expires INTEGER NOT NULL,
                record_hex TEXT NOT NULL,
                updated_at INTEGER NOT NULL,
                PRIMARY KEY (provider_id, service_type)
            );
            CREATE INDEX IF NOT EXISTS idx_services_provider ON services(provider_id);
            CREATE INDEX IF NOT EXISTS idx_services_type ON services(service_type);

            CREATE VIRTUAL TABLE IF NOT EXISTS services_fts
            USING fts5(service_key, provider_id, addrs, required_credentials);

            CREATE TABLE IF NOT EXISTS communities (
                community_id TEXT PRIMARY KEY,
                controller TEXT NOT NULL,
                join_policy INTEGER NOT NULL,
                required_credentials_json TEXT,
                economics_json TEXT NOT NULL,
                governance_json TEXT NOT NULL,
                expires INTEGER NOT NULL,
                record_hex TEXT NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_communities_controller ON communities(controller);

            CREATE TABLE IF NOT EXISTS skills (
                skill_id TEXT PRIMARY KEY,
                author TEXT NOT NULL,
                name TEXT NOT NULL,
                version TEXT NOT NULL,
                summary TEXT NOT NULL,
                license TEXT NOT NULL,
                capabilities_json TEXT NOT NULL,
                permissions_json TEXT NOT NULL,
                sandbox_class INTEGER NOT NULL,
                endpoints_json TEXT,
                artifacts_json TEXT,
                requirements_json TEXT,
                pricing_json TEXT,
                attestations_json TEXT,
                metadata_json TEXT,
                ts INTEGER NOT NULL,
                manifest_hash_hex TEXT NOT NULL,
                manifest_hex TEXT NOT NULL,
                revoked INTEGER NOT NULL,
                revoked_at INTEGER,
                revocation_reason TEXT,
                published_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_skills_author ON skills(author);
            CREATE INDEX IF NOT EXISTS idx_skills_sandbox ON skills(sandbox_class);

            CREATE TABLE IF NOT EXISTS skill_caps (
                skill_id TEXT NOT NULL,
                cap TEXT NOT NULL,
                UNIQUE(skill_id, cap)
            );
            CREATE INDEX IF NOT EXISTS idx_skill_caps_cap ON skill_caps(cap);

            CREATE VIRTUAL TABLE IF NOT EXISTS skills_fts
            USING fts5(skill_id, name, summary, capabilities, permissions, requirements);

            CREATE TABLE IF NOT EXISTS work_offers (
                offer_id TEXT PRIMARY KEY,
                issuer TEXT NOT NULL,
                title TEXT NOT NULL,
                summary TEXT NOT NULL,
                scope TEXT NOT NULL,
                budget_amount INTEGER NOT NULL,
                budget_currency TEXT NOT NULL,
                duration_sec INTEGER NOT NULL,
                deliverables_json TEXT NOT NULL,
                requirements_json TEXT,
                ts INTEGER NOT NULL,
                exp INTEGER NOT NULL,
                offer_hash_hex TEXT NOT NULL,
                offer_hex TEXT NOT NULL,
                published_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_work_offers_issuer ON work_offers(issuer);
            CREATE INDEX IF NOT EXISTS idx_work_offers_currency ON work_offers(budget_currency);

            CREATE VIRTUAL TABLE IF NOT EXISTS work_offers_fts
            USING fts5(offer_id, title, summary, scope, deliverables, requirements);

            CREATE TABLE IF NOT EXISTS work_agreements (
                agreement_id TEXT PRIMARY KEY,
                offer_id TEXT NOT NULL,
                issuer TEXT NOT NULL,
                counterparty TEXT NOT NULL,
                budget_amount INTEGER NOT NULL,
                budget_currency TEXT NOT NULL,
                start_ts INTEGER NOT NULL,
                end_ts INTEGER NOT NULL,
                deliverables_json TEXT NOT NULL,
                milestones_count INTEGER NOT NULL,
                escrow_id TEXT,
                dispute_policy_json TEXT,
                ts INTEGER NOT NULL,
                agreement_hash_hex TEXT NOT NULL,
                agreement_hex TEXT NOT NULL,
                closed INTEGER NOT NULL,
                closed_at INTEGER,
                close_reason TEXT,
                published_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_work_agreements_issuer ON work_agreements(issuer);
            CREATE INDEX IF NOT EXISTS idx_work_agreements_currency ON work_agreements(budget_currency);

            CREATE VIRTUAL TABLE IF NOT EXISTS work_agreements_fts
            USING fts5(agreement_id, deliverables);

            CREATE TABLE IF NOT EXISTS receipts (
                receipt_id TEXT PRIMARY KEY,
                ts INTEGER NOT NULL,
                actor TEXT NOT NULL,
                pairing_id TEXT,
                community_id TEXT,
                event_json TEXT NOT NULL,
                auth_json TEXT NOT NULL,
                economics_json TEXT NOT NULL,
                prev_hash_hex TEXT NOT NULL,
                seq INTEGER NOT NULL,
                receipt_hash_hex TEXT NOT NULL,
                signature_hex TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_receipts_actor ON receipts(actor);
            CREATE INDEX IF NOT EXISTS idx_receipts_seq ON receipts(actor, seq);

            CREATE TABLE IF NOT EXISTS identities (
                did TEXT PRIMARY KEY,
                pk_ed25519_hex TEXT NOT NULL,
                pk_x25519_hex TEXT NOT NULL,
                created INTEGER NOT NULL,
                updated INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS identity_revocations (
                credential_hash_hex TEXT PRIMARY KEY
            );
            "#,
        )?;
        Ok(())
    }

    pub fn stats(&self) -> Result<Value> {
        let agents: u64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM agents", [], |row| row.get(0))?;
        let services: u64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM services", [], |row| row.get(0))?;
        let skills: u64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM skills", [], |row| row.get(0))?;
        let work_offers: u64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM work_offers", [], |row| row.get(0))?;
        let work_agreements: u64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM work_agreements", [], |row| row.get(0))?;
        let receipts: u64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM receipts", [], |row| row.get(0))?;
        let identities: u64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM identities", [], |row| row.get(0))?;
        Ok(json!({
            "agents": agents,
            "services": services,
            "skills": skills,
            "work_offers": work_offers,
            "work_agreements": work_agreements,
            "receipts": receipts,
            "identities": identities,
        }))
    }

    pub fn replace_identity_state(&mut self, state: &IdentityState) -> Result<()> {
        let tx = self
            .conn
            .transaction()
            .context("begin identity state transaction")?;
        tx.execute("DELETE FROM identities", [])?;
        tx.execute("DELETE FROM identity_revocations", [])?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO identities (did, pk_ed25519_hex, pk_x25519_hex, created, updated)
                 VALUES (?, ?, ?, ?, ?)",
            )?;
            for record in state.records.values() {
                stmt.execute(params![
                    record.did,
                    hex::encode(&record.pk_ed25519),
                    hex::encode(&record.pk_x25519),
                    record.created,
                    record.updated
                ])?;
            }
        }
        {
            let mut stmt =
                tx.prepare("INSERT INTO identity_revocations (credential_hash_hex) VALUES (?)")?;
            for item in &state.revocations {
                stmt.execute(params![item])?;
            }
        }
        tx.commit().context("commit identity state")?;
        Ok(())
    }

    pub fn replace_skill_registry_state(
        &mut self,
        state: &SkillRegistryState,
        identity: &IdentityState,
    ) -> Result<()> {
        let tx = self
            .conn
            .transaction()
            .context("begin skill registry transaction")?;
        tx.execute("DELETE FROM skills", [])?;
        tx.execute("DELETE FROM skill_caps", [])?;
        tx.execute("DELETE FROM skills_fts", [])?;
        for record in state.records.values() {
            let manifest_bytes = hex::decode(&record.manifest_hex)
                .with_context(|| format!("decode manifest hex {}", record.skill_id))?;
            let manifest_hash = sha256(&manifest_bytes);
            let manifest_hash_hex = hex::encode(manifest_hash);
            if manifest_hash_hex != record.manifest_hash_hex {
                return Err(anyhow!("manifest hash mismatch for {}", record.skill_id));
            }
            let identity_entry = identity
                .records
                .get(&record.author)
                .ok_or_else(|| anyhow!("missing identity for {}", record.author))?;
            let manifest_payload =
                verify_skill_manifest(&manifest_bytes, &identity_entry.pk_ed25519)
                    .with_context(|| format!("verify skill manifest {}", record.skill_id))?;
            if manifest_payload.skill_id != record.skill_id {
                return Err(anyhow!("skill id mismatch for {}", record.skill_id));
            }
            if manifest_payload.author != record.author {
                return Err(anyhow!("manifest author mismatch for {}", record.skill_id));
            }
            Self::insert_skill_with_conn(
                &tx,
                &manifest_payload,
                &record.manifest_hex,
                &manifest_hash_hex,
                Some(record),
            )?;
        }
        tx.commit().context("commit skill registry state")?;
        Ok(())
    }

    pub fn replace_work_registry_state(
        &mut self,
        state: &WorkRegistryState,
        identity: &IdentityState,
    ) -> Result<()> {
        let tx = self
            .conn
            .transaction()
            .context("begin work registry transaction")?;
        tx.execute("DELETE FROM work_offers", [])?;
        tx.execute("DELETE FROM work_offers_fts", [])?;
        tx.execute("DELETE FROM work_agreements", [])?;
        tx.execute("DELETE FROM work_agreements_fts", [])?;

        for record in state.offers.values() {
            let offer_bytes = hex::decode(&record.offer_hex)
                .with_context(|| format!("decode offer hex {}", record.offer_id))?;
            let offer_hash = sha256(&offer_bytes);
            let offer_hash_hex = hex::encode(offer_hash);
            if offer_hash_hex != record.offer_hash_hex {
                return Err(anyhow!("offer hash mismatch for {}", record.offer_id));
            }
            let identity_entry = identity
                .records
                .get(&record.issuer)
                .ok_or_else(|| anyhow!("missing identity for {}", record.issuer))?;
            let offer_payload = verify_work_offer(&offer_bytes, &identity_entry.pk_ed25519)
                .with_context(|| format!("verify work offer {}", record.offer_id))?;
            if offer_payload.offer_id != record.offer_id {
                return Err(anyhow!("offer id mismatch for {}", record.offer_id));
            }
            if offer_payload.issuer != record.issuer {
                return Err(anyhow!("offer issuer mismatch for {}", record.offer_id));
            }
            Self::insert_work_offer_with_conn(
                &tx,
                &offer_payload,
                &record.offer_hex,
                &offer_hash_hex,
                record.published_at,
                record.published_at,
            )?;
        }

        for record in state.agreements.values() {
            let agreement_bytes = hex::decode(&record.agreement_hex)
                .with_context(|| format!("decode agreement hex {}", record.agreement_id))?;
            let agreement_hash = sha256(&agreement_bytes);
            let agreement_hash_hex = hex::encode(agreement_hash);
            if agreement_hash_hex != record.agreement_hash_hex {
                return Err(anyhow!(
                    "agreement hash mismatch for {}",
                    record.agreement_id
                ));
            }
            let identity_entry = identity
                .records
                .get(&record.issuer)
                .ok_or_else(|| anyhow!("missing identity for {}", record.issuer))?;
            let agreement_payload =
                verify_work_agreement(&agreement_bytes, &identity_entry.pk_ed25519)
                    .with_context(|| format!("verify work agreement {}", record.agreement_id))?;
            if agreement_payload.agreement_id != record.agreement_id {
                return Err(anyhow!("agreement id mismatch for {}", record.agreement_id));
            }
            if agreement_payload.issuer != record.issuer {
                return Err(anyhow!(
                    "agreement issuer mismatch for {}",
                    record.agreement_id
                ));
            }
            Self::insert_work_agreement_with_conn(
                &tx,
                &agreement_payload,
                &record.agreement_hex,
                &agreement_hash_hex,
                record.closed,
                record.closed_at,
                record.close_reason.as_deref(),
                record.published_at,
                record.updated_at,
            )?;
        }

        tx.commit().context("commit work registry state")?;
        Ok(())
    }

    pub fn upsert_agent(
        &mut self,
        payload: &AgentRecordPayload,
        record_hex: &str,
        now: u64,
    ) -> Result<()> {
        let pubkeys_json = serde_json::to_string(
            &payload
                .agent_pubkeys
                .iter()
                .map(|k| hex::encode(k))
                .collect::<Vec<_>>(),
        )?;
        let node_ids_json = serde_json::to_string(&payload.contact.node_ids)?;
        let addrs_json = serde_json::to_string(&payload.contact.addrs)?;
        let caps_json = serde_json::to_string(&payload.capabilities)?;
        self.conn.execute(
            "INSERT OR REPLACE INTO agents
             (agent_id, pubkeys_json, node_ids_json, addrs_json, capabilities_json, expires, record_hex, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                payload.agent_id,
                pubkeys_json,
                node_ids_json,
                addrs_json,
                caps_json,
                payload.expires,
                record_hex,
                now
            ],
        )?;
        self.conn.execute(
            "DELETE FROM agent_caps WHERE agent_id = ?",
            params![payload.agent_id],
        )?;
        {
            let mut stmt = self
                .conn
                .prepare("INSERT OR IGNORE INTO agent_caps (agent_id, cap) VALUES (?, ?)")?;
            for cap in &payload.capabilities {
                stmt.execute(params![payload.agent_id, cap])?;
            }
        }
        self.conn.execute(
            "DELETE FROM agents_fts WHERE agent_id = ?",
            params![payload.agent_id],
        )?;
        let caps_text = payload.capabilities.join(" ");
        self.conn.execute(
            "INSERT INTO agents_fts (agent_id, capabilities) VALUES (?, ?)",
            params![payload.agent_id, caps_text],
        )?;
        Ok(())
    }

    pub fn upsert_service(
        &mut self,
        payload: &ServiceRecordPayload,
        record_hex: &str,
        now: u64,
    ) -> Result<()> {
        let addrs_json = serde_json::to_string(&payload.addrs)?;
        let required_json = match &payload.required_credentials {
            Some(list) => Some(serde_json::to_string(list)?),
            None => None,
        };
        let pricing_json = match &payload.pricing {
            Some(value) => Some(serde_json::to_string(&cbor_to_json_value(value))?),
            None => None,
        };
        let service_key = format!("{}:{}", payload.provider_id, payload.service_type);
        self.conn.execute(
            "INSERT OR REPLACE INTO services
             (service_key, provider_id, service_type, addrs_json, required_credentials_json, pricing_json, expires, record_hex, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                service_key,
                payload.provider_id,
                payload.service_type as u64,
                addrs_json,
                required_json,
                pricing_json,
                payload.expires,
                record_hex,
                now
            ],
        )?;
        self.conn.execute(
            "DELETE FROM services_fts WHERE service_key = ?",
            params![service_key],
        )?;
        let required_text = payload
            .required_credentials
            .clone()
            .unwrap_or_default()
            .join(" ");
        let addrs_text = payload.addrs.join(" ");
        self.conn.execute(
            "INSERT INTO services_fts (service_key, provider_id, addrs, required_credentials)
             VALUES (?, ?, ?, ?)",
            params![service_key, payload.provider_id, addrs_text, required_text],
        )?;
        Ok(())
    }

    pub fn upsert_community(
        &mut self,
        payload: &CommunityRecordPayload,
        record_hex: &str,
        now: u64,
    ) -> Result<()> {
        let required_json = match &payload.required_credentials {
            Some(list) => Some(serde_json::to_string(list)?),
            None => None,
        };
        let economics_json = serde_json::to_string(&cbor_to_json_value(&payload.economics))?;
        let governance_json = serde_json::to_string(&cbor_to_json_value(&payload.governance))?;
        self.conn.execute(
            "INSERT OR REPLACE INTO communities
             (community_id, controller, join_policy, required_credentials_json, economics_json, governance_json, expires, record_hex, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                payload.community_id,
                payload.controller,
                payload.join_policy as u64,
                required_json,
                economics_json,
                governance_json,
                payload.expires,
                record_hex,
                now
            ],
        )?;
        Ok(())
    }

    pub fn upsert_skill_manifest(
        &mut self,
        payload: &SkillManifestPayload,
        manifest_hex: &str,
        manifest_hash_hex: &str,
        registry: Option<&SkillRegistryRecord>,
    ) -> Result<()> {
        Self::insert_skill_with_conn(
            &self.conn,
            payload,
            manifest_hex,
            manifest_hash_hex,
            registry,
        )
    }

    pub fn upsert_work_offer(
        &mut self,
        payload: &WorkOfferPayload,
        offer_hex: &str,
        offer_hash_hex: &str,
        registry: Option<&WorkOfferRegistryRecord>,
    ) -> Result<()> {
        let published_at = registry.map(|r| r.published_at).unwrap_or(payload.ts);
        Self::insert_work_offer_with_conn(
            &self.conn,
            payload,
            offer_hex,
            offer_hash_hex,
            published_at,
            published_at,
        )
    }

    pub fn upsert_work_agreement(
        &mut self,
        payload: &WorkAgreementPayload,
        agreement_hex: &str,
        agreement_hash_hex: &str,
        registry: Option<&WorkAgreementRegistryRecord>,
    ) -> Result<()> {
        let published_at = registry.map(|r| r.published_at).unwrap_or(payload.ts);
        let updated_at = registry.map(|r| r.updated_at).unwrap_or(payload.ts);
        let closed = registry.map(|r| r.closed).unwrap_or(false);
        let closed_at = registry.and_then(|r| r.closed_at);
        let close_reason = registry.and_then(|r| r.close_reason.as_deref());
        Self::insert_work_agreement_with_conn(
            &self.conn,
            payload,
            agreement_hex,
            agreement_hash_hex,
            closed,
            closed_at,
            close_reason,
            published_at,
            updated_at,
        )
    }

    pub fn insert_receipt(
        &mut self,
        payload: &ReceiptPayload,
        receipt_hash_hex: &str,
        signature_hex: &str,
        event_json: &str,
        auth_json: &str,
        economics_json: &str,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO receipts
             (receipt_id, ts, actor, pairing_id, community_id, event_json, auth_json, economics_json,
              prev_hash_hex, seq, receipt_hash_hex, signature_hex)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                payload.receipt_id,
                payload.ts,
                payload.actor,
                payload.pairing_id,
                payload.community_id,
                event_json,
                auth_json,
                economics_json,
                hex::encode(&payload.prev_hash),
                payload.seq,
                receipt_hash_hex,
                signature_hex
            ],
        )?;
        Ok(())
    }

    pub fn last_receipt_for_actor(&self, actor: &str) -> Result<Option<(u64, String)>> {
        let row: Option<(u64, String)> = self
            .conn
            .query_row(
                "SELECT seq, receipt_hash_hex FROM receipts WHERE actor = ? ORDER BY seq DESC LIMIT 1",
                params![actor],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        Ok(row)
    }

    pub fn search_agents(&self, query: &SearchQuery) -> Result<Vec<Value>> {
        let (limit, offset) = limit_offset(query);
        let mut sql = String::from(
            "SELECT a.agent_id, a.pubkeys_json, a.node_ids_json, a.addrs_json, a.capabilities_json, a.expires
             FROM agents a",
        );
        let mut conditions = Vec::new();
        let mut params_vec: Vec<rusqlite::types::Value> = Vec::new();
        if let Some(q) = query.q.as_ref().filter(|q| !q.trim().is_empty()) {
            sql.push_str(" JOIN agents_fts f ON a.agent_id = f.agent_id");
            conditions.push("f MATCH ?");
            params_vec.push(q.to_string().into());
        }
        if let Some(cap) = query.capability.as_ref().filter(|c| !c.trim().is_empty()) {
            conditions.push(
                "EXISTS (SELECT 1 FROM agent_caps c WHERE c.agent_id = a.agent_id AND c.cap = ?)",
            );
            params_vec.push(cap.to_string().into());
        }
        apply_conditions(&mut sql, &conditions);
        sql.push_str(" ORDER BY a.expires DESC LIMIT ? OFFSET ?");
        params_vec.push((limit as i64).into());
        params_vec.push((offset as i64).into());
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(params_vec.iter()), |row| {
            let agent_id: String = row.get(0)?;
            let pubkeys_json: String = row.get(1)?;
            let node_ids_json: String = row.get(2)?;
            let addrs_json: String = row.get(3)?;
            let caps_json: String = row.get(4)?;
            let expires: u64 = row.get(5)?;
            Ok(json!({
                "agent_id": agent_id,
                "pubkeys": parse_json_value(pubkeys_json),
                "node_ids": parse_json_value(node_ids_json),
                "addrs": parse_json_value(addrs_json),
                "capabilities": parse_json_value(caps_json),
                "expires": expires,
            }))
        })?;
        collect_rows(rows)
    }

    pub fn search_skills(&self, query: &SearchQuery) -> Result<Vec<Value>> {
        let (limit, offset) = limit_offset(query);
        let mut sql = String::from(
            "SELECT s.skill_id, s.author, s.name, s.version, s.summary, s.license,
                    s.capabilities_json, s.permissions_json, s.sandbox_class, s.endpoints_json,
                    s.requirements_json, s.pricing_json, s.attestations_json, s.metadata_json,
                    s.ts, s.manifest_hash_hex, s.revoked, s.revoked_at, s.revocation_reason,
                    s.published_at, s.updated_at
             FROM skills s",
        );
        let mut conditions = Vec::new();
        let mut params_vec: Vec<rusqlite::types::Value> = Vec::new();
        if let Some(q) = query.q.as_ref().filter(|q| !q.trim().is_empty()) {
            sql.push_str(" JOIN skills_fts f ON s.skill_id = f.skill_id");
            conditions.push("f MATCH ?");
            params_vec.push(q.to_string().into());
        }
        if let Some(cap) = query.capability.as_ref().filter(|c| !c.trim().is_empty()) {
            conditions.push(
                "EXISTS (SELECT 1 FROM skill_caps c WHERE c.skill_id = s.skill_id AND c.cap = ?)",
            );
            params_vec.push(cap.to_string().into());
        }
        if let Some(sandbox) = query.sandbox_class {
            conditions.push("s.sandbox_class = ?");
            params_vec.push((sandbox as i64).into());
        }
        if let Some(status) = query.status.as_ref().filter(|s| !s.trim().is_empty()) {
            match status.as_str() {
                "active" => conditions.push("s.revoked = 0"),
                "revoked" => conditions.push("s.revoked = 1"),
                _ => return Err(anyhow!("invalid status filter")),
            }
        }
        apply_conditions(&mut sql, &conditions);
        sql.push_str(" ORDER BY s.updated_at DESC LIMIT ? OFFSET ?");
        params_vec.push((limit as i64).into());
        params_vec.push((offset as i64).into());
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(params_vec.iter()), |row| {
            let skill_id: String = row.get(0)?;
            let author: String = row.get(1)?;
            let name: String = row.get(2)?;
            let version: String = row.get(3)?;
            let summary: String = row.get(4)?;
            let license: String = row.get(5)?;
            let capabilities_json: String = row.get(6)?;
            let permissions_json: String = row.get(7)?;
            let sandbox_class: u64 = row.get(8)?;
            let endpoints_json: Option<String> = row.get(9)?;
            let requirements_json: Option<String> = row.get(10)?;
            let pricing_json: Option<String> = row.get(11)?;
            let attestations_json: Option<String> = row.get(12)?;
            let metadata_json: Option<String> = row.get(13)?;
            let ts: u64 = row.get(14)?;
            let manifest_hash_hex: String = row.get(15)?;
            let revoked: u64 = row.get(16)?;
            let revoked_at: Option<u64> = row.get(17)?;
            let revocation_reason: Option<String> = row.get(18)?;
            let published_at: u64 = row.get(19)?;
            let updated_at: u64 = row.get(20)?;
            Ok(json!({
                "skill_id": skill_id,
                "author": author,
                "name": name,
                "version": version,
                "summary": summary,
                "license": license,
                "capabilities": parse_json_value(capabilities_json),
                "permissions": parse_json_value(permissions_json),
                "sandbox_class": sandbox_class,
                "endpoints": parse_optional_json(endpoints_json),
                "requirements": parse_optional_json(requirements_json),
                "pricing": parse_optional_json(pricing_json),
                "attestations": parse_optional_json(attestations_json),
                "metadata": parse_optional_json(metadata_json),
                "ts": ts,
                "manifest_hash_hex": manifest_hash_hex,
                "revoked": revoked == 1,
                "revoked_at": revoked_at,
                "revocation_reason": revocation_reason,
                "published_at": published_at,
                "updated_at": updated_at,
            }))
        })?;
        collect_rows(rows)
    }

    pub fn search_services(&self, query: &SearchQuery) -> Result<Vec<Value>> {
        let (limit, offset) = limit_offset(query);
        let now = now_ts();
        let mut sql = String::from(
            "SELECT s.provider_id, s.service_type, s.addrs_json, s.required_credentials_json,
                    s.pricing_json, s.expires
             FROM services s",
        );
        let mut conditions = Vec::new();
        let mut params_vec: Vec<rusqlite::types::Value> = Vec::new();
        if let Some(q) = query.q.as_ref().filter(|q| !q.trim().is_empty()) {
            sql.push_str(" JOIN services_fts f ON s.service_key = f.service_key");
            conditions.push("f MATCH ?");
            params_vec.push(q.to_string().into());
        }
        if let Some(service_type) = query.service_type {
            conditions.push("s.service_type = ?");
            params_vec.push((service_type as i64).into());
        }
        if let Some(provider) = query.provider_id.as_ref().filter(|p| !p.trim().is_empty()) {
            conditions.push("s.provider_id = ?");
            params_vec.push(provider.to_string().into());
        }
        if let Some(status) = query.status.as_ref().filter(|s| !s.trim().is_empty()) {
            match status.as_str() {
                "active" => {
                    conditions.push("s.expires > ?");
                    params_vec.push((now as i64).into());
                }
                "expired" => {
                    conditions.push("s.expires <= ?");
                    params_vec.push((now as i64).into());
                }
                _ => return Err(anyhow!("invalid status filter")),
            }
        }
        apply_conditions(&mut sql, &conditions);
        sql.push_str(" ORDER BY s.expires DESC LIMIT ? OFFSET ?");
        params_vec.push((limit as i64).into());
        params_vec.push((offset as i64).into());
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(params_vec.iter()), |row| {
            let provider_id: String = row.get(0)?;
            let service_type: u64 = row.get(1)?;
            let addrs_json: String = row.get(2)?;
            let required_json: Option<String> = row.get(3)?;
            let pricing_json: Option<String> = row.get(4)?;
            let expires: u64 = row.get(5)?;
            Ok(json!({
                "provider_id": provider_id,
                "service_type": service_type,
                "addrs": parse_json_value(addrs_json),
                "required_credentials": parse_optional_json(required_json),
                "pricing": parse_optional_json(pricing_json),
                "expires": expires,
                "status": if expires > now { "active" } else { "expired" }
            }))
        })?;
        collect_rows(rows)
    }

    pub fn search_work_offers(&self, query: &SearchQuery) -> Result<Vec<Value>> {
        let (limit, offset) = limit_offset(query);
        let now = now_ts();
        let mut sql = String::from(
            "SELECT w.offer_id, w.issuer, w.title, w.summary, w.scope, w.budget_amount,
                    w.budget_currency, w.duration_sec, w.deliverables_json, w.requirements_json,
                    w.ts, w.exp, w.offer_hash_hex, w.published_at, w.updated_at
             FROM work_offers w",
        );
        let mut conditions = Vec::new();
        let mut params_vec: Vec<rusqlite::types::Value> = Vec::new();
        if let Some(q) = query.q.as_ref().filter(|q| !q.trim().is_empty()) {
            sql.push_str(" JOIN work_offers_fts f ON w.offer_id = f.offer_id");
            conditions.push("f MATCH ?");
            params_vec.push(q.to_string().into());
        }
        if let Some(currency) = query.currency.as_ref().filter(|c| !c.trim().is_empty()) {
            conditions.push("w.budget_currency = ?");
            params_vec.push(currency.to_string().into());
        }
        if let Some(provider) = query.provider_id.as_ref().filter(|p| !p.trim().is_empty()) {
            conditions.push("w.issuer = ?");
            params_vec.push(provider.to_string().into());
        }
        if let Some(status) = query.status.as_ref().filter(|s| !s.trim().is_empty()) {
            match status.as_str() {
                "open" => {
                    conditions.push("w.exp > ?");
                    params_vec.push((now as i64).into());
                }
                "expired" => {
                    conditions.push("w.exp <= ?");
                    params_vec.push((now as i64).into());
                }
                _ => return Err(anyhow!("invalid status filter")),
            }
        }
        apply_conditions(&mut sql, &conditions);
        sql.push_str(" ORDER BY w.updated_at DESC LIMIT ? OFFSET ?");
        params_vec.push((limit as i64).into());
        params_vec.push((offset as i64).into());
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(params_vec.iter()), |row| {
            let offer_id: String = row.get(0)?;
            let issuer: String = row.get(1)?;
            let title: String = row.get(2)?;
            let summary: String = row.get(3)?;
            let scope: String = row.get(4)?;
            let budget_amount: u64 = row.get(5)?;
            let budget_currency: String = row.get(6)?;
            let duration_sec: u64 = row.get(7)?;
            let deliverables_json: String = row.get(8)?;
            let requirements_json: Option<String> = row.get(9)?;
            let ts: u64 = row.get(10)?;
            let exp: u64 = row.get(11)?;
            let offer_hash_hex: String = row.get(12)?;
            let published_at: u64 = row.get(13)?;
            let updated_at: u64 = row.get(14)?;
            Ok(json!({
                "offer_id": offer_id,
                "issuer": issuer,
                "title": title,
                "summary": summary,
                "scope": scope,
                "budget_amount": budget_amount,
                "budget_currency": budget_currency,
                "duration_sec": duration_sec,
                "deliverables": parse_json_value(deliverables_json),
                "requirements": parse_optional_json(requirements_json),
                "ts": ts,
                "exp": exp,
                "offer_hash_hex": offer_hash_hex,
                "status": if exp > now { "open" } else { "expired" },
                "published_at": published_at,
                "updated_at": updated_at,
            }))
        })?;
        collect_rows(rows)
    }

    pub fn search_work_agreements(&self, query: &SearchQuery) -> Result<Vec<Value>> {
        let (limit, offset) = limit_offset(query);
        let now = now_ts();
        let mut sql = String::from(
            "SELECT w.agreement_id, w.offer_id, w.issuer, w.counterparty, w.budget_amount,
                    w.budget_currency, w.start_ts, w.end_ts, w.deliverables_json,
                    w.milestones_count, w.escrow_id, w.dispute_policy_json, w.ts,
                    w.agreement_hash_hex, w.closed, w.closed_at, w.close_reason,
                    w.published_at, w.updated_at
             FROM work_agreements w",
        );
        let mut conditions = Vec::new();
        let mut params_vec: Vec<rusqlite::types::Value> = Vec::new();
        if let Some(q) = query.q.as_ref().filter(|q| !q.trim().is_empty()) {
            sql.push_str(" JOIN work_agreements_fts f ON w.agreement_id = f.agreement_id");
            conditions.push("f MATCH ?");
            params_vec.push(q.to_string().into());
        }
        if let Some(currency) = query.currency.as_ref().filter(|c| !c.trim().is_empty()) {
            conditions.push("w.budget_currency = ?");
            params_vec.push(currency.to_string().into());
        }
        if let Some(provider) = query.provider_id.as_ref().filter(|p| !p.trim().is_empty()) {
            conditions.push("(w.issuer = ? OR w.counterparty = ?)");
            params_vec.push(provider.to_string().into());
            params_vec.push(provider.to_string().into());
        }
        if let Some(status) = query.status.as_ref().filter(|s| !s.trim().is_empty()) {
            match status.as_str() {
                "open" => conditions.push("w.closed = 0"),
                "closed" => conditions.push("w.closed = 1"),
                "active" => {
                    conditions.push("w.closed = 0 AND w.start_ts <= ? AND w.end_ts >= ?");
                    params_vec.push((now as i64).into());
                    params_vec.push((now as i64).into());
                }
                _ => return Err(anyhow!("invalid status filter")),
            }
        }
        apply_conditions(&mut sql, &conditions);
        sql.push_str(" ORDER BY w.updated_at DESC LIMIT ? OFFSET ?");
        params_vec.push((limit as i64).into());
        params_vec.push((offset as i64).into());
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(params_vec.iter()), |row| {
            let agreement_id: String = row.get(0)?;
            let offer_id: String = row.get(1)?;
            let issuer: String = row.get(2)?;
            let counterparty: String = row.get(3)?;
            let budget_amount: u64 = row.get(4)?;
            let budget_currency: String = row.get(5)?;
            let start_ts: u64 = row.get(6)?;
            let end_ts: u64 = row.get(7)?;
            let deliverables_json: String = row.get(8)?;
            let milestones_count: u64 = row.get(9)?;
            let escrow_id: Option<String> = row.get(10)?;
            let dispute_policy_json: Option<String> = row.get(11)?;
            let ts: u64 = row.get(12)?;
            let agreement_hash_hex: String = row.get(13)?;
            let closed: u64 = row.get(14)?;
            let closed_at: Option<u64> = row.get(15)?;
            let close_reason: Option<String> = row.get(16)?;
            let published_at: u64 = row.get(17)?;
            let updated_at: u64 = row.get(18)?;
            let status = if closed == 1 {
                "closed"
            } else if start_ts <= now && end_ts >= now {
                "active"
            } else {
                "open"
            };
            Ok(json!({
                "agreement_id": agreement_id,
                "offer_id": offer_id,
                "issuer": issuer,
                "counterparty": counterparty,
                "budget_amount": budget_amount,
                "budget_currency": budget_currency,
                "start_ts": start_ts,
                "end_ts": end_ts,
                "deliverables": parse_json_value(deliverables_json),
                "milestones_count": milestones_count,
                "escrow_id": escrow_id,
                "dispute_policy": parse_optional_json(dispute_policy_json),
                "ts": ts,
                "agreement_hash_hex": agreement_hash_hex,
                "closed": closed == 1,
                "closed_at": closed_at,
                "close_reason": close_reason,
                "status": status,
                "published_at": published_at,
                "updated_at": updated_at,
            }))
        })?;
        collect_rows(rows)
    }

    fn insert_skill_with_conn(
        conn: &Connection,
        payload: &SkillManifestPayload,
        manifest_hex: &str,
        manifest_hash_hex: &str,
        registry: Option<&SkillRegistryRecord>,
    ) -> Result<()> {
        let capabilities_json = serde_json::to_string(&payload.capabilities)?;
        let permissions_json = serde_json::to_string(&payload.permissions)?;
        let endpoints_json = match &payload.endpoints {
            Some(list) => Some(serde_json::to_string(list)?),
            None => None,
        };
        let artifacts_json = match &payload.artifacts {
            Some(list) => {
                let values = list
                    .iter()
                    .map(|artifact| {
                        json!({
                            "kind": artifact.kind,
                            "digest_hex": hex::encode(&artifact.digest),
                            "size": artifact.size,
                            "uris": artifact.uris,
                        })
                    })
                    .collect::<Vec<_>>();
                Some(serde_json::to_string(&values)?)
            }
            None => None,
        };
        let requirements_json = match &payload.requirements {
            Some(list) => Some(serde_json::to_string(list)?),
            None => None,
        };
        let pricing_json = match &payload.pricing {
            Some(value) => Some(serde_json::to_string(&cbor_to_json_value(value))?),
            None => None,
        };
        let attestations_json = match &payload.attestations {
            Some(value) => Some(serde_json::to_string(&cbor_to_json_value(value))?),
            None => None,
        };
        let metadata_json = match &payload.metadata {
            Some(value) => Some(serde_json::to_string(&cbor_to_json_value(value))?),
            None => None,
        };
        let revoked = registry.map(|r| r.revoked).unwrap_or(false);
        let revoked_at = registry.and_then(|r| r.revoked_at);
        let revocation_reason = registry.and_then(|r| r.revocation_reason.as_deref());
        let published_at = registry.map(|r| r.published_at).unwrap_or(payload.ts);
        let updated_at = registry.map(|r| r.updated_at).unwrap_or(payload.ts);
        conn.execute(
            "INSERT OR REPLACE INTO skills
             (skill_id, author, name, version, summary, license, capabilities_json, permissions_json,
              sandbox_class, endpoints_json, artifacts_json, requirements_json, pricing_json,
              attestations_json, metadata_json, ts, manifest_hash_hex, manifest_hex, revoked, revoked_at,
              revocation_reason, published_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                payload.skill_id,
                payload.author,
                payload.name,
                payload.version,
                payload.summary,
                payload.license,
                capabilities_json,
                permissions_json,
                payload.sandbox_class as u64,
                endpoints_json,
                artifacts_json,
                requirements_json,
                pricing_json,
                attestations_json,
                metadata_json,
                payload.ts,
                manifest_hash_hex,
                manifest_hex,
                if revoked { 1 } else { 0 },
                revoked_at,
                revocation_reason,
                published_at,
                updated_at
            ],
        )?;
        conn.execute(
            "DELETE FROM skill_caps WHERE skill_id = ?",
            params![payload.skill_id],
        )?;
        {
            let mut stmt =
                conn.prepare("INSERT OR IGNORE INTO skill_caps (skill_id, cap) VALUES (?, ?)")?;
            for cap in &payload.capabilities {
                stmt.execute(params![payload.skill_id, cap])?;
            }
        }
        conn.execute(
            "DELETE FROM skills_fts WHERE skill_id = ?",
            params![payload.skill_id],
        )?;
        let caps_text = payload.capabilities.join(" ");
        let perms_text = payload.permissions.join(" ");
        let req_text = payload.requirements.clone().unwrap_or_default().join(" ");
        conn.execute(
            "INSERT INTO skills_fts (skill_id, name, summary, capabilities, permissions, requirements)
             VALUES (?, ?, ?, ?, ?, ?)",
            params![
                payload.skill_id,
                payload.name,
                payload.summary,
                caps_text,
                perms_text,
                req_text
            ],
        )?;
        Ok(())
    }

    fn insert_work_offer_with_conn(
        conn: &Connection,
        payload: &WorkOfferPayload,
        offer_hex: &str,
        offer_hash_hex: &str,
        published_at: u64,
        updated_at: u64,
    ) -> Result<()> {
        let deliverables_json = serde_json::to_string(&payload.deliverables)?;
        let requirements_json = match &payload.requirements {
            Some(list) => Some(serde_json::to_string(list)?),
            None => None,
        };
        conn.execute(
            "INSERT OR REPLACE INTO work_offers
             (offer_id, issuer, title, summary, scope, budget_amount, budget_currency, duration_sec,
              deliverables_json, requirements_json, ts, exp, offer_hash_hex, offer_hex, published_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                payload.offer_id,
                payload.issuer,
                payload.title,
                payload.summary,
                payload.scope,
                payload.budget_amount,
                payload.budget_currency,
                payload.duration_sec,
                deliverables_json,
                requirements_json,
                payload.ts,
                payload.exp,
                offer_hash_hex,
                offer_hex,
                published_at,
                updated_at
            ],
        )?;
        conn.execute(
            "DELETE FROM work_offers_fts WHERE offer_id = ?",
            params![payload.offer_id],
        )?;
        let deliverables_text = payload.deliverables.join(" ");
        let requirements_text = payload.requirements.clone().unwrap_or_default().join(" ");
        conn.execute(
            "INSERT INTO work_offers_fts (offer_id, title, summary, scope, deliverables, requirements)
             VALUES (?, ?, ?, ?, ?, ?)",
            params![
                payload.offer_id,
                payload.title,
                payload.summary,
                payload.scope,
                deliverables_text,
                requirements_text
            ],
        )?;
        Ok(())
    }

    fn insert_work_agreement_with_conn(
        conn: &Connection,
        payload: &WorkAgreementPayload,
        agreement_hex: &str,
        agreement_hash_hex: &str,
        closed: bool,
        closed_at: Option<u64>,
        close_reason: Option<&str>,
        published_at: u64,
        updated_at: u64,
    ) -> Result<()> {
        let deliverables_json = serde_json::to_string(&payload.deliverables)?;
        let milestones_count = payload
            .milestones
            .as_ref()
            .map(|m| m.len() as u64)
            .unwrap_or(0);
        let dispute_policy_json = match &payload.dispute_policy {
            Some(value) => Some(serde_json::to_string(&cbor_to_json_value(value))?),
            None => None,
        };
        conn.execute(
            "INSERT OR REPLACE INTO work_agreements
             (agreement_id, offer_id, issuer, counterparty, budget_amount, budget_currency, start_ts, end_ts,
              deliverables_json, milestones_count, escrow_id, dispute_policy_json, ts, agreement_hash_hex,
              agreement_hex, closed, closed_at, close_reason, published_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                payload.agreement_id,
                payload.offer_id,
                payload.issuer,
                payload.counterparty,
                payload.budget_amount,
                payload.budget_currency,
                payload.start_ts,
                payload.end_ts,
                deliverables_json,
                milestones_count,
                payload.escrow_id,
                dispute_policy_json,
                payload.ts,
                agreement_hash_hex,
                agreement_hex,
                if closed { 1 } else { 0 },
                closed_at,
                close_reason,
                published_at,
                updated_at
            ],
        )?;
        conn.execute(
            "DELETE FROM work_agreements_fts WHERE agreement_id = ?",
            params![payload.agreement_id],
        )?;
        let deliverables_text = payload.deliverables.join(" ");
        conn.execute(
            "INSERT INTO work_agreements_fts (agreement_id, deliverables) VALUES (?, ?)",
            params![payload.agreement_id, deliverables_text],
        )?;
        Ok(())
    }
}

fn now_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn limit_offset(query: &SearchQuery) -> (u64, u64) {
    let limit = query.limit.unwrap_or(25).min(100);
    let offset = query.offset.unwrap_or(0);
    (limit, offset)
}

fn apply_conditions(sql: &mut String, conditions: &[&str]) {
    if conditions.is_empty() {
        return;
    }
    sql.push_str(" WHERE ");
    sql.push_str(&conditions.join(" AND "));
}

fn collect_rows(
    rows: rusqlite::MappedRows<'_, impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<Value>>,
) -> Result<Vec<Value>> {
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

fn parse_json_value(raw: String) -> Value {
    serde_json::from_str(&raw).unwrap_or(Value::String(raw))
}

fn parse_optional_json(raw: Option<String>) -> Value {
    match raw {
        Some(value) => parse_json_value(value),
        None => Value::Null,
    }
}
