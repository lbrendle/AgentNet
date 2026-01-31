use anyhow::{anyhow, Context, Result};
use anetsdk::{
    decode_canonical, encode_canonical, parse_action_intent, parse_agentmail_message,
    parse_agentmail_payload, parse_approval_payload, parse_grant_payload, parse_nodehello_payload,
    parse_receipt_payload, parse_skill_manifest_payload, parse_skill_publish_payload,
    parse_skill_revoke_payload, parse_skill_update_payload, parse_tx_envelope_payload,
    parse_work_agreement_close_payload, parse_work_agreement_payload,
    parse_work_agreement_publish_payload, parse_work_agreement_update_payload,
    parse_work_offer_payload, parse_work_offer_publish_payload, payload_from_parsed, sha256,
    sign_ed25519_hash, verify_ed25519_hash, verify_skill_manifest, verify_work_agreement,
    verify_work_offer, CborValue,
};
use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
struct VectorsFile {
    ed25519_public_key_hex: String,
    ed25519_seed_hex: Option<String>,
    vectors: Vec<VectorEntry>,
}

#[derive(Debug, Deserialize)]
struct VectorEntry {
    id: String,
    #[serde(default)]
    object_cbor_hex: Option<String>,
    #[serde(default)]
    sha256_hex: Option<String>,
    #[serde(default)]
    signature_hex: Option<String>,

    #[serde(default)]
    approval_payload_cbor_hex: Option<String>,
    #[serde(default)]
    approval_payload_sha256_hex: Option<String>,
    #[serde(default)]
    approval_signature_hex: Option<String>,
    #[serde(default)]
    approval_full_object_cbor_hex: Option<String>,
    #[serde(default)]
    intent_hash_hex: Option<String>,

    #[serde(default)]
    grant_payload_cbor_hex: Option<String>,
    #[serde(default)]
    grant_payload_sha256_hex: Option<String>,
    #[serde(default)]
    grant_signature_hex: Option<String>,
    #[serde(default)]
    grant_full_object_cbor_hex: Option<String>,

    #[serde(default)]
    nodehello_payload_cbor_hex: Option<String>,
    #[serde(default)]
    nodehello_payload_sha256_hex: Option<String>,
    #[serde(default)]
    nodehello_signature_hex: Option<String>,

    #[serde(default)]
    receipt1_payload_cbor_hex: Option<String>,
    #[serde(default)]
    receipt1_hash_hex: Option<String>,
    #[serde(default)]
    receipt1_sig_hex: Option<String>,
    #[serde(default)]
    receipt2_payload_cbor_hex: Option<String>,
    #[serde(default)]
    receipt2_hash_hex: Option<String>,
    #[serde(default)]
    receipt2_sig_hex: Option<String>,
    #[serde(default)]
    receipt2_prev_hash_hex: Option<String>,

    #[serde(default)]
    tx_envelope_payload_cbor_hex: Option<String>,
    #[serde(default)]
    tx_envelope_payload_sha256_hex: Option<String>,
    #[serde(default)]
    tx_signature_hex: Option<String>,

    #[serde(default)]
    skill_manifest_full_object_cbor_hex: Option<String>,
    #[serde(default)]
    work_offer_full_object_cbor_hex: Option<String>,
    #[serde(default)]
    work_agreement_full_object_cbor_hex: Option<String>,

    #[serde(default)]
    skill_publish_payload_cbor_hex: Option<String>,
    #[serde(default)]
    skill_publish_payload_sha256_hex: Option<String>,
    #[serde(default)]
    skill_update_payload_cbor_hex: Option<String>,
    #[serde(default)]
    skill_update_payload_sha256_hex: Option<String>,
    #[serde(default)]
    skill_revoke_payload_cbor_hex: Option<String>,
    #[serde(default)]
    skill_revoke_payload_sha256_hex: Option<String>,

    #[serde(default)]
    work_offer_payload_cbor_hex: Option<String>,
    #[serde(default)]
    work_offer_payload_sha256_hex: Option<String>,
    #[serde(default)]
    work_offer_signature_hex: Option<String>,

