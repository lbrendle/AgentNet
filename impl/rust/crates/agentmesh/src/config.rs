use anyhow::{Context, Result};
use anetsdk::CborValue;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub chain_id: String,
    pub agent_did: String,
    pub key_path: PathBuf,
    #[serde(default)]
    pub node_id: Option<String>,
    #[serde(default)]
    pub listen_addrs: Vec<String>,
    #[serde(default)]
    pub bootstrap: Vec<String>,
    #[serde(default)]
    pub protocols: Vec<String>,
    #[serde(default)]
    pub roles: Vec<String>,
    #[serde(default)]
    pub features: FeaturesConfig,
    #[serde(default)]
    pub pubsub: PubSubConfig,
    #[serde(default)]
    pub handshake: HandshakeConfig,
}

#[derive(Debug, Deserialize, Default)]
pub struct FeaturesConfig {
    #[serde(default)]
    pub encodings: Vec<String>,
    pub max_msg_bytes: Option<u64>,
    pub supports_receipt_anchoring: Option<bool>,
    pub time_sync: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PubSubConfig {
    #[serde(default)]
    pub topics: Vec<String>,
    pub require_economic_proof: Option<bool>,
    pub verify_signatures: Option<bool>,
}

impl Default for PubSubConfig {
    fn default() -> Self {
        Self {
            topics: Vec::new(),
            require_economic_proof: Some(false),
            verify_signatures: Some(true),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct HandshakeConfig {
    pub max_clock_skew_sec: Option<i64>,
    pub require_peer_id_match: Option<bool>,
}

impl Default for HandshakeConfig {
    fn default() -> Self {
        Self {
            max_clock_skew_sec: Some(300),
            require_peer_id_match: Some(true),
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let data = fs::read_to_string(path).with_context(|| format!("read config {}", path.display()))?;
        let cfg: Config = toml::from_str(&data).context("parse config toml")?;
        Ok(cfg)
    }

    pub fn protocols_or_default(&self) -> Vec<String> {
        if self.protocols.is_empty() {
            vec![
                "agentnet/handshake/1.0.0".to_string(),
                "agentnet/dht/1.0.0".to_string(),
                "agentnet/pubsub/1.0.0".to_string(),
            ]
        } else {
            self.protocols.clone()
        }
    }

    pub fn roles_or_default(&self) -> Vec<String> {
        if self.roles.is_empty() {
            vec!["mesh".to_string()]
        } else {
            self.roles.clone()
        }
    }
}

impl FeaturesConfig {
    pub fn to_cbor(&self) -> CborValue {
        let mut entries = Vec::new();
        if !self.encodings.is_empty() {
            entries.push((
                CborValue::Unsigned(0),
                CborValue::Array(self.encodings.iter().map(|s| CborValue::Text(s.clone())).collect()),
            ));
        }
        if let Some(max_msg_bytes) = self.max_msg_bytes {
            entries.push((CborValue::Unsigned(1), CborValue::Unsigned(max_msg_bytes)));
        }
        if let Some(supports_receipt_anchoring) = self.supports_receipt_anchoring {
            entries.push((
                CborValue::Unsigned(2),
                CborValue::Bool(supports_receipt_anchoring),
            ));
        }
        if let Some(time_sync) = &self.time_sync {
            entries.push((CborValue::Unsigned(3), CborValue::Text(time_sync.clone())));
        }
        CborValue::Map(entries)
    }
}

impl HandshakeConfig {
    pub fn max_clock_skew_sec(&self) -> i64 {
        self.max_clock_skew_sec.unwrap_or(300)
    }

    pub fn require_peer_id_match(&self) -> bool {
        self.require_peer_id_match.unwrap_or(true)
    }
}

impl PubSubConfig {
    pub fn require_economic_proof(&self) -> bool {
        self.require_economic_proof.unwrap_or(false)
    }

    pub fn verify_signatures(&self) -> bool {
        self.verify_signatures.unwrap_or(true)
    }
}
