use crate::schema::{
    expect_bytes, expect_bytes_len, expect_map, expect_text, expect_text_array, expect_u64,
    get_optional, get_required,
};
use crate::signed::{split_signed_map, with_signature};
use crate::{
    decode_canonical, encode_canonical, sha256, sign_ed25519_hash, verify_ed25519_hash, CborValue,
    Error,
};

const WORK_SIG_KEY: u64 = 16;

#[derive(Debug, Clone)]
pub struct WorkMilestone {
    pub milestone_id: String,
    pub description: String,
    pub due_ts: u64,
    pub amount: u64,
    pub deliverable_hash: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct WorkOfferPayload {
    pub offer_id: String,
    pub issuer: String,
    pub title: String,
    pub summary: String,
    pub scope: String,
    pub budget_amount: u64,
    pub budget_currency: String,
    pub duration_sec: u64,
    pub deliverables: Vec<String>,
    pub requirements: Option<Vec<String>>,
    pub ts: u64,
    pub exp: u64,
}

#[derive(Debug, Clone)]
pub struct WorkOffer {
    pub payload: WorkOfferPayload,
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct WorkAgreementPayload {
    pub agreement_id: String,
    pub offer_id: String,
    pub issuer: String,
    pub counterparty: String,
    pub budget_amount: u64,
    pub budget_currency: String,
    pub start_ts: u64,
    pub end_ts: u64,
    pub deliverables: Vec<String>,
    pub milestones: Option<Vec<WorkMilestone>>,
    pub escrow_id: Option<String>,
    pub dispute_policy: Option<CborValue>,
    pub ts: u64,
}

#[derive(Debug, Clone)]
pub struct WorkAgreement {
    pub payload: WorkAgreementPayload,
    pub signature: Vec<u8>,
}

pub fn parse_work_offer_payload(value: &CborValue) -> Result<WorkOfferPayload, Error> {
    let map = expect_map(value)?;
    let offer_id = expect_text(get_required(&map, 0)?)?;
    let issuer = expect_text(get_required(&map, 1)?)?;
    let title = expect_text(get_required(&map, 2)?)?;
    let summary = expect_text(get_required(&map, 3)?)?;
    let scope = expect_text(get_required(&map, 4)?)?;
    let budget_amount = expect_u64(get_required(&map, 5)?)?;
    let budget_currency = expect_text(get_required(&map, 6)?)?;
    let duration_sec = expect_u64(get_required(&map, 7)?)?;
    let deliverables = expect_text_array(get_required(&map, 8)?)?;
    let requirements = match get_optional(&map, 9) {
        Some(value) => Some(expect_text_array(value)?),
        None => None,
    };
    let ts = expect_u64(get_required(&map, 10)?)?;
    let exp = expect_u64(get_required(&map, 11)?)?;
    let payload = WorkOfferPayload {
        offer_id,
        issuer,
        title,
        summary,
        scope,
        budget_amount,
        budget_currency,
        duration_sec,
        deliverables,
        requirements,
        ts,
        exp,
    };
    payload.validate()?;
    Ok(payload)
}

pub fn parse_work_offer(value: &CborValue) -> Result<WorkOffer, Error> {
    let (payload_entries, signature) = split_signed_map(value, WORK_SIG_KEY)?;
    let payload_value = CborValue::Map(payload_entries);
    let payload = parse_work_offer_payload(&payload_value)?;
    Ok(WorkOffer { payload, signature })
}

pub fn decode_work_offer(data: &[u8]) -> Result<WorkOffer, Error> {
    let value = decode_canonical(data)?;
    parse_work_offer(&value)
}

pub fn build_work_offer(payload: &WorkOfferPayload, secret_key: &[u8]) -> Result<Vec<u8>, Error> {
    payload.validate()?;
    let payload_value = payload.to_cbor()?;
    let payload_cbor = encode_canonical(&payload_value)?;
    let hash = sha256(&payload_cbor);
    let sig = sign_ed25519_hash(secret_key, &hash)?;
    let full = with_signature(&payload_value, WORK_SIG_KEY, sig)?;
    encode_canonical(&full)
}

pub fn verify_work_offer(data: &[u8], public_key: &[u8]) -> Result<WorkOfferPayload, Error> {
    let value = decode_canonical(data)?;
    let (payload_entries, signature) = split_signed_map(&value, WORK_SIG_KEY)?;
    let payload_value = CborValue::Map(payload_entries);
    let payload_cbor = encode_canonical(&payload_value)?;
    let hash = sha256(&payload_cbor);
    verify_ed25519_hash(public_key, &hash, &signature)?;
    parse_work_offer_payload(&payload_value)
}

pub fn parse_work_agreement_payload(value: &CborValue) -> Result<WorkAgreementPayload, Error> {
    let map = expect_map(value)?;
    let agreement_id = expect_text(get_required(&map, 0)?)?;
    let offer_id = expect_text(get_required(&map, 1)?)?;
    let issuer = expect_text(get_required(&map, 2)?)?;
    let counterparty = expect_text(get_required(&map, 3)?)?;
    let budget_amount = expect_u64(get_required(&map, 4)?)?;
    let budget_currency = expect_text(get_required(&map, 5)?)?;
    let start_ts = expect_u64(get_required(&map, 6)?)?;
    let end_ts = expect_u64(get_required(&map, 7)?)?;
    let deliverables = expect_text_array(get_required(&map, 8)?)?;
    let milestones = match get_optional(&map, 9) {
        Some(value) => Some(parse_milestones(value)?),
        None => None,
    };
    let escrow_id = match get_optional(&map, 10) {
        Some(value) => Some(expect_text(value)?),
        None => None,
    };
    let dispute_policy = get_optional(&map, 11).cloned();
    let ts = expect_u64(get_required(&map, 12)?)?;
    let payload = WorkAgreementPayload {
        agreement_id,
        offer_id,
        issuer,
        counterparty,
        budget_amount,
        budget_currency,
        start_ts,
        end_ts,
        deliverables,
        milestones,
        escrow_id,
        dispute_policy,
        ts,
    };
    payload.validate()?;
    Ok(payload)
}

pub fn parse_work_agreement(value: &CborValue) -> Result<WorkAgreement, Error> {
    let (payload_entries, signature) = split_signed_map(value, WORK_SIG_KEY)?;
    let payload_value = CborValue::Map(payload_entries);
    let payload = parse_work_agreement_payload(&payload_value)?;
    Ok(WorkAgreement { payload, signature })
}

pub fn decode_work_agreement(data: &[u8]) -> Result<WorkAgreement, Error> {
    let value = decode_canonical(data)?;
    parse_work_agreement(&value)
}

pub fn build_work_agreement(
    payload: &WorkAgreementPayload,
    secret_key: &[u8],
) -> Result<Vec<u8>, Error> {
    payload.validate()?;
    let payload_value = payload.to_cbor()?;
    let payload_cbor = encode_canonical(&payload_value)?;
    let hash = sha256(&payload_cbor);
    let sig = sign_ed25519_hash(secret_key, &hash)?;
    let full = with_signature(&payload_value, WORK_SIG_KEY, sig)?;
    encode_canonical(&full)
}

pub fn verify_work_agreement(
    data: &[u8],
    public_key: &[u8],
) -> Result<WorkAgreementPayload, Error> {
    let value = decode_canonical(data)?;
    let (payload_entries, signature) = split_signed_map(&value, WORK_SIG_KEY)?;
    let payload_value = CborValue::Map(payload_entries);
    let payload_cbor = encode_canonical(&payload_value)?;
    let hash = sha256(&payload_cbor);
    verify_ed25519_hash(public_key, &hash, &signature)?;
    parse_work_agreement_payload(&payload_value)
}

impl WorkMilestone {
    pub fn to_cbor(&self) -> Result<CborValue, Error> {
        ensure_nonempty(&self.milestone_id, "milestone id required")?;
        ensure_nonempty(&self.description, "milestone description required")?;
        ensure_positive(self.due_ts, "milestone due_ts required")?;
        ensure_positive(self.amount, "milestone amount required")?;
        let mut entries = Vec::new();
        entries.push((
            CborValue::Unsigned(0),
            CborValue::Text(self.milestone_id.clone()),
        ));
        entries.push((
            CborValue::Unsigned(1),
            CborValue::Text(self.description.clone()),
        ));
        entries.push((CborValue::Unsigned(2), CborValue::Unsigned(self.due_ts)));
        entries.push((CborValue::Unsigned(3), CborValue::Unsigned(self.amount)));
        if let Some(hash) = &self.deliverable_hash {
            if hash.len() != 32 {
                return Err(Error::Cbor("deliverable hash must be 32 bytes"));
            }
            entries.push((CborValue::Unsigned(4), CborValue::Bytes(hash.clone())));
        }
        Ok(CborValue::Map(entries))
    }
}

impl WorkOfferPayload {
    pub fn to_cbor(&self) -> Result<CborValue, Error> {
        self.validate()?;
        let mut entries = Vec::new();
        entries.push((
            CborValue::Unsigned(0),
            CborValue::Text(self.offer_id.clone()),
        ));
        entries.push((CborValue::Unsigned(1), CborValue::Text(self.issuer.clone())));
        entries.push((CborValue::Unsigned(2), CborValue::Text(self.title.clone())));
        entries.push((
            CborValue::Unsigned(3),
            CborValue::Text(self.summary.clone()),
        ));
        entries.push((CborValue::Unsigned(4), CborValue::Text(self.scope.clone())));
        entries.push((
            CborValue::Unsigned(5),
            CborValue::Unsigned(self.budget_amount),
        ));
        entries.push((
            CborValue::Unsigned(6),
            CborValue::Text(self.budget_currency.clone()),
        ));
        entries.push((
            CborValue::Unsigned(7),
            CborValue::Unsigned(self.duration_sec),
        ));
        entries.push((
            CborValue::Unsigned(8),
            CborValue::Array(
                self.deliverables
                    .iter()
                    .map(|s| CborValue::Text(s.clone()))
                    .collect(),
            ),
        ));
        if let Some(reqs) = &self.requirements {
            entries.push((
                CborValue::Unsigned(9),
                CborValue::Array(reqs.iter().map(|s| CborValue::Text(s.clone())).collect()),
            ));
        }
        entries.push((CborValue::Unsigned(10), CborValue::Unsigned(self.ts)));
        entries.push((CborValue::Unsigned(11), CborValue::Unsigned(self.exp)));
        Ok(CborValue::Map(entries))
    }