    #[serde(default)]
    work_agreement_payload_cbor_hex: Option<String>,
    #[serde(default)]
    work_agreement_payload_sha256_hex: Option<String>,
    #[serde(default)]
    work_agreement_signature_hex: Option<String>,

    #[serde(default)]
    work_offer_publish_payload_cbor_hex: Option<String>,
    #[serde(default)]
    work_offer_publish_payload_sha256_hex: Option<String>,
    #[serde(default)]
    work_agreement_publish_payload_cbor_hex: Option<String>,
    #[serde(default)]
    work_agreement_publish_payload_sha256_hex: Option<String>,
    #[serde(default)]
    work_agreement_update_payload_cbor_hex: Option<String>,
    #[serde(default)]
    work_agreement_update_payload_sha256_hex: Option<String>,
    #[serde(default)]
    work_agreement_close_payload_cbor_hex: Option<String>,
    #[serde(default)]
    work_agreement_close_payload_sha256_hex: Option<String>,

    #[serde(default)]
    kill_switch_payload_cbor_hex: Option<String>,
    #[serde(default)]
    kill_switch_payload_sha256_hex: Option<String>,
    #[serde(default)]
    kill_switch_signature_hex: Option<String>,
    #[serde(default)]
    kill_switch_full_object_cbor_hex: Option<String>,

    #[serde(default)]
    agentmail_payload_cbor_hex: Option<String>,
    #[serde(default)]
    agentmail_payload_sha256_hex: Option<String>,
    #[serde(default)]
    agentmail_signature_hex: Option<String>,
    #[serde(default)]
    agentmail_full_object_cbor_hex: Option<String>,
}

