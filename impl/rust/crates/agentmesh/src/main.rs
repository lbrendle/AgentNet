mod config;
mod keys;
mod mesh;
mod state;
mod tx;

use crate::config::Config;
use crate::keys::{generate_keypair, load_keypair, write_secret_key};
use crate::mesh::build_mesh;
use anyhow::{Context, Result};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use clap::{Parser, Subcommand};
use libp2p::identity;
use rand::RngCore;
use std::path::PathBuf;
use tracing::info;

#[derive(Parser)]
#[command(name = "agentmesh")]
#[command(about = "AgentNet Mesh Node", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Run {
        #[arg(long)]
        config: PathBuf,
    },
    Keygen {
        #[arg(long)]
        out: PathBuf,
    },
    PeerId {
        #[arg(long)]
        key: PathBuf,
    },
    Pubkey {
        #[arg(long)]
        key: PathBuf,
    },
    Publish {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        topic: String,
        #[arg(long)]
        payload_type: u16,
        #[arg(long)]
        payload_cbor: PathBuf,
        #[arg(long)]
        proof_tx_hex: Option<String>,
        #[arg(long)]
        proof_voucher_hex: Option<String>,
        #[arg(long, default_value = "5")]
        settle_seconds: u64,
    },
    DhtPut {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        record_key: String,
        #[arg(long)]
        record_type: String,
        #[arg(long)]
        record_cbor: PathBuf,
        #[arg(long)]
        pubkey_hex: Option<String>,
        #[arg(long, default_value = "5")]
        settle_seconds: u64,
    },
    Kill {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        kill_key: PathBuf,
        #[arg(long)]
        action: String,
        #[arg(long)]
        reason: String,
        #[arg(long, default_value = "5")]
        settle_seconds: u64,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    match cli.command {
        Commands::Run { config } => {
            let cfg = Config::load(&config)?;
            let keys = load_keypair(&cfg.key_path)?;
            let mesh = build_mesh(cfg, keys)?;
            mesh.run().await?;
        }
        Commands::Keygen { out } => {
            let keys = generate_keypair();
            write_secret_key(&out, &keys.signing_key)?;
            info!("key generated at {}", out.display());
        }
        Commands::PeerId { key } => {
            let keys = load_keypair(&key)?;
            let secret = keys.signing_key.to_bytes();
            let mut secret_bytes = [0u8; 32];
            secret_bytes.copy_from_slice(&secret);
            let keypair = identity::Keypair::ed25519_from_bytes(&mut secret_bytes)
                .context("build libp2p keypair")?;
            println!("{}", keypair.public().to_peer_id());
        }
        Commands::Pubkey { key } => {
            let keys = load_keypair(&key)?;
            let pubkey = keys.verifying_key.to_bytes();
            println!("{}", hex::encode(pubkey));
        }
        Commands::Publish {
            config,
            topic,
            payload_type,
            payload_cbor,
            proof_tx_hex,
            proof_voucher_hex,
            settle_seconds,
        } => {
            let cfg = Config::load(&config)?;
            let keys = load_keypair(&cfg.key_path)?;
            let mut mesh = build_mesh(cfg, keys)?;
            let payload_bytes = std::fs::read(&payload_cbor)
                .with_context(|| format!("read payload {}", payload_cbor.display()))?;
            let payload = anetsdk::decode_canonical(&payload_bytes)?;
            let proof = if let Some(tx_hex) = proof_tx_hex {
                let tx = hex::decode(tx_hex).context("decode proof_tx_hex")?;
                Some(anetsdk::EconomicProof::OnChainTx { tx_hash: tx })
            } else if let Some(voucher_hex) = proof_voucher_hex {
                let voucher = hex::decode(voucher_hex).context("decode proof_voucher_hex")?;
                Some(anetsdk::EconomicProof::Voucher { voucher })
            } else {
                None
            };
            mesh.publish_envelope(&topic, payload_type, payload, proof)?;
            mesh.run_for(std::time::Duration::from_secs(settle_seconds))
                .await?;
        }
        Commands::DhtPut {
            config,
            record_key,
            record_type,
            record_cbor,
            pubkey_hex,
            settle_seconds,
        } => {
            let cfg = Config::load(&config)?;
            let keys = load_keypair(&cfg.key_path)?;
            let mut mesh = build_mesh(cfg, keys)?;
            let record_bytes = std::fs::read(&record_cbor)
                .with_context(|| format!("read record {}", record_cbor.display()))?;
            let expires = validate_record(&record_type, &record_bytes, pubkey_hex.as_deref())?;
            mesh.put_record(record_key, record_bytes, expires)?;
            mesh.run_for(std::time::Duration::from_secs(settle_seconds))
                .await?;
        }
        Commands::Kill {
            config,
            kill_key,
            action,
            reason,
            settle_seconds,
        } => {
            let cfg = Config::load(&config)?;
            if action == "release" && !cfg.kill_switch.allow_release() {
                anyhow::bail!("kill switch release disabled by config");
            }
            let kill_type = cfg.kill_switch.payload_type();
            let kill_topic = cfg.kill_switch.topic();
            let keys = load_keypair(&cfg.key_path)?;
            let mut mesh = build_mesh(cfg, keys)?;
            let kill_payload = build_kill_switch_payload(&kill_key, &action, &reason)?;
            mesh.publish_envelope(&kill_topic, kill_type, kill_payload, None)?;
            mesh.run_for(std::time::Duration::from_secs(settle_seconds))
                .await?;
        }
    }

    Ok(())
}