    pub fn validate(&self) -> Result<(), Error> {
        ensure_nonempty(&self.offer_id, "offer id required")?;
        ensure_nonempty(&self.issuer, "issuer required")?;
        ensure_nonempty(&self.title, "title required")?;
        ensure_nonempty(&self.summary, "summary required")?;
        ensure_nonempty(&self.scope, "scope required")?;
        ensure_positive(self.budget_amount, "budget amount required")?;
        ensure_nonempty(&self.budget_currency, "budget currency required")?;
        ensure_positive(self.duration_sec, "duration required")?;
        ensure_nonempty_list(&self.deliverables, "deliverables required")?;
        if let Some(reqs) = &self.requirements {
            ensure_list_items(reqs, "requirements required")?;
        }
        ensure_positive(self.ts, "timestamp required")?;
        if self.exp <= self.ts {
            return Err(Error::Cbor("expiry must be after timestamp"));
        }
        Ok(())
    }
}

impl WorkAgreementPayload {
    pub fn to_cbor(&self) -> Result<CborValue, Error> {
        self.validate()?;
        let mut entries = Vec::new();
        entries.push((
            CborValue::Unsigned(0),
            CborValue::Text(self.agreement_id.clone()),
        ));
        entries.push((
            CborValue::Unsigned(1),
            CborValue::Text(self.offer_id.clone()),
        ));
        entries.push((CborValue::Unsigned(2), CborValue::Text(self.issuer.clone())));
        entries.push((
            CborValue::Unsigned(3),
            CborValue::Text(self.counterparty.clone()),
        ));
        entries.push((
            CborValue::Unsigned(4),
            CborValue::Unsigned(self.budget_amount),
        ));
        entries.push((
            CborValue::Unsigned(5),
            CborValue::Text(self.budget_currency.clone()),
        ));
        entries.push((CborValue::Unsigned(6), CborValue::Unsigned(self.start_ts)));
        entries.push((CborValue::Unsigned(7), CborValue::Unsigned(self.end_ts)));
        entries.push((
            CborValue::Unsigned(8),
            CborValue::Array(
                self.deliverables
                    .iter()
                    .map(|s| CborValue::Text(s.clone()))
                    .collect(),
            ),
        ));
        if let Some(milestones) = &self.milestones {
            let mut items = Vec::with_capacity(milestones.len());
            for milestone in milestones {
                items.push(milestone.to_cbor()?);
            }
            entries.push((CborValue::Unsigned(9), CborValue::Array(items)));
        }
        if let Some(escrow_id) = &self.escrow_id {
            entries.push((CborValue::Unsigned(10), CborValue::Text(escrow_id.clone())));
        }
        if let Some(dispute_policy) = &self.dispute_policy {
            entries.push((CborValue::Unsigned(11), dispute_policy.clone()));
        }
        entries.push((CborValue::Unsigned(12), CborValue::Unsigned(self.ts)));
        Ok(CborValue::Map(entries))
    }