fn main() -> Result<()> {
    let path = std::env::args().nth(1).ok_or_else(|| anyhow!("usage: anet-vectors <path-to-vectors.json>"))?;
    let data = fs::read_to_string(&path).with_context(|| format!("read {path}"))?;
    let vectors: VectorsFile = serde_json::from_str(&data).context("parse vectors json")?;

    let public_key = hex::decode(&vectors.ed25519_public_key_hex).context("decode public key")?;
    let seed = vectors.ed25519_seed_hex.map(|hex| hex::decode(hex)).transpose().context("decode seed")?;

    let mut action_intent_hash: Option<Vec<u8>> = None;

    for entry in vectors.vectors {
        match entry.id.as_str() {
            "TV1_ActionIntent" => {
                let cbor = decode_hex_required(entry.object_cbor_hex, "object_cbor_hex")?;
                let expected_hash = decode_hex_required(entry.sha256_hex, "sha256_hex")?;
                let expected_sig = decode_hex_required(entry.signature_hex, "signature_hex")?;

                verify_hash_and_sig("TV1_ActionIntent", &public_key, &cbor, &expected_hash, &expected_sig)?;
                action_intent_hash = Some(expected_hash);
                ensure_roundtrip("TV1_ActionIntent", &cbor)?;
                let parsed = decode_canonical(&cbor)?;
                parse_action_intent(&parsed)?;
            }
            "TV2_Approval" => {
                let payload = decode_hex_required(entry.approval_payload_cbor_hex, "approval_payload_cbor_hex")?;
                let expected_hash = decode_hex_required(entry.approval_payload_sha256_hex, "approval_payload_sha256_hex")?;
                let expected_sig = decode_hex_required(entry.approval_signature_hex, "approval_signature_hex")?;
                let intent_hash = decode_hex_required(entry.intent_hash_hex, "intent_hash_hex")?;
                if let Some(full_hex) = entry.approval_full_object_cbor_hex {
                    let full = hex::decode(&full_hex).context("decode approval_full_object_cbor_hex")?;
                    ensure_roundtrip("TV2_Approval full object", &full)?;
                }

                verify_hash_and_sig("TV2_Approval", &public_key, &payload, &expected_hash, &expected_sig)?;
                ensure_roundtrip("TV2_Approval", &payload)?;
                let parsed = decode_canonical(&payload)?;
                parse_approval_payload(&parsed)?;

                if let Some(action_hash) = &action_intent_hash {
                    if action_hash != &intent_hash {
                        return Err(anyhow!("TV2_Approval intent hash mismatch"));
                    }
                }
            }
            "TV3_Grant" => {
                let payload = decode_hex_required(entry.grant_payload_cbor_hex, "grant_payload_cbor_hex")?;
                let expected_hash = decode_hex_required(entry.grant_payload_sha256_hex, "grant_payload_sha256_hex")?;
                let expected_sig = decode_hex_required(entry.grant_signature_hex, "grant_signature_hex")?;
                if let Some(full_hex) = entry.grant_full_object_cbor_hex {
                    let full = hex::decode(&full_hex).context("decode grant_full_object_cbor_hex")?;
                    ensure_roundtrip("TV3_Grant full object", &full)?;
                }

                verify_hash_and_sig("TV3_Grant", &public_key, &payload, &expected_hash, &expected_sig)?;
                ensure_roundtrip("TV3_Grant", &payload)?;
                let parsed = decode_canonical(&payload)?;
                parse_grant_payload(&parsed)?;
            }
            "TV4_NodeHello" => {
                let payload = decode_hex_required(entry.nodehello_payload_cbor_hex, "nodehello_payload_cbor_hex")?;
                let expected_hash = decode_hex_required(entry.nodehello_payload_sha256_hex, "nodehello_payload_sha256_hex")?;
                let expected_sig = decode_hex_required(entry.nodehello_signature_hex, "nodehello_signature_hex")?;

                verify_hash_and_sig("TV4_NodeHello", &public_key, &payload, &expected_hash, &expected_sig)?;
                ensure_roundtrip("TV4_NodeHello", &payload)?;
                let parsed = decode_canonical(&payload)?;
                let node = parse_nodehello_payload(&parsed)?;
                if let Some(seed) = &seed {
                    let payload_obj = payload_from_parsed(&node);
                    let sig = sign_ed25519_hash(seed, &expected_hash)?;
                    if sig != expected_sig {
                        return Err(anyhow!("TV4_NodeHello signature mismatch from signer"));
                    }
                    let payload_cbor = encode_canonical(&payload_obj.to_cbor())?;
                    if payload_cbor != payload {
                        return Err(anyhow!("TV4_NodeHello payload regeneration mismatch"));
                    }
                }
            }
            "TV5_ReceiptChain" => {
                let receipt1 = decode_hex_required(entry.receipt1_payload_cbor_hex, "receipt1_payload_cbor_hex")?;
                let receipt1_hash = decode_hex_required(entry.receipt1_hash_hex, "receipt1_hash_hex")?;
                let receipt1_sig = decode_hex_required(entry.receipt1_sig_hex, "receipt1_sig_hex")?;
                let receipt2 = decode_hex_required(entry.receipt2_payload_cbor_hex, "receipt2_payload_cbor_hex")?;
                let receipt2_hash = decode_hex_required(entry.receipt2_hash_hex, "receipt2_hash_hex")?;
                let receipt2_sig = decode_hex_required(entry.receipt2_sig_hex, "receipt2_sig_hex")?;
                let receipt2_prev = decode_hex_required(entry.receipt2_prev_hash_hex, "receipt2_prev_hash_hex")?;

                verify_hash_and_sig("TV5_ReceiptChain receipt1", &public_key, &receipt1, &receipt1_hash, &receipt1_sig)?;
                verify_hash_and_sig("TV5_ReceiptChain receipt2", &public_key, &receipt2, &receipt2_hash, &receipt2_sig)?;

                if receipt2_prev != receipt1_hash {
                    return Err(anyhow!("TV5_ReceiptChain prev_hash mismatch"));
                }
                ensure_roundtrip("TV5_ReceiptChain receipt1", &receipt1)?;
                ensure_roundtrip("TV5_ReceiptChain receipt2", &receipt2)?;
                let parsed1 = decode_canonical(&receipt1)?;
                let parsed2 = decode_canonical(&receipt2)?;
                parse_receipt_payload(&parsed1)?;
                parse_receipt_payload(&parsed2)?;
            }
            "TV6_EscrowLockTx" => {
                let payload = decode_hex_required(entry.tx_envelope_payload_cbor_hex, "tx_envelope_payload_cbor_hex")?;
                let expected_hash = decode_hex_required(entry.tx_envelope_payload_sha256_hex, "tx_envelope_payload_sha256_hex")?;
                let expected_sig = decode_hex_required(entry.tx_signature_hex, "tx_signature_hex")?;

                verify_hash_and_sig("TV6_EscrowLockTx", &public_key, &payload, &expected_hash, &expected_sig)?;
                ensure_roundtrip("TV6_EscrowLockTx", &payload)?;
                let parsed = decode_canonical(&payload)?;
                parse_tx_envelope_payload(&parsed)?;
            }
            "TV7_KillSwitch" => {
                let payload = decode_hex_required(entry.kill_switch_payload_cbor_hex, "kill_switch_payload_cbor_hex")?;
                let expected_hash = decode_hex_required(entry.kill_switch_payload_sha256_hex, "kill_switch_payload_sha256_hex")?;
                let expected_sig = decode_hex_required(entry.kill_switch_signature_hex, "kill_switch_signature_hex")?;
                let full = decode_hex_required(entry.kill_switch_full_object_cbor_hex, "kill_switch_full_object_cbor_hex")?;

                verify_hash_and_sig("TV7_KillSwitch", &public_key, &payload, &expected_hash, &expected_sig)?;
                ensure_roundtrip("TV7_KillSwitch payload", &payload)?;
                ensure_roundtrip("TV7_KillSwitch full object", &full)?;

                let payload_value = decode_canonical(&payload)?;
                let payload_parts = parse_kill_switch_map(&payload_value)?;
                if payload_parts.signature.is_some() {
                    return Err(anyhow!("TV7_KillSwitch payload must not include signature"));
                }
                ensure_kill_switch_parts("TV7_KillSwitch payload", &payload_parts, false)?;

                let full_value = decode_canonical(&full)?;
                let full_parts = parse_kill_switch_map(&full_value)?;
                ensure_kill_switch_parts("TV7_KillSwitch full object", &full_parts, true)?;
                let full_sig = full_parts
                    .signature
                    .ok_or_else(|| anyhow!("TV7_KillSwitch full object missing signature"))?;
                if full_sig != expected_sig {
                    return Err(anyhow!("TV7_KillSwitch signature mismatch"));
                }
                if payload_parts.action != full_parts.action
                    || payload_parts.reason != full_parts.reason
                    || payload_parts.ts != full_parts.ts
                    || payload_parts.nonce != full_parts.nonce
                {
                    return Err(anyhow!("TV7_KillSwitch full object fields mismatch"));
                }
                let reconstructed = encode_canonical(&kill_switch_payload_map(&payload_parts))?;
                if reconstructed != payload {
                    return Err(anyhow!("TV7_KillSwitch payload reconstruction mismatch"));
                }
            }
            "TV8_SkillManifest" => {
                let payload = decode_hex_required(entry.object_cbor_hex, "object_cbor_hex")?;
                let expected_hash = decode_hex_required(entry.sha256_hex, "sha256_hex")?;
                let expected_sig = decode_hex_required(entry.signature_hex, "signature_hex")?;
                verify_hash_and_sig("TV8_SkillManifest", &public_key, &payload, &expected_hash, &expected_sig)?;
                ensure_roundtrip("TV8_SkillManifest", &payload)?;
                let parsed = decode_canonical(&payload)?;
                parse_skill_manifest_payload(&parsed)?;
                if let Some(full_hex) = entry.skill_manifest_full_object_cbor_hex {
                    let full = hex::decode(&full_hex).context("decode skill_manifest_full_object_cbor_hex")?;
                    ensure_roundtrip("TV8_SkillManifest full object", &full)?;
                    verify_skill_manifest(&full, &public_key)?;
                }
            }
            "TV9_WorkOffer" => {
                let payload = decode_hex_required(entry.work_offer_payload_cbor_hex, "work_offer_payload_cbor_hex")?;
                let expected_hash = decode_hex_required(entry.work_offer_payload_sha256_hex, "work_offer_payload_sha256_hex")?;
                let expected_sig = decode_hex_required(entry.work_offer_signature_hex, "work_offer_signature_hex")?;
                verify_hash_and_sig("TV9_WorkOffer", &public_key, &payload, &expected_hash, &expected_sig)?;
                ensure_roundtrip("TV9_WorkOffer", &payload)?;
                let parsed = decode_canonical(&payload)?;
                parse_work_offer_payload(&parsed)?;
                if let Some(full_hex) = entry.work_offer_full_object_cbor_hex {
                    let full = hex::decode(&full_hex).context("decode work_offer_full_object_cbor_hex")?;
                    ensure_roundtrip("TV9_WorkOffer full object", &full)?;
                    verify_work_offer(&full, &public_key)?;
                }
            }
            "TV10_WorkAgreement" => {
                let payload = decode_hex_required(entry.work_agreement_payload_cbor_hex, "work_agreement_payload_cbor_hex")?;
                let expected_hash = decode_hex_required(entry.work_agreement_payload_sha256_hex, "work_agreement_payload_sha256_hex")?;
                let expected_sig = decode_hex_required(entry.work_agreement_signature_hex, "work_agreement_signature_hex")?;
                verify_hash_and_sig("TV10_WorkAgreement", &public_key, &payload, &expected_hash, &expected_sig)?;
                ensure_roundtrip("TV10_WorkAgreement", &payload)?;
                let parsed = decode_canonical(&payload)?;
                parse_work_agreement_payload(&parsed)?;
                if let Some(full_hex) = entry.work_agreement_full_object_cbor_hex {
                    let full = hex::decode(&full_hex).context("decode work_agreement_full_object_cbor_hex")?;
                    ensure_roundtrip("TV10_WorkAgreement full object", &full)?;
                    verify_work_agreement(&full, &public_key)?;
                }
            }
            "TV11_SkillPublishPayload" => {
                let payload = decode_hex_required(entry.skill_publish_payload_cbor_hex, "skill_publish_payload_cbor_hex")?;
                let expected_hash = decode_hex_required(entry.skill_publish_payload_sha256_hex, "skill_publish_payload_sha256_hex")?;
                let digest = sha256(&payload);
                if digest.as_slice() != expected_hash {
                    return Err(anyhow!("TV11_SkillPublishPayload hash mismatch"));
                }
                ensure_roundtrip("TV11_SkillPublishPayload", &payload)?;
                let parsed = decode_canonical(&payload)?;
                parse_skill_publish_payload(&parsed)?;
            }
            "TV12_SkillUpdatePayload" => {
                let payload = decode_hex_required(entry.skill_update_payload_cbor_hex, "skill_update_payload_cbor_hex")?;
                let expected_hash = decode_hex_required(entry.skill_update_payload_sha256_hex, "skill_update_payload_sha256_hex")?;
                let digest = sha256(&payload);
                if digest.as_slice() != expected_hash {
                    return Err(anyhow!("TV12_SkillUpdatePayload hash mismatch"));
                }
                ensure_roundtrip("TV12_SkillUpdatePayload", &payload)?;
                let parsed = decode_canonical(&payload)?;
                parse_skill_update_payload(&parsed)?;
            }
            "TV13_SkillRevokePayload" => {
                let payload = decode_hex_required(entry.skill_revoke_payload_cbor_hex, "skill_revoke_payload_cbor_hex")?;
                let expected_hash = decode_hex_required(entry.skill_revoke_payload_sha256_hex, "skill_revoke_payload_sha256_hex")?;
                let digest = sha256(&payload);
                if digest.as_slice() != expected_hash {
                    return Err(anyhow!("TV13_SkillRevokePayload hash mismatch"));
                }
                ensure_roundtrip("TV13_SkillRevokePayload", &payload)?;
                let parsed = decode_canonical(&payload)?;
                parse_skill_revoke_payload(&parsed)?;
            }
            "TV14_WorkOfferPublishPayload" => {
                let payload = decode_hex_required(entry.work_offer_publish_payload_cbor_hex, "work_offer_publish_payload_cbor_hex")?;
                let expected_hash = decode_hex_required(entry.work_offer_publish_payload_sha256_hex, "work_offer_publish_payload_sha256_hex")?;
                let digest = sha256(&payload);
                if digest.as_slice() != expected_hash {
                    return Err(anyhow!("TV14_WorkOfferPublishPayload hash mismatch"));
                }
                ensure_roundtrip("TV14_WorkOfferPublishPayload", &payload)?;
                let parsed = decode_canonical(&payload)?;
                parse_work_offer_publish_payload(&parsed)?;
            }
            "TV15_WorkAgreementPublishPayload" => {
                let payload = decode_hex_required(entry.work_agreement_publish_payload_cbor_hex, "work_agreement_publish_payload_cbor_hex")?;
                let expected_hash = decode_hex_required(entry.work_agreement_publish_payload_sha256_hex, "work_agreement_publish_payload_sha256_hex")?;
                let digest = sha256(&payload);
                if digest.as_slice() != expected_hash {
                    return Err(anyhow!("TV15_WorkAgreementPublishPayload hash mismatch"));
                }
                ensure_roundtrip("TV15_WorkAgreementPublishPayload", &payload)?;
                let parsed = decode_canonical(&payload)?;
                parse_work_agreement_publish_payload(&parsed)?;
            }
            "TV16_WorkAgreementUpdatePayload" => {
                let payload = decode_hex_required(entry.work_agreement_update_payload_cbor_hex, "work_agreement_update_payload_cbor_hex")?;
                let expected_hash = decode_hex_required(entry.work_agreement_update_payload_sha256_hex, "work_agreement_update_payload_sha256_hex")?;
                let digest = sha256(&payload);
                if digest.as_slice() != expected_hash {
                    return Err(anyhow!("TV16_WorkAgreementUpdatePayload hash mismatch"));
                }
                ensure_roundtrip("TV16_WorkAgreementUpdatePayload", &payload)?;
                let parsed = decode_canonical(&payload)?;
                parse_work_agreement_update_payload(&parsed)?;
            }
            "TV17_WorkAgreementClosePayload" => {
                let payload = decode_hex_required(entry.work_agreement_close_payload_cbor_hex, "work_agreement_close_payload_cbor_hex")?;
                let expected_hash = decode_hex_required(entry.work_agreement_close_payload_sha256_hex, "work_agreement_close_payload_sha256_hex")?;
                let digest = sha256(&payload);
                if digest.as_slice() != expected_hash {
                    return Err(anyhow!("TV17_WorkAgreementClosePayload hash mismatch"));
                }
                ensure_roundtrip("TV17_WorkAgreementClosePayload", &payload)?;
                let parsed = decode_canonical(&payload)?;
                parse_work_agreement_close_payload(&parsed)?;
            }
            "TV18_AgentMailMessage" => {
                let payload = decode_hex_required(entry.agentmail_payload_cbor_hex, "agentmail_payload_cbor_hex")?;
                let expected_hash = decode_hex_required(entry.agentmail_payload_sha256_hex, "agentmail_payload_sha256_hex")?;
                let expected_sig = decode_hex_required(entry.agentmail_signature_hex, "agentmail_signature_hex")?;
                let digest = sha256(&payload);
                if digest.as_slice() != expected_hash {
                    return Err(anyhow!("TV18_AgentMailMessage hash mismatch"));
                }
                verify_ed25519_hash(&public_key, &digest, &expected_sig)
                    .context("TV18_AgentMailMessage signature verify")?;
                ensure_roundtrip("TV18_AgentMailMessage payload", &payload)?;
                let parsed = decode_canonical(&payload)?;
                parse_agentmail_payload(&parsed)?;

                if let Some(full_hex) = entry.agentmail_full_object_cbor_hex.clone() {
                    let full = hex::decode(&full_hex).context("decode agentmail_full_object_cbor_hex")?;
                    ensure_roundtrip("TV18_AgentMailMessage full object", &full)?;
                    let message = parse_agentmail_message(&decode_canonical(&full)?)?;
                    if message.signature != expected_sig {
                        return Err(anyhow!("TV18_AgentMailMessage signature mismatch"));
                    }
                }
            }
            _ => {
                return Err(anyhow!("unknown vector id: {}", entry.id));
            }
        }
    }

    println!("vector verification complete");
    Ok(())
}