fn build_kill_switch_payload(
    kill_key: &PathBuf,
    action: &str,
    reason: &str,
) -> Result<anetsdk::CborValue> {
    let action_code = match action {
        "engage" => 0u8,
        "release" => 1u8,
        _ => anyhow::bail!("kill action must be engage or release"),
    };
    let secret_b64 = std::fs::read_to_string(kill_key)
        .with_context(|| format!("read kill key {}", kill_key.display()))?;
    let secret = BASE64
        .decode(secret_b64.trim())
        .context("decode kill key")?;
    if secret.len() != 32 {
        anyhow::bail!("kill key must be 32 bytes");
    }
    let mut nonce = [0u8; 16];
    let mut rng = rand::rngs::OsRng;
    rng.fill_bytes(&mut nonce);
    let ts = {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_else(|_| std::time::Duration::from_secs(0))
            .as_secs()
    };
    let payload = anetsdk::CborValue::Map(vec![
        (
            anetsdk::CborValue::Unsigned(0),
            anetsdk::CborValue::Unsigned(action_code as u64),
        ),
        (
            anetsdk::CborValue::Unsigned(1),
            anetsdk::CborValue::Text(reason.to_string()),
        ),
        (
            anetsdk::CborValue::Unsigned(2),
            anetsdk::CborValue::Unsigned(ts),
        ),
        (
            anetsdk::CborValue::Unsigned(3),
            anetsdk::CborValue::Bytes(nonce.to_vec()),
        ),
    ]);
    let payload_cbor = anetsdk::encode_canonical(&payload)?;
    let hash = anetsdk::sha256(&payload_cbor);
    let signature = anetsdk::sign_ed25519_hash(&secret, &hash)?;
    let signed_payload = anetsdk::CborValue::Map(vec![
        (
            anetsdk::CborValue::Unsigned(0),
            anetsdk::CborValue::Unsigned(action_code as u64),
        ),
        (
            anetsdk::CborValue::Unsigned(1),
            anetsdk::CborValue::Text(reason.to_string()),
        ),
        (
            anetsdk::CborValue::Unsigned(2),
            anetsdk::CborValue::Unsigned(ts),
        ),
        (
            anetsdk::CborValue::Unsigned(3),
            anetsdk::CborValue::Bytes(nonce.to_vec()),
        ),
        (
            anetsdk::CborValue::Unsigned(4),
            anetsdk::CborValue::Bytes(signature),
        ),
    ]);
    Ok(signed_payload)
}

fn validate_record(
    record_type: &str,
    record_bytes: &[u8],
    pubkey_hex: Option<&str>,
) -> Result<Option<std::time::Instant>> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|_| std::time::Duration::from_secs(0))
        .as_secs();
    match record_type {
        "agent" => {
            let record = anetsdk::parse_agent_record(&anetsdk::decode_canonical(record_bytes)?)?;
            let mut verified = false;
            for key in &record.payload.agent_pubkeys {
                if anetsdk::verify_agent_record(record_bytes, key).is_ok() {
                    verified = true;
                    break;
                }
            }
            if !verified {
                anyhow::bail!("agent record signature invalid");
            }
            if record.payload.expires <= now {
                anyhow::bail!("agent record expired");
            }
            let expires = to_instant(record.payload.expires)?;
            Ok(expires)
        }
        "service" => {
            let pubkey = pubkey_from_hex(pubkey_hex)?;
            let record = anetsdk::verify_service_record(record_bytes, &pubkey)?;
            if record.expires <= now {
                anyhow::bail!("service record expired");
            }
            let expires = to_instant(record.expires)?;
            Ok(expires)
        }
        "community" => {
            let pubkey = pubkey_from_hex(pubkey_hex)?;
            let record = anetsdk::verify_community_record(record_bytes, &pubkey)?;
            if record.expires <= now {
                anyhow::bail!("community record expired");
            }
            let expires = to_instant(record.expires)?;
            Ok(expires)
        }
        _ => anyhow::bail!("record_type must be agent, service, or community"),
    }
}

fn pubkey_from_hex(pubkey_hex: Option<&str>) -> Result<Vec<u8>> {
    let pubkey_hex =
        pubkey_hex.ok_or_else(|| anyhow::anyhow!("pubkey_hex required for this record type"))?;
    let bytes = hex::decode(pubkey_hex).context("decode pubkey_hex")?;
    if bytes.len() != 32 {
        anyhow::bail!("pubkey_hex must be 32 bytes");
    }
    Ok(bytes)
}

fn to_instant(expires_ts: u64) -> Result<Option<std::time::Instant>> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|_| std::time::Duration::from_secs(0))
        .as_secs();
    if expires_ts <= now {
        return Ok(None);
    }
    let delta = expires_ts - now;
    Ok(Some(
        std::time::Instant::now() + std::time::Duration::from_secs(delta),
    ))
}
