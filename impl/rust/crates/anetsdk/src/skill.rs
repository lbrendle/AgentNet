use crate::signed::{split_signed_map, with_signature};
use crate::{
    decode_canonical, encode_canonical, sha256, sign_ed25519_hash, verify_ed25519_hash, CborValue,
    Error,
};
use crate::schema::{
    expect_bytes, expect_bytes_len, expect_map, expect_text, expect_text_array, expect_u16,
    expect_u64, expect_u8, get_optional, get_required,
};

const SKILL_SIG_KEY: u64 = 16;
const SANDBOX_MIN: u16 = 1;
const SANDBOX_MAX: u16 = 5;

#[derive(Debug, Clone)]
pub struct SkillArtifact {
    pub kind: u8,
    pub digest: Vec<u8>,
    pub size: u64,
    pub uris: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SkillManifestPayload {
    pub skill_id: String,
    pub author: String,
    pub name: String,
    pub version: String,
    pub summary: String,
    pub license: String,
    pub capabilities: Vec<String>,
    pub permissions: Vec<String>,
    pub sandbox_class: u16,
    pub endpoints: Option<Vec<String>>,
    pub artifacts: Option<Vec<SkillArtifact>>,
    pub requirements: Option<Vec<String>>,
    pub pricing: Option<CborValue>,
    pub attestations: Option<CborValue>,
    pub metadata: Option<CborValue>,
    pub ts: u64,
}

#[derive(Debug, Clone)]
pub struct SkillManifest {
    pub payload: SkillManifestPayload,
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct SkillPublishPayload {
    pub manifest: Vec<u8>,
    pub ts: u64,
}

#[derive(Debug, Clone)]
pub struct SkillUpdatePayload {
    pub skill_id: String,
    pub prev_manifest_hash: Vec<u8>,
    pub manifest: Vec<u8>,
    pub ts: u64,
}

#[derive(Debug, Clone)]
pub struct SkillRevokePayload {
    pub skill_id: String,
    pub manifest_hash: Vec<u8>,
    pub reason: String,
    pub ts: u64,
}

pub fn parse_skill_manifest_payload(value: &CborValue) -> Result<SkillManifestPayload, Error> {
    let map = expect_map(value)?;
    let skill_id = expect_text(get_required(&map, 0)?)?;
    let author = expect_text(get_required(&map, 1)?)?;
    let name = expect_text(get_required(&map, 2)?)?;
    let version = expect_text(get_required(&map, 3)?)?;
    let summary = expect_text(get_required(&map, 4)?)?;
    let license = expect_text(get_required(&map, 5)?)?;
    let capabilities = expect_text_array(get_required(&map, 6)?)?;
    let permissions = expect_text_array(get_required(&map, 7)?)?;
    let sandbox_class = expect_u16(get_required(&map, 8)?)?;
    let endpoints = match get_optional(&map, 9) {
        Some(value) => Some(expect_text_array(value)?),
        None => None,
    };
    let artifacts = match get_optional(&map, 10) {
        Some(value) => Some(parse_artifacts(value)?),
        None => None,
    };
    let requirements = match get_optional(&map, 11) {
        Some(value) => Some(expect_text_array(value)?),
        None => None,
    };
    let pricing = get_optional(&map, 12).cloned();
    let attestations = get_optional(&map, 13).cloned();
    let metadata = get_optional(&map, 14).cloned();
    let ts = expect_u64(get_required(&map, 15)?)?;

    let payload = SkillManifestPayload {
        skill_id,
        author,
        name,
        version,
        summary,
        license,
        capabilities,
        permissions,
        sandbox_class,
        endpoints,
        artifacts,
        requirements,
        pricing,
        attestations,
        metadata,
        ts,
    };
    payload.validate()?;
    Ok(payload)
}

pub fn parse_skill_manifest(value: &CborValue) -> Result<SkillManifest, Error> {
    let (payload_entries, signature) = split_signed_map(value, SKILL_SIG_KEY)?;
    let payload_value = CborValue::Map(payload_entries);
    let payload = parse_skill_manifest_payload(&payload_value)?;
    Ok(SkillManifest { payload, signature })
}

pub fn decode_skill_manifest(data: &[u8]) -> Result<SkillManifest, Error> {
    let value = decode_canonical(data)?;
    parse_skill_manifest(&value)
}

pub fn build_skill_manifest(payload: &SkillManifestPayload, secret_key: &[u8]) -> Result<Vec<u8>, Error> {
    payload.validate()?;
    let payload_value = payload.to_cbor()?;
    let payload_cbor = encode_canonical(&payload_value)?;
    let hash = sha256(&payload_cbor);
    let sig = sign_ed25519_hash(secret_key, &hash)?;
    let full = with_signature(&payload_value, SKILL_SIG_KEY, sig)?;
    encode_canonical(&full)
}

pub fn verify_skill_manifest(data: &[u8], public_key: &[u8]) -> Result<SkillManifestPayload, Error> {
    let value = decode_canonical(data)?;
    let (payload_entries, signature) = split_signed_map(&value, SKILL_SIG_KEY)?;
    let payload_value = CborValue::Map(payload_entries);
    let payload_cbor = encode_canonical(&payload_value)?;
    let hash = sha256(&payload_cbor);
    verify_ed25519_hash(public_key, &hash, &signature)?;
    parse_skill_manifest_payload(&payload_value)
}

pub fn parse_skill_publish_payload(value: &CborValue) -> Result<SkillPublishPayload, Error> {
    let map = expect_map(value)?;
    let manifest = expect_bytes(get_required(&map, 0)?)?;
    let ts = expect_u64(get_required(&map, 1)?)?;
    if ts == 0 {
        return Err(Error::Cbor("timestamp required"));
    }
    decode_skill_manifest(&manifest)?;
    Ok(SkillPublishPayload { manifest, ts })
}

pub fn parse_skill_update_payload(value: &CborValue) -> Result<SkillUpdatePayload, Error> {
    let map = expect_map(value)?;
    let skill_id = expect_text(get_required(&map, 0)?)?;
    let prev_manifest_hash = expect_bytes_len(get_required(&map, 1)?, 32)?;
    let manifest = expect_bytes(get_required(&map, 2)?)?;
    let ts = expect_u64(get_required(&map, 3)?)?;
    if ts == 0 {
        return Err(Error::Cbor("timestamp required"));
    }
    decode_skill_manifest(&manifest)?;
    Ok(SkillUpdatePayload {
        skill_id,
        prev_manifest_hash,
        manifest,
        ts,
    })
}

pub fn parse_skill_revoke_payload(value: &CborValue) -> Result<SkillRevokePayload, Error> {
    let map = expect_map(value)?;
    let skill_id = expect_text(get_required(&map, 0)?)?;
    let manifest_hash = expect_bytes_len(get_required(&map, 1)?, 32)?;
    let reason = expect_text(get_required(&map, 2)?)?;
    let ts = expect_u64(get_required(&map, 3)?)?;
    if ts == 0 {
        return Err(Error::Cbor("timestamp required"));
    }
    if reason.trim().is_empty() {
        return Err(Error::Cbor("reason required"));
    }
    Ok(SkillRevokePayload {
        skill_id,
        manifest_hash,
        reason,
        ts,
    })
}

pub fn skill_publish_payload_to_cbor(payload: &SkillPublishPayload) -> Result<CborValue, Error> {
    if payload.ts == 0 {
        return Err(Error::Cbor("timestamp required"));
    }
    decode_skill_manifest(&payload.manifest)?;
    Ok(CborValue::Map(vec![
        (CborValue::Unsigned(0), CborValue::Bytes(payload.manifest.clone())),
        (CborValue::Unsigned(1), CborValue::Unsigned(payload.ts)),
    ]))
}

pub fn skill_update_payload_to_cbor(payload: &SkillUpdatePayload) -> Result<CborValue, Error> {
    if payload.ts == 0 {
        return Err(Error::Cbor("timestamp required"));
    }
    if payload.skill_id.trim().is_empty() {
        return Err(Error::Cbor("skill id required"));
    }
    if payload.prev_manifest_hash.len() != 32 {
        return Err(Error::Cbor("invalid manifest hash length"));
    }
    decode_skill_manifest(&payload.manifest)?;
    Ok(CborValue::Map(vec![
        (CborValue::Unsigned(0), CborValue::Text(payload.skill_id.clone())),
        (
            CborValue::Unsigned(1),
            CborValue::Bytes(payload.prev_manifest_hash.clone()),
        ),
        (CborValue::Unsigned(2), CborValue::Bytes(payload.manifest.clone())),
        (CborValue::Unsigned(3), CborValue::Unsigned(payload.ts)),
    ]))
}

pub fn skill_revoke_payload_to_cbor(payload: &SkillRevokePayload) -> Result<CborValue, Error> {
    if payload.ts == 0 {
        return Err(Error::Cbor("timestamp required"));
    }
    if payload.skill_id.trim().is_empty() {
        return Err(Error::Cbor("skill id required"));
    }
    if payload.reason.trim().is_empty() {
        return Err(Error::Cbor("reason required"));
    }
    if payload.manifest_hash.len() != 32 {
        return Err(Error::Cbor("invalid manifest hash length"));
    }
    Ok(CborValue::Map(vec![
        (CborValue::Unsigned(0), CborValue::Text(payload.skill_id.clone())),
        (CborValue::Unsigned(1), CborValue::Bytes(payload.manifest_hash.clone())),
        (CborValue::Unsigned(2), CborValue::Text(payload.reason.clone())),
        (CborValue::Unsigned(3), CborValue::Unsigned(payload.ts)),
    ]))
}

impl SkillArtifact {
    pub fn to_cbor(&self) -> Result<CborValue, Error> {
        ensure_nonzero(self.kind as u64, "artifact kind required")?;
        ensure_nonempty_list(&self.uris, "artifact uris required")?;
        ensure_nonzero(self.size, "artifact size required")?;
        if self.digest.len() != 32 {
            return Err(Error::Cbor("artifact digest must be 32 bytes"));
        }
        Ok(CborValue::Map(vec![
            (CborValue::Unsigned(0), CborValue::Unsigned(self.kind as u64)),
            (CborValue::Unsigned(1), CborValue::Bytes(self.digest.clone())),
            (CborValue::Unsigned(2), CborValue::Unsigned(self.size)),
            (
                CborValue::Unsigned(3),
                CborValue::Array(self.uris.iter().map(|s| CborValue::Text(s.clone())).collect()),
            ),
        ]))
    }
}

impl SkillManifestPayload {
    pub fn to_cbor(&self) -> Result<CborValue, Error> {
        self.validate()?;
        let mut entries = Vec::new();
        entries.push((CborValue::Unsigned(0), CborValue::Text(self.skill_id.clone())));
        entries.push((CborValue::Unsigned(1), CborValue::Text(self.author.clone())));
        entries.push((CborValue::Unsigned(2), CborValue::Text(self.name.clone())));
        entries.push((CborValue::Unsigned(3), CborValue::Text(self.version.clone())));
        entries.push((CborValue::Unsigned(4), CborValue::Text(self.summary.clone())));
        entries.push((CborValue::Unsigned(5), CborValue::Text(self.license.clone())));
        entries.push((
            CborValue::Unsigned(6),
            CborValue::Array(self.capabilities.iter().map(|s| CborValue::Text(s.clone())).collect()),
        ));
        entries.push((
            CborValue::Unsigned(7),
            CborValue::Array(self.permissions.iter().map(|s| CborValue::Text(s.clone())).collect()),
        ));
        entries.push((CborValue::Unsigned(8), CborValue::Unsigned(self.sandbox_class as u64)));
        if let Some(endpoints) = &self.endpoints {
            entries.push((
                CborValue::Unsigned(9),
                CborValue::Array(endpoints.iter().map(|s| CborValue::Text(s.clone())).collect()),
            ));
        }
        if let Some(artifacts) = &self.artifacts {
            let mut items = Vec::with_capacity(artifacts.len());
            for artifact in artifacts {
                items.push(artifact.to_cbor()?);
            }
            entries.push((CborValue::Unsigned(10), CborValue::Array(items)));
        }
        if let Some(reqs) = &self.requirements {
            entries.push((
                CborValue::Unsigned(11),
                CborValue::Array(reqs.iter().map(|s| CborValue::Text(s.clone())).collect()),
            ));
        }
        if let Some(pricing) = &self.pricing {
            entries.push((CborValue::Unsigned(12), pricing.clone()));
        }
        if let Some(attestations) = &self.attestations {
            entries.push((CborValue::Unsigned(13), attestations.clone()));
        }
        if let Some(metadata) = &self.metadata {
            entries.push((CborValue::Unsigned(14), metadata.clone()));
        }
        entries.push((CborValue::Unsigned(15), CborValue::Unsigned(self.ts)));
        Ok(CborValue::Map(entries))
    }