fn decode_hex_required(value: Option<String>, field: &str) -> Result<Vec<u8>> {
    let hex_str = value.ok_or_else(|| anyhow!("missing {field}"))?;
    hex::decode(&hex_str).with_context(|| format!("decode {field}"))
}

fn verify_hash_and_sig(label: &str, public_key: &[u8], cbor: &[u8], expected_hash: &[u8], signature: &[u8]) -> Result<()> {
    let hash = sha256(cbor);
    if hash.as_slice() != expected_hash {
        return Err(anyhow!("{label} hash mismatch"));
    }
    verify_ed25519_hash(public_key, &hash, signature).with_context(|| format!("{label} signature verify"))?;
    Ok(())
}

fn ensure_roundtrip(label: &str, cbor: &[u8]) -> Result<()> {
    let value = decode_canonical(cbor).with_context(|| format!("{label} decode"))?;
    let encoded = encode_canonical(&value).with_context(|| format!("{label} encode"))?;
    if encoded != cbor {
        return Err(anyhow!("{label} canonical roundtrip mismatch"));
    }
    Ok(())
}

#[derive(Debug)]
struct KillSwitchParts {
    action: u8,
    reason: String,
    ts: u64,
    nonce: Vec<u8>,
    signature: Option<Vec<u8>>,
}

