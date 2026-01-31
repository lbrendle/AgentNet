use anyhow::{anyhow, Context, Result};
use anetsdk::{
    decode_canonical, encode_canonical, parse_action_intent, parse_approval_payload,
    parse_grant_payload, parse_nodehello_payload, parse_receipt_payload, parse_tx_envelope_payload,
    payload_from_parsed, sha256, sign_ed25519_hash, verify_ed25519_hash,
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
