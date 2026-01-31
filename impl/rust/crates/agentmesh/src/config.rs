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
    pub state_dir: Option<PathBuf>,
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
    #[serde(default)]
    pub kill_switch: KillSwitchConfig,
    #[serde(default)]
    pub receipts: ReceiptConfig,
    #[serde(default)]
    pub tx: TxConfig,
    #[serde(default)]
    pub rate_limits: RateLimitConfig,
    #[serde(default)]
    pub dht: DhtConfig,
    #[serde(default)]
    pub agentmail: AgentMailConfig,
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
    pub economic_proof_validator_cmd: Option<Vec<String>>,
    pub economic_proof_validator_timeout_ms: Option<u64>,
    pub economic_proof_fail_open: Option<bool>,
}

impl Default for PubSubConfig {
    fn default() -> Self {
        Self {
            topics: Vec::new(),
            require_economic_proof: Some(false),
            verify_signatures: Some(true),
            economic_proof_validator_cmd: None,
            economic_proof_validator_timeout_ms: Some(5000),
            economic_proof_fail_open: Some(false),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct DhtConfig {
    pub enabled: Option<bool>,
    pub publish_interval_sec: Option<u64>,
    pub agent_record: Option<AgentRecordConfig>,
    #[serde(default)]
    pub service_records: Vec<ServiceRecordConfig>,
    pub community_record: Option<CommunityRecordConfig>,
}

impl Default for DhtConfig {
    fn default() -> Self {
        Self {
            enabled: Some(false),
            publish_interval_sec: None,
            agent_record: None,
            service_records: Vec::new(),
            community_record: None,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct AgentRecordConfig {
    pub record_key: String,
    #[serde(default)]
    pub agent_pubkeys_hex: Vec<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    pub expires_sec: u64,
    pub signing_key_path: Option<PathBuf>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServiceRecordConfig {
    pub record_key: String,
    pub service_type: u16,
    #[serde(default)]
    pub addrs: Vec<String>,
    pub required_credentials: Option<Vec<String>>,
    pub pricing_cbor_path: Option<PathBuf>,
    pub expires_sec: u64,
    pub signing_key_path: Option<PathBuf>,
    pub provider_id: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CommunityRecordConfig {
    pub record_key: String,
    pub community_id: String,
    pub controller: Option<String>,
    pub join_policy: u8,
    pub required_credentials: Option<Vec<String>>,
    pub economics_cbor_path: PathBuf,
    pub governance_cbor_path: PathBuf,
    pub expires_sec: u64,
    pub signing_key_path: Option<PathBuf>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct KillSwitchConfig {
    pub enabled: Option<bool>,
    pub topic: Option<String>,
    pub payload_type: Option<u16>,
    pub pubkey_hex: Option<String>,
    pub max_clock_skew_sec: Option<i64>,
    pub replay_window: Option<usize>,
    pub allow_release: Option<bool>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AgentMailConfig {
    pub enabled: Option<bool>,
    pub topic: Option<String>,
    pub payload_type: Option<u16>,
    pub require_recipient: Option<bool>,
    pub enforce_sender_match: Option<bool>,
    pub require_postage_for_unknown: Option<bool>,
    pub max_clock_skew_sec: Option<i64>,
    pub max_markdown_bytes: Option<u64>,
    pub max_attachments: Option<usize>,
    pub max_attachment_bytes: Option<u64>,
    pub max_total_attachment_bytes: Option<u64>,
    #[serde(default)]
    pub allow_senders: Vec<String>,
    #[serde(default)]
    pub deny_senders: Vec<String>,
    #[serde(default)]
    pub sender_pubkeys: Vec<SenderKeyConfig>,
    pub inbox_path: Option<PathBuf>,
    pub seen_path: Option<PathBuf>,
    pub retention_sec: Option<u64>,
    pub max_seen_entries: Option<usize>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ReceiptConfig {
    pub enabled: Option<bool>,
    pub path: Option<PathBuf>,
    pub emit_policy_accepts: Option<bool>,
    pub emit_policy_denies: Option<bool>,
    pub emit_kill_switch: Option<bool>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TxConfig {
    pub enabled: Option<bool>,
    pub pubsub_payload_type: Option<u16>,
    #[serde(default)]
    pub sender_pubkeys: Vec<SenderKeyConfig>,
    #[serde(default)]
    pub escrow: EscrowConfig,
    #[serde(default)]
    pub identity: IdentityConfig,
    #[serde(default)]
    pub budget: BudgetConfig,
    #[serde(default)]
    pub skill_registry: SkillRegistryConfig,
    #[serde(default)]
    pub work_registry: WorkRegistryConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SenderKeyConfig {
    pub did: String,
    pub pubkey_hex: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct EscrowConfig {
    pub enabled: Option<bool>,
    pub state_path: Option<PathBuf>,
    pub log_path: Option<PathBuf>,
    #[serde(default)]
    pub arbitrators: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct IdentityConfig {
    pub enabled: Option<bool>,
    pub state_path: Option<PathBuf>,
    pub allow_register: Option<bool>,
    pub allow_rotate: Option<bool>,
    pub allow_revoke: Option<bool>,
    pub max_clock_skew_sec: Option<i64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BudgetConfig {
    pub enabled: Option<bool>,
    pub window_sec: Option<u64>,
    #[serde(default)]
    pub caps: Vec<BudgetCurrencyCap>,
    pub state_path: Option<PathBuf>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SkillRegistryConfig {
    pub enabled: Option<bool>,
    pub state_path: Option<PathBuf>,
    pub allow_publish: Option<bool>,
    pub allow_update: Option<bool>,
    pub allow_revoke: Option<bool>,
    pub max_clock_skew_sec: Option<i64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WorkRegistryConfig {
    pub enabled: Option<bool>,
    pub state_path: Option<PathBuf>,
    pub allow_offer_publish: Option<bool>,
    pub allow_agreement_publish: Option<bool>,
    pub allow_agreement_update: Option<bool>,
    pub allow_agreement_close: Option<bool>,
    pub max_clock_skew_sec: Option<i64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BudgetCurrencyCap {
    pub currency: String,
    pub max_amount: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RateLimitConfig {
    pub enabled: Option<bool>,
    pub window_sec: Option<u64>,
    pub max_messages: Option<u64>,
    pub max_bytes: Option<u64>,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: Some(false),
            window_sec: None,
            max_messages: None,
            max_bytes: None,
        }
    }
}

impl Default for TxConfig {
    fn default() -> Self {
        Self {
            enabled: Some(false),
            pubsub_payload_type: None,
            sender_pubkeys: Vec::new(),
            escrow: EscrowConfig::default(),
            identity: IdentityConfig::default(),
            budget: BudgetConfig::default(),
            skill_registry: SkillRegistryConfig::default(),
            work_registry: WorkRegistryConfig::default(),
        }
    }
}

impl Default for EscrowConfig {
    fn default() -> Self {
        Self {
            enabled: Some(false),
            state_path: None,
            log_path: None,
            arbitrators: Vec::new(),
        }
    }
}

impl Default for IdentityConfig {
    fn default() -> Self {
        Self {
            enabled: Some(false),
            state_path: None,
            allow_register: Some(false),
            allow_rotate: Some(false),
            allow_revoke: Some(false),
            max_clock_skew_sec: Some(300),
        }
    }
}

impl Default for BudgetConfig {
    fn default() -> Self {
        Self {
            enabled: Some(false),
            window_sec: None,
            caps: Vec::new(),
            state_path: None,
        }
    }
}

impl Default for SkillRegistryConfig {
    fn default() -> Self {
        Self {
            enabled: Some(false),
            state_path: None,
            allow_publish: Some(false),
            allow_update: Some(false),
            allow_revoke: Some(false),
            max_clock_skew_sec: Some(300),
        }
    }
}

impl Default for WorkRegistryConfig {
    fn default() -> Self {
        Self {
            enabled: Some(false),
            state_path: None,
            allow_offer_publish: Some(false),
            allow_agreement_publish: Some(false),
            allow_agreement_update: Some(false),
            allow_agreement_close: Some(false),
            max_clock_skew_sec: Some(300),
        }
    }
}

impl Default for ReceiptConfig {
    fn default() -> Self {
        Self {
            enabled: Some(false),
            path: None,
            emit_policy_accepts: Some(false),
            emit_policy_denies: Some(true),
            emit_kill_switch: Some(true),
        }
    }
}

impl Default for KillSwitchConfig {
    fn default() -> Self {
        Self {
            enabled: Some(false),
            topic: Some("agentnet/kill/1.0.0".to_string()),
            payload_type: Some(65535),
            pubkey_hex: None,
            max_clock_skew_sec: Some(300),
            replay_window: Some(1024),
            allow_release: Some(false),
        }
    }
}

impl Default for AgentMailConfig {
    fn default() -> Self {
        Self {
            enabled: Some(false),
            topic: Some("agentnet/mail/1.0.0".to_string()),
            payload_type: Some(1000),
            require_recipient: Some(true),
            enforce_sender_match: Some(true),
            require_postage_for_unknown: Some(true),
            max_clock_skew_sec: Some(300),
            max_markdown_bytes: Some(65_536),
            max_attachments: Some(16),
            max_attachment_bytes: Some(26_214_400),
            max_total_attachment_bytes: Some(104_857_600),
            allow_senders: Vec::new(),
            deny_senders: Vec::new(),
            sender_pubkeys: Vec::new(),
            inbox_path: None,
            seen_path: None,
            retention_sec: Some(604_800),
            max_seen_entries: Some(100_000),
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

    pub fn state_dir_or_default(&self) -> PathBuf {
        self.state_dir
            .clone()
            .unwrap_or_else(|| PathBuf::from(".agentmesh"))
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

    pub fn economic_proof_validator_cmd(&self) -> Option<&[String]> {
        self.economic_proof_validator_cmd
            .as_deref()
            .filter(|cmd| !cmd.is_empty())
    }

    pub fn economic_proof_validator_timeout_ms(&self) -> u64 {
        self.economic_proof_validator_timeout_ms.unwrap_or(5000)
    }

    pub fn economic_proof_fail_open(&self) -> bool {
        self.economic_proof_fail_open.unwrap_or(false)
    }
}

impl KillSwitchConfig {
    pub fn enabled(&self) -> bool {
        self.enabled.unwrap_or(false)
    }

    pub fn topic(&self) -> String {
        self.topic
            .clone()
            .unwrap_or_else(|| "agentnet/kill/1.0.0".to_string())
    }

    pub fn payload_type(&self) -> u16 {
        self.payload_type.unwrap_or(65535)
    }

    pub fn max_clock_skew_sec(&self) -> i64 {
        self.max_clock_skew_sec.unwrap_or(300)
    }

    pub fn replay_window(&self) -> usize {
        self.replay_window.unwrap_or(1024)
    }

    pub fn allow_release(&self) -> bool {
        self.allow_release.unwrap_or(false)
    }
}

impl AgentMailConfig {
    pub fn enabled(&self) -> bool {
        self.enabled.unwrap_or(false)
    }

    pub fn topic(&self) -> String {
        self.topic
            .clone()
            .unwrap_or_else(|| "agentnet/mail/1.0.0".to_string())
    }

    pub fn payload_type(&self) -> u16 {
        self.payload_type.unwrap_or(1000)
    }

    pub fn require_recipient(&self) -> bool {
        self.require_recipient.unwrap_or(true)
    }

    pub fn enforce_sender_match(&self) -> bool {
        self.enforce_sender_match.unwrap_or(true)
    }

    pub fn require_postage_for_unknown(&self) -> bool {
        self.require_postage_for_unknown.unwrap_or(true)
    }

    pub fn max_clock_skew_sec(&self) -> i64 {
        self.max_clock_skew_sec.unwrap_or(300)
    }

    pub fn max_markdown_bytes(&self) -> u64 {
        self.max_markdown_bytes.unwrap_or(65_536)
    }

    pub fn max_attachments(&self) -> usize {
        self.max_attachments.unwrap_or(16)
    }

    pub fn max_attachment_bytes(&self) -> u64 {
        self.max_attachment_bytes.unwrap_or(26_214_400)
    }

    pub fn max_total_attachment_bytes(&self) -> u64 {
        self.max_total_attachment_bytes.unwrap_or(104_857_600)
    }

    pub fn retention_sec(&self) -> u64 {
        self.retention_sec.unwrap_or(604_800)
    }

    pub fn max_seen_entries(&self) -> usize {
        self.max_seen_entries.unwrap_or(100_000)
    }

    pub fn inbox_path_or_default(&self, state_dir: &Path) -> PathBuf {
        self.inbox_path
            .clone()
            .unwrap_or_else(|| state_dir.join("agentmail").join("inbox.log"))
    }

    pub fn seen_path_or_default(&self, state_dir: &Path) -> PathBuf {
        self.seen_path
            .clone()
            .unwrap_or_else(|| state_dir.join("agentmail").join("seen.log"))
    }

    pub fn allow_senders(&self) -> &[String] {
        &self.allow_senders
    }

    pub fn deny_senders(&self) -> &[String] {
        &self.deny_senders
    }

    pub fn sender_pubkeys(&self) -> &[SenderKeyConfig] {
        &self.sender_pubkeys
    }
}

impl ReceiptConfig {
    pub fn enabled(&self) -> bool {
        self.enabled.unwrap_or(false)
    }

    pub fn emit_policy_accepts(&self) -> bool {
        self.emit_policy_accepts.unwrap_or(false)
    }

    pub fn emit_policy_denies(&self) -> bool {
        self.emit_policy_denies.unwrap_or(true)
    }

    pub fn emit_kill_switch(&self) -> bool {
        self.emit_kill_switch.unwrap_or(true)
    }

    pub fn path_or_default(&self, state_dir: &Path) -> PathBuf {
        self.path
            .clone()
            .unwrap_or_else(|| state_dir.join("receipts.log"))
    }
}

impl TxConfig {
    pub fn enabled(&self) -> bool {
        self.enabled.unwrap_or(false)
    }

    pub fn pubsub_payload_type(&self) -> Option<u16> {
        self.pubsub_payload_type
    }

    pub fn sender_pubkeys(&self) -> &Vec<SenderKeyConfig> {
        &self.sender_pubkeys
    }
}

impl IdentityConfig {
    pub fn enabled(&self) -> bool {
        self.enabled.unwrap_or(false)
    }

    pub fn allow_register(&self) -> bool {
        self.allow_register.unwrap_or(false)
    }

    pub fn allow_rotate(&self) -> bool {
        self.allow_rotate.unwrap_or(false)
    }

    pub fn allow_revoke(&self) -> bool {
        self.allow_revoke.unwrap_or(false)
    }

    pub fn max_clock_skew_sec(&self) -> i64 {
        self.max_clock_skew_sec.unwrap_or(300)
    }

    pub fn state_path_or_default(&self, state_dir: &Path) -> PathBuf {
        self.state_path
            .clone()
            .unwrap_or_else(|| state_dir.join("identity_registry.json"))
    }
}

impl BudgetConfig {
    pub fn enabled(&self) -> bool {
        self.enabled.unwrap_or(false)
    }

    pub fn window_sec(&self) -> Option<u64> {
        self.window_sec
    }

    pub fn caps(&self) -> &Vec<BudgetCurrencyCap> {
        &self.caps
    }

    pub fn state_path_or_default(&self, state_dir: &Path) -> PathBuf {
        self.state_path
            .clone()
            .unwrap_or_else(|| state_dir.join("budget_state.json"))
    }
}

impl SkillRegistryConfig {
    pub fn enabled(&self) -> bool {
        self.enabled.unwrap_or(false)
    }

    pub fn allow_publish(&self) -> bool {
        self.allow_publish.unwrap_or(false)
    }

    pub fn allow_update(&self) -> bool {
        self.allow_update.unwrap_or(false)
    }

    pub fn allow_revoke(&self) -> bool {
        self.allow_revoke.unwrap_or(false)
    }

    pub fn max_clock_skew_sec(&self) -> i64 {
        self.max_clock_skew_sec.unwrap_or(300)
    }

    pub fn state_path_or_default(&self, state_dir: &Path) -> PathBuf {
        self.state_path
            .clone()
            .unwrap_or_else(|| state_dir.join("skill_registry.json"))
    }
}

impl WorkRegistryConfig {
    pub fn enabled(&self) -> bool {
        self.enabled.unwrap_or(false)
    }

    pub fn allow_offer_publish(&self) -> bool {
        self.allow_offer_publish.unwrap_or(false)
    }

    pub fn allow_agreement_publish(&self) -> bool {
        self.allow_agreement_publish.unwrap_or(false)
    }

    pub fn allow_agreement_update(&self) -> bool {
        self.allow_agreement_update.unwrap_or(false)
    }

    pub fn allow_agreement_close(&self) -> bool {
        self.allow_agreement_close.unwrap_or(false)
    }

    pub fn max_clock_skew_sec(&self) -> i64 {
        self.max_clock_skew_sec.unwrap_or(300)
    }

    pub fn state_path_or_default(&self, state_dir: &Path) -> PathBuf {
        self.state_path
            .clone()
            .unwrap_or_else(|| state_dir.join("work_registry.json"))
    }
}

impl EscrowConfig {
    pub fn enabled(&self) -> bool {
        self.enabled.unwrap_or(false)
    }

    pub fn state_path_or_default(&self, state_dir: &Path) -> PathBuf {
        self.state_path
            .clone()
            .unwrap_or_else(|| state_dir.join("escrow_state.json"))
    }

    pub fn log_path_or_default(&self, state_dir: &Path) -> PathBuf {
        self.log_path
            .clone()
            .unwrap_or_else(|| state_dir.join("escrow_events.log"))
    }
}

impl RateLimitConfig {
    pub fn enabled(&self) -> bool {
        self.enabled.unwrap_or(false)
    }

    pub fn window_sec(&self) -> Option<u64> {
        self.window_sec
    }

    pub fn max_messages(&self) -> Option<u64> {
        self.max_messages
    }

    pub fn max_bytes(&self) -> Option<u64> {
        self.max_bytes
    }
}

impl DhtConfig {
    pub fn enabled(&self) -> bool {
        self.enabled.unwrap_or(false)
    }

    pub fn publish_interval_sec(&self, fallback: u64) -> u64 {
        self.publish_interval_sec.unwrap_or(fallback)
    }
}