fn parse_kill_switch_map(value: &CborValue) -> Result<KillSwitchParts> {
    let entries = match value {
        CborValue::Map(entries) => entries.clone(),
        _ => return Err(anyhow!("kill switch map must be cbor map")),
    };
    let mut action: Option<u8> = None;
    let mut reason: Option<String> = None;
    let mut ts: Option<u64> = None;
    let mut nonce: Option<Vec<u8>> = None;
    let mut signature: Option<Vec<u8>> = None;

    for (k, v) in entries {
        if let CborValue::Unsigned(key) = k {
            match key {
                0 => {
                    if let CborValue::Unsigned(val) = v {
                        if val <= u8::MAX as u64 {
                            action = Some(val as u8);
                        }
                    }
                }
                1 => {
                    if let CborValue::Text(val) = v {
                        reason = Some(val);
                    }
                }
                2 => {
                    if let CborValue::Unsigned(val) = v {
                        ts = Some(val);
                    }
                }
                3 => {
                    if let CborValue::Bytes(val) = v {
                        nonce = Some(val);
                    }
                }
                4 => {
                    if let CborValue::Bytes(val) = v {
                        signature = Some(val);
                    }
                }
                _ => {}
            }
        }
    }

    Ok(KillSwitchParts {
        action: action.ok_or_else(|| anyhow!("kill switch action missing"))?,
        reason: reason.ok_or_else(|| anyhow!("kill switch reason missing"))?,
        ts: ts.ok_or_else(|| anyhow!("kill switch ts missing"))?,
        nonce: nonce.ok_or_else(|| anyhow!("kill switch nonce missing"))?,
        signature,
    })
}

fn ensure_kill_switch_parts(label: &str, parts: &KillSwitchParts, require_signature: bool) -> Result<()> {
    if parts.action > 1 {
        return Err(anyhow!("{label} invalid action"));
    }
    if parts.nonce.len() != 16 {
        return Err(anyhow!("{label} nonce length invalid"));
    }
    if require_signature {
        match &parts.signature {
            Some(sig) if sig.len() == 64 => {}
            Some(_) => return Err(anyhow!("{label} signature length invalid")),
            None => return Err(anyhow!("{label} signature missing")),
        }
    }
    Ok(())
}

fn kill_switch_payload_map(parts: &KillSwitchParts) -> CborValue {
    CborValue::Map(vec![
        (CborValue::Unsigned(0), CborValue::Unsigned(parts.action as u64)),
        (CborValue::Unsigned(1), CborValue::Text(parts.reason.clone())),
        (CborValue::Unsigned(2), CborValue::Unsigned(parts.ts)),
        (CborValue::Unsigned(3), CborValue::Bytes(parts.nonce.clone())),
    ])
}