    pub fn validate(&self) -> Result<(), Error> {
        ensure_nonempty(&self.agreement_id, "agreement id required")?;
        ensure_nonempty(&self.offer_id, "offer id required")?;
        ensure_nonempty(&self.issuer, "issuer required")?;
        ensure_nonempty(&self.counterparty, "counterparty required")?;
        ensure_positive(self.budget_amount, "budget amount required")?;
        ensure_nonempty(&self.budget_currency, "budget currency required")?;
        ensure_positive(self.start_ts, "start_ts required")?;
        ensure_positive(self.end_ts, "end_ts required")?;
        if self.end_ts <= self.start_ts {
            return Err(Error::Cbor("end_ts must be after start_ts"));
        }
        ensure_nonempty_list(&self.deliverables, "deliverables required")?;
        if let Some(milestones) = &self.milestones {
            if milestones.is_empty() {
                return Err(Error::Cbor("milestones required"));
            }
            for milestone in milestones {
                milestone.to_cbor()?;
            }
        }
        if let Some(escrow_id) = &self.escrow_id {
            ensure_nonempty(escrow_id, "escrow id required")?;
        }
        ensure_positive(self.ts, "timestamp required")?;
        Ok(())
    }
}

fn parse_milestones(value: &CborValue) -> Result<Vec<WorkMilestone>, Error> {
    match value {
        CborValue::Array(items) => {
            if items.is_empty() {
                return Err(Error::Cbor("milestones required"));
            }
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                out.push(parse_milestone(item)?);
            }
            Ok(out)
        }
        _ => Err(Error::Cbor("expected milestone array")),
    }
}

fn parse_milestone(value: &CborValue) -> Result<WorkMilestone, Error> {
    let map = expect_map(value)?;
    let milestone_id = expect_text(get_required(&map, 0)?)?;
    let description = expect_text(get_required(&map, 1)?)?;
    let due_ts = expect_u64(get_required(&map, 2)?)?;
    let amount = expect_u64(get_required(&map, 3)?)?;
    let deliverable_hash = match get_optional(&map, 4) {
        Some(value) => Some(expect_bytes_len(value, 32)?),
        None => None,
    };
    let milestone = WorkMilestone {
        milestone_id,
        description,
        due_ts,
        amount,
        deliverable_hash,
    };
    milestone.to_cbor()?;
    Ok(milestone)
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

fn ensure_positive(value: u64, field: &'static str) -> Result<(), Error> {
    if value == 0 {
        return Err(Error::Cbor(field));
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct WorkOfferPublishPayload {
    pub offer: Vec<u8>,
    pub ts: u64,
}

#[derive(Debug, Clone)]
pub struct WorkAgreementPublishPayload {
    pub agreement: Vec<u8>,
    pub ts: u64,
}

#[derive(Debug, Clone)]
pub struct WorkAgreementUpdatePayload {
    pub agreement_id: String,
    pub prev_agreement_hash: Vec<u8>,
    pub agreement: Vec<u8>,
    pub ts: u64,
}

#[derive(Debug, Clone)]
pub struct WorkAgreementClosePayload {
    pub agreement_id: String,
    pub agreement_hash: Vec<u8>,
    pub reason: String,
    pub ts: u64,
}

pub fn parse_work_offer_publish_payload(
    value: &CborValue,
) -> Result<WorkOfferPublishPayload, Error> {
    let map = expect_map(value)?;
    let offer = expect_bytes(get_required(&map, 0)?)?;
    let ts = expect_u64(get_required(&map, 1)?)?;
    if ts == 0 {
        return Err(Error::Cbor("timestamp required"));
    }
    decode_work_offer(&offer)?;
    Ok(WorkOfferPublishPayload { offer, ts })
}

pub fn parse_work_agreement_publish_payload(
    value: &CborValue,
) -> Result<WorkAgreementPublishPayload, Error> {
    let map = expect_map(value)?;
    let agreement = expect_bytes(get_required(&map, 0)?)?;
    let ts = expect_u64(get_required(&map, 1)?)?;
    if ts == 0 {
        return Err(Error::Cbor("timestamp required"));
    }
    decode_work_agreement(&agreement)?;
    Ok(WorkAgreementPublishPayload { agreement, ts })
}

pub fn parse_work_agreement_update_payload(
    value: &CborValue,
) -> Result<WorkAgreementUpdatePayload, Error> {
    let map = expect_map(value)?;
    let agreement_id = expect_text(get_required(&map, 0)?)?;
    let prev_agreement_hash = expect_bytes_len(get_required(&map, 1)?, 32)?;
    let agreement = expect_bytes(get_required(&map, 2)?)?;
    let ts = expect_u64(get_required(&map, 3)?)?;
    if ts == 0 {
        return Err(Error::Cbor("timestamp required"));
    }
    decode_work_agreement(&agreement)?;
    Ok(WorkAgreementUpdatePayload {
        agreement_id,
        prev_agreement_hash,
        agreement,
        ts,
    })
}

pub fn parse_work_agreement_close_payload(
    value: &CborValue,
) -> Result<WorkAgreementClosePayload, Error> {
    let map = expect_map(value)?;
    let agreement_id = expect_text(get_required(&map, 0)?)?;
    let agreement_hash = expect_bytes_len(get_required(&map, 1)?, 32)?;
    let reason = expect_text(get_required(&map, 2)?)?;
    let ts = expect_u64(get_required(&map, 3)?)?;
    if ts == 0 {
        return Err(Error::Cbor("timestamp required"));
    }
    if reason.trim().is_empty() {
        return Err(Error::Cbor("reason required"));
    }
    Ok(WorkAgreementClosePayload {
        agreement_id,
        agreement_hash,
        reason,
        ts,
    })
}

pub fn work_offer_publish_payload_to_cbor(
    payload: &WorkOfferPublishPayload,
) -> Result<CborValue, Error> {
    if payload.ts == 0 {
        return Err(Error::Cbor("timestamp required"));
    }
    decode_work_offer(&payload.offer)?;
    Ok(CborValue::Map(vec![
        (
            CborValue::Unsigned(0),
            CborValue::Bytes(payload.offer.clone()),
        ),
        (CborValue::Unsigned(1), CborValue::Unsigned(payload.ts)),
    ]))
}

pub fn work_agreement_publish_payload_to_cbor(
    payload: &WorkAgreementPublishPayload,
) -> Result<CborValue, Error> {
    if payload.ts == 0 {
        return Err(Error::Cbor("timestamp required"));
    }
    decode_work_agreement(&payload.agreement)?;
    Ok(CborValue::Map(vec![
        (
            CborValue::Unsigned(0),
            CborValue::Bytes(payload.agreement.clone()),
        ),
        (CborValue::Unsigned(1), CborValue::Unsigned(payload.ts)),
    ]))
}

pub fn work_agreement_update_payload_to_cbor(
    payload: &WorkAgreementUpdatePayload,
) -> Result<CborValue, Error> {
    if payload.ts == 0 {
        return Err(Error::Cbor("timestamp required"));
    }
    if payload.agreement_id.trim().is_empty() {
        return Err(Error::Cbor("agreement id required"));
    }
    if payload.prev_agreement_hash.len() != 32 {
        return Err(Error::Cbor("invalid agreement hash length"));
    }
    decode_work_agreement(&payload.agreement)?;
    Ok(CborValue::Map(vec![
        (
            CborValue::Unsigned(0),
            CborValue::Text(payload.agreement_id.clone()),
        ),
        (
            CborValue::Unsigned(1),
            CborValue::Bytes(payload.prev_agreement_hash.clone()),
        ),
        (
            CborValue::Unsigned(2),
            CborValue::Bytes(payload.agreement.clone()),
        ),
        (CborValue::Unsigned(3), CborValue::Unsigned(payload.ts)),
    ]))
}

pub fn work_agreement_close_payload_to_cbor(
    payload: &WorkAgreementClosePayload,
) -> Result<CborValue, Error> {
    if payload.ts == 0 {
        return Err(Error::Cbor("timestamp required"));
    }
    if payload.agreement_id.trim().is_empty() {
        return Err(Error::Cbor("agreement id required"));
    }
    if payload.reason.trim().is_empty() {
        return Err(Error::Cbor("reason required"));
    }
    if payload.agreement_hash.len() != 32 {
        return Err(Error::Cbor("invalid agreement hash length"));
    }
    Ok(CborValue::Map(vec![
        (
            CborValue::Unsigned(0),
            CborValue::Text(payload.agreement_id.clone()),
        ),
        (
            CborValue::Unsigned(1),
            CborValue::Bytes(payload.agreement_hash.clone()),
        ),
        (
            CborValue::Unsigned(2),
            CborValue::Text(payload.reason.clone()),
        ),
        (CborValue::Unsigned(3), CborValue::Unsigned(payload.ts)),
    ]))
}
