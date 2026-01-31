use anyhow::{Context, Result};
use anetsdk::{decode_canonical, encode_canonical, sha256, verify_ed25519_hash, CborValue};
use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Parser)]
#[command(name = "anet-econ-verify")]
#[command(about = "AgentNet economic proof verifier", long_about = None)]
struct Cli {
    #[arg(long)]
    config: PathBuf,
}

#[derive(Debug, Deserialize)]
struct Config {
    voucher: Option<VoucherConfig>,
    onchain: Option<OnchainConfig>,
}

#[derive(Debug, Deserialize)]
struct VoucherConfig {
    issuers: Vec<IssuerConfig>,
    nonce_state_path: PathBuf,
    max_clock_skew_sec: Option<i64>,
    require_topic_match: Option<bool>,
    allowed_purposes: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct IssuerConfig {
    did: String,
    pubkey_hex: String,
}

#[derive(Debug, Deserialize)]
struct OnchainConfig {
    enabled: Option<bool>,
    chain_id: Option<String>,
    rpc_url: Option<String>,
    min_confirmations: Option<u64>,
    require_success: Option<bool>,
    max_tx_age_sec: Option<u64>,
    required_to: Option<String>,
    required_from: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct EconomicProofValidationRequest {
    proof_cbor_hex: String,
    proof_type: u64,
    topic: String,
    sender: String,
    payload_type: u16,
    seq: u64,
    ts: u64,
    message_id: String,
    peer_id: String,
}

#[derive(Debug)]
struct VoucherPayload {
    issuer: String,
    payer: String,
    amount: u64,
    currency: String,
    purpose: String,
    ts: u64,
    exp: u64,
    nonce: Vec<u8>,
    signature: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
struct NonceState {
    used: HashSet<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = load_config(&cli.config)?;
    let request = read_request()?;

    match request.proof_type {
        2 => verify_voucher(&config, &request)?,
        1 => verify_onchain(&config, &request)?,
        _ => anyhow::bail!("unsupported proof type"),
    }

    Ok(())
}

fn load_config(path: &Path) -> Result<Config> {
    let data = fs::read_to_string(path).with_context(|| format!("read config {}", path.display()))?;
    let cfg: Config = toml::from_str(&data).context("parse config")?;
    Ok(cfg)
}

fn read_request() -> Result<EconomicProofValidationRequest> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).context("read stdin")?;
    let request: EconomicProofValidationRequest = serde_json::from_str(&input).context("parse request")?;
    Ok(request)
}

fn verify_voucher(config: &Config, request: &EconomicProofValidationRequest) -> Result<()> {
    let voucher_cfg = config
        .voucher
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("voucher verifier disabled"))?;
    if voucher_cfg.issuers.is_empty() {
        anyhow::bail!("voucher issuers not configured");
    }
    let proof_bytes = hex::decode(&request.proof_cbor_hex).context("decode proof cbor")?;
    let proof_value = decode_canonical(&proof_bytes)?;
    let (kind, voucher_bytes) = parse_proof_wrapper(&proof_value)?;
    if kind != 2 {
        anyhow::bail!("voucher proof type mismatch");
    }
    let voucher_value = decode_canonical(&voucher_bytes)?;
    let voucher = parse_voucher(&voucher_value)?;
    let issuer_pk = issuer_pubkey(voucher_cfg, &voucher.issuer)?;
    verify_voucher_signature(&voucher, &issuer_pk)?;
    validate_voucher_policy(voucher_cfg, request, &voucher)?;
    validate_nonce(voucher_cfg, &voucher)?;
    Ok(())
}

