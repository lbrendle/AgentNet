mod config;
mod keys;
mod mesh;

use crate::config::Config;
use crate::keys::{generate_keypair, load_keypair, write_secret_key};
use crate::mesh::build_mesh;
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use libp2p::identity;
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
    }

    Ok(())
}