    pub fn validate(&self) -> Result<(), Error> {
        ensure_nonempty(&self.skill_id, "skill id required")?;
        ensure_nonempty(&self.author, "author required")?;
        ensure_nonempty(&self.name, "name required")?;
        ensure_nonempty(&self.version, "version required")?;
        ensure_nonempty(&self.summary, "summary required")?;
        ensure_nonempty(&self.license, "license required")?;
        ensure_nonempty_list(&self.capabilities, "capabilities required")?;
        ensure_list_items(&self.permissions, "permissions required")?;
        validate_sandbox(self.sandbox_class)?;

        if let Some(endpoints) = &self.endpoints {
            ensure_nonempty_list(endpoints, "endpoints required")?;
        }
        if let Some(artifacts) = &self.artifacts {
            if artifacts.is_empty() {
                return Err(Error::Cbor("artifacts required"));
            }
            for artifact in artifacts {
                artifact.to_cbor()?;
            }
        }
        if let Some(reqs) = &self.requirements {
            ensure_list_items(reqs, "requirements required")?;
        }
        if self.endpoints.is_none() && self.artifacts.is_none() {
            return Err(Error::Cbor("skill requires endpoints or artifacts"));
        }
        Ok(())
    }
}

fn parse_artifacts(value: &CborValue) -> Result<Vec<SkillArtifact>, Error> {
    match value {
        CborValue::Array(items) => {
            if items.is_empty() {
                return Err(Error::Cbor("artifacts required"));
            }
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                out.push(parse_artifact(item)?);
            }
            Ok(out)
        }
        _ => Err(Error::Cbor("expected artifact array")),
    }
}

fn parse_artifact(value: &CborValue) -> Result<SkillArtifact, Error> {
    let map = expect_map(value)?;
    let kind = expect_u8(get_required(&map, 0)?)?;
    let digest = expect_bytes_len(get_required(&map, 1)?, 32)?;
    let size = expect_u64(get_required(&map, 2)?)?;
    let uris = expect_text_array(get_required(&map, 3)?)?;
    let artifact = SkillArtifact {
        kind,
        digest,
        size,
        uris,
    };
    artifact.to_cbor()?;
    Ok(artifact)
}

fn validate_sandbox(class_id: u16) -> Result<(), Error> {
    if class_id < SANDBOX_MIN || class_id > SANDBOX_MAX {
        return Err(Error::Cbor("invalid sandbox class"));
    }
    Ok(())
}

fn ensure_nonempty(value: &str, field: &'static str) -> Result<(), Error> {
    if value.trim().is_empty() {
        return Err(Error::Cbor(field));
    }
    Ok(())
}

fn ensure_nonempty_list(values: &[String], field: &'static str) -> Result<(), Error> {
    if values.is_empty() {
        return Err(Error::Cbor(field));
    }
    ensure_list_items(values, field)
}

fn ensure_list_items(values: &[String], field: &'static str) -> Result<(), Error> {
    for item in values {
        if item.trim().is_empty() {
            return Err(Error::Cbor(field));
        }
    }
    Ok(())
}

fn ensure_nonzero(value: u64, field: &'static str) -> Result<(), Error> {
    if value == 0 {
        return Err(Error::Cbor(field));
    }
    Ok(())
}