fn verify_onchain(config: &Config, request: &EconomicProofValidationRequest) -> Result<()> {
    let onchain_cfg = config
        .onchain
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("onchain verifier disabled"))?;
    if onchain_cfg.enabled.unwrap_or(false) == false {
        anyhow::bail!("onchain verifier disabled");
    }
    let rpc_url = onchain_cfg
        .rpc_url
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("onchain rpc_url required"))?;
    let expected_chain_id = onchain_cfg
        .chain_id
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("onchain chain_id required"))?;

    let proof_bytes = hex::decode(&request.proof_cbor_hex).context("decode proof cbor")?;
    let proof_value = decode_canonical(&proof_bytes)?;
    let (kind, tx_hash_bytes) = parse_proof_wrapper(&proof_value)?;
    if kind != 1 {
        anyhow::bail!("onchain proof type mismatch");
    }
    if tx_hash_bytes.len() != 32 {
        anyhow::bail!("tx hash length invalid");
    }

    let client = reqwest::blocking::Client::new();
    let chain_id_value = rpc_call(&client, rpc_url, "eth_chainId", json!([]))?;
    let chain_id_hex = chain_id_value
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("invalid chainId response"))?;
    let chain_id = parse_chain_id(chain_id_hex)?;
    let expected_chain = parse_chain_id(expected_chain_id)?;
    if chain_id != expected_chain {
        anyhow::bail!("chain id mismatch");
    }

    let tx_hash_hex = format!("0x{}", hex::encode(tx_hash_bytes));
    let tx_value = rpc_call(
        &client,
        rpc_url,
        "eth_getTransactionByHash",
        json!([tx_hash_hex]),
    )?;
    if tx_value.is_null() {
        anyhow::bail!("transaction not found");
    }
    let tx_from = tx_value
        .get("from")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_lowercase();
    let tx_to = tx_value
        .get("to")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_lowercase();
    if let Some(required_from) = &onchain_cfg.required_from {
        if tx_from != required_from.to_lowercase() {
            anyhow::bail!("tx from mismatch");
        }
    }
    if let Some(required_to) = &onchain_cfg.required_to {
        if tx_to != required_to.to_lowercase() {
            anyhow::bail!("tx to mismatch");
        }
    }

    let receipt_value = rpc_call(
        &client,
        rpc_url,
        "eth_getTransactionReceipt",
        json!([tx_hash_hex]),
    )?;
    if receipt_value.is_null() {
        anyhow::bail!("receipt not found");
    }
    let status = receipt_value
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("0x0");
    let require_success = onchain_cfg.require_success.unwrap_or(true);
    if require_success && status != "0x1" {
        anyhow::bail!("transaction status not successful");
    }

    let tx_block_hex = receipt_value
        .get("blockNumber")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("receipt missing blockNumber"))?;
    let tx_block = parse_hex_u64(tx_block_hex)?;
    let latest_block_hex = rpc_call(&client, rpc_url, "eth_blockNumber", json!([]))?;
    let latest_block = parse_hex_u64(
        latest_block_hex
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("invalid blockNumber response"))?,
    )?;
    if latest_block < tx_block {
        anyhow::bail!("chain head behind tx block");
    }
    let confirmations = latest_block.saturating_sub(tx_block).saturating_add(1);
    if let Some(min_conf) = onchain_cfg.min_confirmations {
        if confirmations < min_conf {
            anyhow::bail!("insufficient confirmations");
        }
    }

    if let Some(max_age) = onchain_cfg.max_tx_age_sec {
        let block_value = rpc_call(
            &client,
            rpc_url,
            "eth_getBlockByNumber",
            json!([tx_block_hex, false]),
        )?;
        if block_value.is_null() {
            anyhow::bail!("block not found");
        }
        let ts_hex = block_value
            .get("timestamp")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("block missing timestamp"))?;
        let block_ts = parse_hex_u64(ts_hex)?;
        let now = unix_time();
        if now.saturating_sub(block_ts) > max_age {
            anyhow::bail!("transaction too old");
        }
    }

    Ok(())
}

fn parse_proof_wrapper(value: &CborValue) -> Result<(u8, Vec<u8>)> {
    let entries = expect_map(value)?;
    let kind = expect_u8(get_required(&entries, 0)?)?;
    let data = expect_bytes(get_required(&entries, 1)?)?;
    Ok((kind, data))
}

fn parse_voucher(value: &CborValue) -> Result<VoucherPayload> {
    let entries = expect_map(value)?;
    let issuer = expect_text(get_required(&entries, 0)?)?;
    let payer = expect_text(get_required(&entries, 1)?)?;
    let amount = expect_u64(get_required(&entries, 2)?)?;
    let currency = expect_text(get_required(&entries, 3)?)?;
    let purpose = expect_text(get_required(&entries, 4)?)?;
    let ts = expect_u64(get_required(&entries, 5)?)?;
    let exp = expect_u64(get_required(&entries, 6)?)?;
    let nonce = expect_bytes_len(get_required(&entries, 7)?, 16)?;
    let signature = expect_bytes_len(get_required(&entries, 8)?, 64)?;
    Ok(VoucherPayload {
        issuer,
        payer,
        amount,
        currency,
        purpose,
        ts,
        exp,
        nonce,
        signature,
    })
}

fn verify_voucher_signature(voucher: &VoucherPayload, issuer_pk: &[u8]) -> Result<()> {
    let payload_map = CborValue::Map(vec![
        (CborValue::Unsigned(0), CborValue::Text(voucher.issuer.clone())),
        (CborValue::Unsigned(1), CborValue::Text(voucher.payer.clone())),
        (CborValue::Unsigned(2), CborValue::Unsigned(voucher.amount)),
        (CborValue::Unsigned(3), CborValue::Text(voucher.currency.clone())),
        (CborValue::Unsigned(4), CborValue::Text(voucher.purpose.clone())),
        (CborValue::Unsigned(5), CborValue::Unsigned(voucher.ts)),
        (CborValue::Unsigned(6), CborValue::Unsigned(voucher.exp)),
        (CborValue::Unsigned(7), CborValue::Bytes(voucher.nonce.clone())),
    ]);
    let payload_cbor = encode_canonical(&payload_map)?;
    let hash = sha256(&payload_cbor);
    verify_ed25519_hash(issuer_pk, &hash, &voucher.signature)?;
    Ok(())
}

fn validate_voucher_policy(
    cfg: &VoucherConfig,
    request: &EconomicProofValidationRequest,
    voucher: &VoucherPayload,
) -> Result<()> {
    if voucher.amount == 0 {
        anyhow::bail!("voucher amount must be > 0");
    }
    if voucher.currency.is_empty() {
        anyhow::bail!("voucher currency required");
    }
    if voucher.payer != request.sender {
        anyhow::bail!("voucher payer mismatch");
    }
    let require_topic_match = cfg.require_topic_match.unwrap_or(true);
    if require_topic_match && voucher.purpose != request.topic {
        anyhow::bail!("voucher purpose mismatch");
    }
    if let Some(allowed) = &cfg.allowed_purposes {
        if !allowed.is_empty() && !allowed.contains(&voucher.purpose) {
            anyhow::bail!("voucher purpose not allowed");
        }
    }
    let now = unix_time();
    let skew = cfg.max_clock_skew_sec.unwrap_or(300).abs() as u64;
    if voucher.ts > now.saturating_add(skew) {
        anyhow::bail!("voucher timestamp outside window");
    }
    if voucher.exp < now.saturating_sub(skew) {
        anyhow::bail!("voucher expired");
    }
    if voucher.exp < voucher.ts {
        anyhow::bail!("voucher expiry before timestamp");
    }
    Ok(())
}

fn validate_nonce(cfg: &VoucherConfig, voucher: &VoucherPayload) -> Result<()> {
    let mut state = load_nonce_state(&cfg.nonce_state_path)?;
    let nonce_key = format!(
        "{}:{}:{}:{}:{}:{}",
        voucher.issuer,
        hex::encode(&voucher.nonce),
        voucher.payer,
        voucher.amount,
        voucher.currency,
        voucher.purpose
    );
    if state.used.contains(&nonce_key) {
        anyhow::bail!("voucher nonce replay detected");
    }
    state.used.insert(nonce_key);
    persist_nonce_state(&cfg.nonce_state_path, &state)?;
    Ok(())
}

fn issuer_pubkey(cfg: &VoucherConfig, issuer: &str) -> Result<Vec<u8>> {
    let mut map = HashMap::new();
    for entry in &cfg.issuers {
        let pk = hex::decode(&entry.pubkey_hex)
            .with_context(|| format!("decode issuer pubkey {}", entry.did))?;
        if pk.len() != 32 {
            anyhow::bail!("issuer pubkey length invalid for {}", entry.did);
        }
        map.insert(entry.did.clone(), pk);
    }
    map.get(issuer)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("unknown voucher issuer"))
}

fn rpc_call(
    client: &reqwest::blocking::Client,
    rpc_url: &str,
    method: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value> {
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params,
    });
    let response = client
        .post(rpc_url)
        .json(&request)
        .send()
        .context("rpc request failed")?;
    if !response.status().is_success() {
        anyhow::bail!("rpc request failed with status {}", response.status());
    }
    let value: serde_json::Value = response.json().context("parse rpc response")?;
    if let Some(error) = value.get("error") {
        anyhow::bail!("rpc error: {}", error);
    }
    value
        .get("result")
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("rpc missing result"))
}

fn parse_chain_id(value: &str) -> Result<u64> {
    if let Some(stripped) = value.strip_prefix("0x") {
        return u64::from_str_radix(stripped, 16).context("parse chain id hex");
    }
    value.parse::<u64>().context("parse chain id")
}

fn parse_hex_u64(value: &str) -> Result<u64> {
    let stripped = value.strip_prefix("0x").unwrap_or(value);
    u64::from_str_radix(stripped, 16).context("parse hex u64")
}

fn load_nonce_state(path: &Path) -> Result<NonceState> {
    if !path.exists() {
        return Ok(NonceState { used: HashSet::new() });
    }
    let data = fs::read(path).with_context(|| format!("read nonce state {}", path.display()))?;
    let state: NonceState = serde_json::from_slice(&data).context("parse nonce state")?;
    Ok(state)
}

fn persist_nonce_state(path: &Path, state: &NonceState) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create nonce state dir {}", parent.display()))?;
    }
    let data = serde_json::to_vec_pretty(state).context("encode nonce state")?;
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)
        .with_context(|| format!("write nonce state {}", path.display()))?;
    file.write_all(&data).context("persist nonce state")?;
    Ok(())
}

fn unix_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn expect_map(value: &CborValue) -> Result<Vec<(CborValue, CborValue)>> {
    match value {
        CborValue::Map(entries) => Ok(entries.clone()),
        _ => anyhow::bail!("expected map"),
    }
}

fn get_required(entries: &[(CborValue, CborValue)], key: u64) -> Result<&CborValue> {
    for (k, v) in entries {
        if let CborValue::Unsigned(n) = k {
            if *n == key {
                return Ok(v);
            }
        }
    }
    anyhow::bail!("missing required key")
}

fn expect_u8(value: &CborValue) -> Result<u8> {
    match value {
        CborValue::Unsigned(n) if *n <= u8::MAX as u64 => Ok(*n as u8),
        _ => anyhow::bail!("expected u8"),
    }
}

fn expect_u64(value: &CborValue) -> Result<u64> {
    match value {
        CborValue::Unsigned(n) => Ok(*n),
        _ => anyhow::bail!("expected unsigned"),
    }
}

fn expect_text(value: &CborValue) -> Result<String> {
    match value {
        CborValue::Text(s) => Ok(s.clone()),
        _ => anyhow::bail!("expected text"),
    }
}

fn expect_bytes(value: &CborValue) -> Result<Vec<u8>> {
    match value {
        CborValue::Bytes(b) => Ok(b.clone()),
        _ => anyhow::bail!("expected bytes"),
    }
}

fn expect_bytes_len(value: &CborValue, len: usize) -> Result<Vec<u8>> {
    let bytes = expect_bytes(value)?;
    if bytes.len() != len {
        anyhow::bail!("invalid byte length");
    }
    Ok(bytes)
}
