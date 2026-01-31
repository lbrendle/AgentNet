use crate::config::{
    AgentMailConfig, AgentRecordConfig, CommunityRecordConfig, Config, DhtConfig, HandshakeConfig,
    KillSwitchConfig, PubSubConfig, RateLimitConfig, SenderKeyConfig, ServiceRecordConfig,
    TxConfig,
};
use crate::keys::KeyMaterial;
use crate::state::SequenceStore;
use crate::tx::{ReceiptSpec, TxEngine};
use ::futures::prelude::*;
use anetsdk::{
    build_agent_record, build_community_record, build_nodehello, build_pubsub_envelope,
    build_service_record, encode_canonical, sha256, sign_ed25519_hash, verify_agentmail_message,
    verify_ed25519_hash, verify_nodehello, verify_pubsub_envelope, AgentRecordPayload, CborValue,
    CommunityRecordPayload, EconomicProof, NodeHelloPayload, PubSubEnvelopePayload, ReceiptLog,
    ServiceRecordPayload,
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use libp2p::core::muxing::StreamMuxerBox;
use libp2p::core::transport::{Boxed, Transport};
use libp2p::core::upgrade;
use libp2p::gossipsub::{self, IdentTopic, MessageAuthenticity, ValidationMode};
use libp2p::identify;
use libp2p::kad::{self, store::MemoryStore};
use libp2p::noise;
use libp2p::request_response::{self};
use libp2p::swarm::{NetworkBehaviour, StreamProtocol, Swarm, SwarmEvent};
use libp2p::tcp;
use libp2p::websocket;
use libp2p::yamux;
use libp2p::{identity, Multiaddr, PeerId};
use rand::rngs::OsRng;
use rand::RngCore;
use serde::Serialize;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Instant as StdInstant;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::time::Duration;
use tracing::{info, warn};

const HANDSHAKE_PROTOCOL: &str = "agentnet/handshake/1.0.0";
const HANDSHAKE_STREAM_PROTOCOL: &str = "/agentnet/handshake/1.0.0";
const MAX_HANDSHAKE_MSG_BYTES: usize = 1024 * 1024;
const EV_POLICY_DECISION: u64 = 6;
const EV_GOVERNANCE_EVENT: u64 = 7;

#[derive(Clone, Default)]
struct NodeHelloCodec();

#[async_trait]
impl request_response::Codec for NodeHelloCodec {
    type Protocol = StreamProtocol;
    type Request = Vec<u8>;
    type Response = Vec<u8>;

    async fn read_request<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
    ) -> std::io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut buf = Vec::new();
        io.take(MAX_HANDSHAKE_MSG_BYTES as u64)
            .read_to_end(&mut buf)
            .await?;
        Ok(buf)
    }

    async fn read_response<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
    ) -> std::io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut buf = Vec::new();
        io.take(MAX_HANDSHAKE_MSG_BYTES as u64)
            .read_to_end(&mut buf)
            .await?;
        Ok(buf)
    }

    async fn write_request<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        data: Self::Request,
    ) -> std::io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        io.write_all(data.as_ref()).await?;
        Ok(())
    }

    async fn write_response<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        data: Self::Response,
    ) -> std::io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        io.write_all(data.as_ref()).await?;
        Ok(())
    }
}

#[derive(NetworkBehaviour)]
struct MeshBehaviour {
    gossipsub: gossipsub::Behaviour,
    kademlia: kad::Behaviour<MemoryStore>,
    request_response: request_response::Behaviour<NodeHelloCodec>,
    identify: identify::Behaviour,
}

pub struct MeshNode {
    swarm: Swarm<MeshBehaviour>,
    nodehello_template: NodeHelloTemplate,
    handshake: HandshakeConfig,
    pubsub: PubSubConfig,
    agentmail_config: AgentMailConfig,
    agent_did: String,
    seq_store: SequenceStore,
    receipt_state: Option<ReceiptState>,
    tx_engine: Option<TxEngine>,
    rate_limiter: Option<RateLimiter>,
    kill_switch: Option<KillSwitchState>,
    kill_switch_engaged: bool,
    agentmail: Option<AgentMailState>,
    listen_addrs: Vec<Multiaddr>,
    dht: Option<DhtState>,
    peer_hellos: HashMap<PeerId, NodeHelloPayload>,
}

struct NodeHelloTemplate {
    chain_id: String,
    node_id: String,
    node_pubkey: Vec<u8>,
    roles: Vec<String>,
    features: CborValue,
    protocols: Vec<String>,
    secret_key: [u8; 32],
}

struct KillSwitchState {
    config: KillSwitchConfig,
    pubkey: Vec<u8>,
    seen_nonces: HashSet<Vec<u8>>,
    nonce_order: VecDeque<Vec<u8>>,
}

struct ReceiptState {
    log: ReceiptLog,
    policy_hash: [u8; 32],
    emit_policy_accepts: bool,
    emit_policy_denies: bool,
    emit_kill_switch: bool,
}

struct KillSwitchPayload {
    action: u8,
    reason: String,
    ts: u64,
    nonce: Vec<u8>,
    signature: Vec<u8>,
}

#[derive(Serialize)]
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

struct DhtState {
    config: DhtConfig,
    interval: Duration,
    next_publish: StdInstant,
}

struct AgentMailState {
    config: AgentMailConfig,
    sender_pubkeys: HashMap<String, Vec<u8>>,
    allow_senders: HashSet<String>,
    deny_senders: HashSet<String>,
    inbox: AgentMailInbox,
    seen: AgentMailSeen,
}

struct AgentMailInbox {
    path: PathBuf,
    file: File,
}

struct AgentMailSeen {
    path: PathBuf,
    retention_sec: u64,
    max_entries: usize,
    entries: HashMap<String, u64>,
    order: VecDeque<String>,
}

struct RateLimiter {
    window_sec: u64,
    max_messages: Option<u64>,
    max_bytes: Option<u64>,
    state: HashMap<String, SenderWindow>,
}

struct SenderWindow {
    window_start: u64,
    count: u64,
    bytes: u64,
}

impl NodeHelloTemplate {
    fn build(&self) -> Result<Vec<u8>> {
        let mut nonce = [0u8; 16];
        OsRng.fill_bytes(&mut nonce);
        let payload = NodeHelloPayload {
            protocols: self.protocols.clone(),
            chain_id: self.chain_id.clone(),
            node_id: self.node_id.clone(),
            node_pubkey: self.node_pubkey.clone(),
            roles: self.roles.clone(),
            features: self.features.clone(),
            time: unix_time(),
            nonce: nonce.to_vec(),
        };
        build_nodehello(&payload, &self.secret_key).context("build nodehello")
    }
}

pub fn build_mesh(config: Config, keys: KeyMaterial) -> Result<MeshNode> {
    let secret_bytes = keys.signing_key.to_bytes();
    let mut secret = [0u8; 32];
    secret.copy_from_slice(&secret_bytes);

    let libp2p_keypair = keypair_from_secret(secret)?;
    let peer_id = PeerId::from(libp2p_keypair.public());

    let mut gossipsub_config = gossipsub::ConfigBuilder::default();
    gossipsub_config.validation_mode(ValidationMode::Strict);
    gossipsub_config.max_transmit_size(1024 * 1024);
    let gossipsub = gossipsub::Behaviour::new(
        MessageAuthenticity::Signed(libp2p_keypair.clone()),
        gossipsub_config.build().context("build gossipsub config")?,
    )
    .map_err(|err| anyhow::anyhow!(err))?;

    let store = MemoryStore::new(peer_id);
    let mut kademlia = kad::Behaviour::with_config(peer_id, store, kad::Config::default());
    kademlia.set_mode(Some(kad::Mode::Server));

    let req_proto = request_response::ProtocolSupport::Full;
    let req_cfg = request_response::Config::default();
    let request_response = request_response::Behaviour::<NodeHelloCodec>::new(
        std::iter::once((StreamProtocol::new(HANDSHAKE_STREAM_PROTOCOL), req_proto)),
        req_cfg,
    );

    let identify = identify::Behaviour::new(identify::Config::new(
        HANDSHAKE_STREAM_PROTOCOL.to_string(),
        libp2p_keypair.public(),
    ));

    let behaviour = MeshBehaviour {
        gossipsub,
        kademlia,
        request_response,
        identify,
    };

    let transport = build_transport(&config, &libp2p_keypair)?;
    let mut swarm = Swarm::new(
        transport,
        behaviour,
        peer_id,
        libp2p::swarm::Config::with_tokio_executor(),
    );

    for addr in &config.listen_addrs {
        let multiaddr: Multiaddr = addr
            .parse()
            .with_context(|| format!("invalid listen addr {addr}"))?;
        swarm
            .listen_on(multiaddr)
            .with_context(|| format!("listen on {addr}"))?;
    }

    for addr in &config.bootstrap {
        let multiaddr: Multiaddr = addr
            .parse()
            .with_context(|| format!("invalid bootstrap addr {addr}"))?;
        swarm
            .dial(multiaddr)
            .with_context(|| format!("dial {addr}"))?;
    }

    let node_id = config
        .node_id
        .clone()
        .unwrap_or_else(|| peer_id.to_string());
    let protocols = config.protocols_or_default();
    let roles = config.roles_or_default();
    let features = config.features.to_cbor();
    let node_pubkey = keys.verifying_key.to_bytes().to_vec();

    let nodehello_template = NodeHelloTemplate {
        chain_id: config.chain_id.clone(),
        node_id,
        node_pubkey,
        roles,
        features,
        protocols,
        secret_key: secret,
    };

    let state_dir = config.state_dir_or_default();
    let seq_store = SequenceStore::load(&state_dir, "pubsub-seq")?;
    let receipt_state = build_receipt_state(&config, &state_dir)?;
    let tx_engine = TxEngine::build(&config.tx, &state_dir)?;
    let rate_limiter = build_rate_limiter(&config.rate_limits)?;
    let kill_switch = build_kill_switch(&config.kill_switch)?;
    let dht = build_dht_state(&config.dht)?;
    let agentmail_state = build_agentmail_state(&config.agentmail, &state_dir)?;

    Ok(MeshNode {
        swarm,
        nodehello_template,
        handshake: config.handshake,
        pubsub: config.pubsub,
        agentmail_config: config.agentmail.clone(),
        agent_did: config.agent_did.clone(),
        seq_store,
        receipt_state,
        tx_engine,
        rate_limiter,
        kill_switch,
        kill_switch_engaged: false,
        agentmail: agentmail_state,
        listen_addrs: Vec::new(),
        dht,
        peer_hellos: HashMap::new(),
    })
}

fn build_transport(
    config: &Config,
    keypair: &identity::Keypair,
) -> Result<Boxed<(PeerId, StreamMuxerBox)>> {
    let transports = config.transports_or_default();
    let mut enabled = Vec::new();
    for name in transports {
        match name.as_str() {
            "quic" => enabled.push(build_quic_transport(keypair)?),
            "ws" | "websocket" => enabled.push(build_ws_transport(keypair)?),
            _ => anyhow::bail!("unsupported transport: {name}"),
        }
    }
    let mut iter = enabled.into_iter();
    let Some(first) = iter.next() else {
        anyhow::bail!("no transports enabled");
    };
    let combined = iter.fold(first, |acc, transport| {
        acc.or_transport(transport)
            .map(|either, _| match either {
                futures::future::Either::Left(value) => value,
                futures::future::Either::Right(value) => value,
            })
            .boxed()
    });
    Ok(combined)
}

fn build_quic_transport(keypair: &identity::Keypair) -> Result<Boxed<(PeerId, StreamMuxerBox)>> {
    let quic_config = libp2p::quic::Config::new(keypair);
    Ok(libp2p::quic::tokio::Transport::new(quic_config)
        .map(|(peer_id, conn), _| (peer_id, StreamMuxerBox::new(conn)))
        .boxed())
}

fn build_ws_transport(keypair: &identity::Keypair) -> Result<Boxed<(PeerId, StreamMuxerBox)>> {
    let tcp_transport = tcp::tokio::Transport::new(tcp::Config::default().nodelay(true));
    let dns_tcp =
        libp2p::dns::tokio::Transport::system(tcp_transport).context("init dns transport")?;
    let ws_transport = websocket::WsConfig::new(dns_tcp);
    let noise_config =
        noise::Config::new(keypair).map_err(|err| anyhow::anyhow!("noise config: {err}"))?;
    let yamux_config = yamux::Config::default();
    Ok(ws_transport
        .upgrade(upgrade::Version::V1)
        .authenticate(noise_config)
        .multiplex(yamux_config)
        .boxed())
}

impl MeshNode {
    pub async fn run(mut self) -> Result<()> {
        let topics = self.pubsub.topics.clone();
        for topic in topics {
            let t = IdentTopic::new(topic);
            self.swarm
                .behaviour_mut()
                .gossipsub
                .subscribe(&t)
                .context("subscribe topic")?;
        }
        if self.agentmail_config.enabled() {
            let topic = IdentTopic::new(self.agentmail_config.topic());
            self.swarm
                .behaviour_mut()
                .gossipsub
                .subscribe(&topic)
                .context("subscribe agentmail topic")?;
        }
        if let Some(kill_switch) = &self.kill_switch {
            let kill_topic = IdentTopic::new(kill_switch.config.topic());
            self.swarm
                .behaviour_mut()
                .gossipsub
                .subscribe(&kill_topic)
                .context("subscribe kill switch topic")?;
        }

        info!(
            "mesh node running (agent_did={}, peer_id={}, listen_addrs={:?})",
            self.agent_did,
            self.swarm.local_peer_id(),
            self.listen_addrs
        );

        loop {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    info!("shutdown requested");
                    break;
                }
                event = self.swarm.select_next_some() => {
                    self.handle_event(event).await?;
                }
                _ = tokio::time::sleep(Duration::from_millis(200)) => {}
            }
            self.maybe_publish_dht()?;
        }

        Ok(())
    }

    pub async fn run_for(&mut self, duration: Duration) -> Result<()> {
        let deadline = tokio::time::Instant::now() + duration;
        loop {
            if tokio::time::Instant::now() >= deadline {
                break;
            }
            tokio::select! {
                event = self.swarm.select_next_some() => {
                    self.handle_event(event).await?;
                }
                _ = tokio::time::sleep(Duration::from_millis(10)) => {}
            }
            self.maybe_publish_dht()?;
        }
        Ok(())
    }

    pub fn publish_envelope(
        &mut self,
        topic: &str,
        payload_type: u16,
        payload: CborValue,
        economic_proof: Option<EconomicProof>,
    ) -> Result<()> {
        if self.kill_switch_engaged && !self.is_kill_switch_payload(payload_type) {
            anyhow::bail!("kill switch engaged: publish blocked");
        }
        let seq = self.seq_store.next()?;
        let envelope = PubSubEnvelopePayload {
            version: 1,
            topic: topic.to_string(),
            sender: self.agent_did.clone(),
            ts: unix_time(),
            seq,
            payload_type,
            payload,
            economic_proof,
        };
        let data = build_pubsub_envelope(&envelope, &self.nodehello_template.secret_key)?;
        let topic = IdentTopic::new(topic);
        self.swarm
            .behaviour_mut()
            .gossipsub
            .publish(topic, data)
            .map_err(|err| anyhow::anyhow!(err))?;
        Ok(())
    }

    pub fn put_record(
        &mut self,
        key: String,
        value: Vec<u8>,
        expires: Option<std::time::Instant>,
    ) -> Result<()> {
        if self.kill_switch_engaged {
            anyhow::bail!("kill switch engaged: dht publish blocked");
        }
        let record = kad::Record {
            key: kad::RecordKey::new(&key),
            value,
            publisher: None,
            expires,
        };
        self.swarm
            .behaviour_mut()
            .kademlia
            .put_record(record, kad::Quorum::One)
            .map_err(|err| anyhow::anyhow!(format!("kademlia put_record failed: {err:?}")))?;
        Ok(())
    }

    async fn handle_event(&mut self, event: SwarmEvent<MeshBehaviourEvent>) -> Result<()> {
        match event {
            SwarmEvent::Behaviour(event) => {
                self.handle_behaviour_event(event).await?;
            }
            SwarmEvent::NewListenAddr { address, .. } => {
                info!("listening on {address}");
                if !self.listen_addrs.iter().any(|addr| addr == &address) {
                    self.listen_addrs.push(address);
                }
            }
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                info!("connection established with {peer_id}");
                self.send_nodehello(peer_id)?;
            }
            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                info!("connection closed with {peer_id}");
                self.peer_hellos.remove(&peer_id);
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_behaviour_event(&mut self, event: MeshBehaviourEvent) -> Result<()> {
        match event {
            MeshBehaviourEvent::Gossipsub(event) => {
                if let gossipsub::Event::Message {
                    propagation_source,
                    message_id,
                    message,
                } = event
                {
                    self.handle_gossipsub_message(propagation_source, message_id, message)
                        .await?;
                }
            }
            MeshBehaviourEvent::RequestResponse(event) => {
                self.handle_request_response(event)?;
            }
            MeshBehaviourEvent::Kademlia(event) => {
                if let kad::Event::OutboundQueryProgressed { result, .. } = event {
                    if let kad::QueryResult::PutRecord(result) = result {
                        if let Err(err) = result {
                            warn!("kademlia put record failed: {err:?}");
                        }
                    }
                }
            }
            MeshBehaviourEvent::Identify(event) => {
                if let identify::Event::Received { peer_id, info, .. } = event {
                    info!("identify from {peer_id}: protocols={:?}", info.protocols);
                }
            }
        }
        Ok(())
    }

    fn send_nodehello(&mut self, peer_id: PeerId) -> Result<()> {
        let hello = self.nodehello_template.build()?;
        self.swarm
            .behaviour_mut()
            .request_response
            .send_request(&peer_id, hello);
        Ok(())
    }

    fn handle_request_response(
        &mut self,
        event: request_response::Event<Vec<u8>, Vec<u8>>,
    ) -> Result<()> {
        match event {
            request_response::Event::Message { peer, message } => match message {
                request_response::Message::Request {
                    request, channel, ..
                } => {
                    if let Err(err) = self.handle_nodehello(peer, &request) {
                        warn!("nodehello request invalid from {peer}: {err:?}");
                    }
                    let response = self.nodehello_template.build()?;
                    self.swarm
                        .behaviour_mut()
                        .request_response
                        .send_response(channel, response)
                        .map_err(|_| anyhow::anyhow!("failed to send nodehello response"))?;
                }
                request_response::Message::Response { response, .. } => {
                    if let Err(err) = self.handle_nodehello(peer, &response) {
                        warn!("nodehello response invalid from {peer}: {err:?}");
                    }
                }
            },
            request_response::Event::OutboundFailure { peer, error, .. } => {
                warn!("nodehello outbound failure {peer}: {error:?}");
            }
            request_response::Event::InboundFailure { peer, error, .. } => {
                warn!("nodehello inbound failure {peer}: {error:?}");
            }
            request_response::Event::ResponseSent { peer, .. } => {
                info!("nodehello response sent to {peer}");
            }
        }
        Ok(())
    }

    fn handle_nodehello(&mut self, peer: PeerId, data: &[u8]) -> Result<()> {
        let payload = verify_nodehello(data)?;
        self.validate_nodehello(peer, &payload)?;
        self.peer_hellos.insert(peer, payload);
        Ok(())
    }

    fn validate_nodehello(&self, peer: PeerId, payload: &NodeHelloPayload) -> Result<()> {
        if payload.chain_id != self.nodehello_template.chain_id {
            anyhow::bail!("chain_id mismatch");
        }
        if !payload.protocols.iter().any(|p| p == HANDSHAKE_PROTOCOL) {
            anyhow::bail!("handshake protocol missing");
        }
        let now = unix_time() as i64;
        let skew = (payload.time as i64 - now).abs();
        if skew > self.handshake.max_clock_skew_sec() {
            anyhow::bail!("clock skew too large");
        }
        if self.handshake.require_peer_id_match() {
            let expected_peer = peer_id_from_ed25519(&payload.node_pubkey)?;
            if expected_peer != peer {
                anyhow::bail!("peer id mismatch");
            }
        }
        Ok(())
    }

    async fn handle_gossipsub_message(
        &mut self,
        propagation_source: PeerId,
        message_id: gossipsub::MessageId,
        message: gossipsub::Message,
    ) -> Result<()> {
        if !self.peer_hellos.contains_key(&propagation_source) {
            warn!("dropping pubsub msg {message_id} from unknown peer {propagation_source}");
            if let Err(err) = self.emit_policy_decision(
                "reject",
                "unknown peer",
                None,
                Some(&propagation_source),
                Some(&message_id),
            ) {
                warn!("receipt emit failed: {err:?}");
            }
            return Ok(());
        }

        if self.pubsub.verify_signatures() {
            let peer = self.peer_hellos.get(&propagation_source).expect("checked");
            match verify_pubsub_envelope(&message.data, &peer.node_pubkey) {
                Ok(payload) => {
                    if self.is_kill_switch_payload(payload.payload_type) {
                        if let Err(err) = self.handle_kill_switch_payload(&payload) {
                            warn!("kill switch payload invalid: {err:?}");
                        }
                        return Ok(());
                    }
                    if self.kill_switch_engaged {
                        warn!("dropping pubsub msg {message_id}: kill switch engaged");
                        if let Err(err) = self.emit_policy_decision(
                            "reject",
                            "kill switch engaged",
                            Some(&payload),
                            Some(&propagation_source),
                            Some(&message_id),
                        ) {
                            warn!("receipt emit failed: {err:?}");
                        }
                        return Ok(());
                    }
                    if let Some(limiter) = &mut self.rate_limiter {
                        if !limiter.allow(&payload.sender, message.data.len(), unix_time()) {
                            warn!("dropping pubsub msg {message_id}: rate limit exceeded");
                            if let Err(err) = self.emit_policy_decision(
                                "reject",
                                "rate limit exceeded",
                                Some(&payload),
                                Some(&propagation_source),
                                Some(&message_id),
                            ) {
                                warn!("receipt emit failed: {err:?}");
                            }
                            return Ok(());
                        }
                    }
                    if self.pubsub.require_economic_proof() && payload.economic_proof.is_none() {
                        warn!("dropping pubsub msg {message_id}: missing economic proof");
                        if let Err(err) = self.emit_policy_decision(
                            "reject",
                            "missing economic proof",
                            Some(&payload),
                            Some(&propagation_source),
                            Some(&message_id),
                        ) {
                            warn!("receipt emit failed: {err:?}");
                        }
                        return Ok(());
                    }
                    if payload.economic_proof.is_some() {
                        match self
                            .validate_economic_proof(&payload, &propagation_source, &message_id)
                            .await
                        {
                            Ok(true) => {}
                            Ok(false) => {
                                warn!("dropping pubsub msg {message_id}: invalid economic proof");
                                if let Err(err) = self.emit_policy_decision(
                                    "reject",
                                    "invalid economic proof",
                                    Some(&payload),
                                    Some(&propagation_source),
                                    Some(&message_id),
                                ) {
                                    warn!("receipt emit failed: {err:?}");
                                }
                                return Ok(());
                            }
                            Err(err) => {
                                if self.pubsub.economic_proof_fail_open() {
                                    warn!("economic proof validation error, fail-open: {err:?}");
                                } else {
                                    warn!("economic proof validation error: {err:?}");
                                    if let Err(err) = self.emit_policy_decision(
                                        "reject",
                                        "economic proof validation error",
                                        Some(&payload),
                                        Some(&propagation_source),
                                        Some(&message_id),
                                    ) {
                                        warn!("receipt emit failed: {err:?}");
                                    }
                                    return Ok(());
                                }
                            }
                        }
                    }
                    if self.is_agentmail_payload(payload.payload_type) {
                        if let Err(err) = self.handle_agentmail_payload(
                            &payload,
                            &propagation_source,
                            &message_id,
                        ) {
                            warn!("agentmail handling failed: {err:?}");
                        }
                        return Ok(());
                    }
                    if let Some(tx_engine) = &mut self.tx_engine {
                        if tx_engine.matches_payload_type(payload.payload_type) {
                            let economics = economics_from_proof(payload.economic_proof.as_ref())?;
                            let decision = tx_engine.handle_pubsub_payload(
                                &payload.payload,
                                economics,
                                unix_time(),
                            )?;
                            if !decision.accept {
                                let reason = decision.reason.as_deref().unwrap_or("tx rejected");
                                if let Err(err) = self.emit_policy_decision(
                                    "reject",
                                    reason,
                                    Some(&payload),
                                    Some(&propagation_source),
                                    Some(&message_id),
                                ) {
                                    warn!("receipt emit failed: {err:?}");
                                }
                                return Ok(());
                            }
                            if let Some(spec) = decision.receipt {
                                if let Err(err) = self.emit_receipt_spec(spec) {
                                    warn!("receipt emit failed: {err:?}");
                                }
                            }
                        }
                    }
                    self.log_pubsub_payload(&payload);
                    if let Err(err) = self.emit_policy_decision(
                        "accept",
                        "policy ok",
                        Some(&payload),
                        Some(&propagation_source),
                        Some(&message_id),
                    ) {
                        warn!("receipt emit failed: {err:?}");
                    }
                }
                Err(err) => {
                    warn!("invalid pubsub envelope from {propagation_source}: {err:?}");
                    if let Err(err) = self.emit_policy_decision(
                        "reject",
                        "invalid pubsub envelope",
                        None,
                        Some(&propagation_source),
                        Some(&message_id),
                    ) {
                        warn!("receipt emit failed: {err:?}");
                    }
                }
            }
        } else {
            info!("pubsub msg {message_id} received from {propagation_source}");
        }
        Ok(())
    }

    fn log_pubsub_payload(&self, payload: &PubSubEnvelopePayload) {
        info!(
            "pubsub payload topic={} sender={} type={} seq={} ts={}",
            payload.topic, payload.sender, payload.payload_type, payload.seq, payload.ts
        );
    }

    fn emit_policy_decision(
        &mut self,
        decision: &str,
        reason: &str,
        payload: Option<&PubSubEnvelopePayload>,
        peer: Option<&PeerId>,
        message_id: Option<&gossipsub::MessageId>,
    ) -> Result<()> {
        let actor = self.agent_did.clone();
        let secret_key = self.nodehello_template.secret_key;
        let state = match &mut self.receipt_state {
            Some(state) => state,
            None => return Ok(()),
        };
        if decision == "accept" && !state.emit_policy_accepts {
            return Ok(());
        }
        if decision == "reject" && !state.emit_policy_denies {
            return Ok(());
        }

        let mut details = Vec::new();
        details.push((
            CborValue::Unsigned(0),
            CborValue::Text(decision.to_string()),
        ));
        details.push((CborValue::Unsigned(1), CborValue::Text(reason.to_string())));
        if let Some(payload) = payload {
            details.push((
                CborValue::Unsigned(2),
                CborValue::Text(payload.topic.clone()),
            ));
            details.push((
                CborValue::Unsigned(3),
                CborValue::Text(payload.sender.clone()),
            ));
            details.push((
                CborValue::Unsigned(4),
                CborValue::Unsigned(payload.payload_type as u64),
            ));
            details.push((CborValue::Unsigned(5), CborValue::Unsigned(payload.seq)));
            details.push((CborValue::Unsigned(6), CborValue::Unsigned(payload.ts)));
        }
        if let Some(peer) = peer {
            details.push((CborValue::Unsigned(7), CborValue::Text(peer.to_string())));
        }
        if let Some(message_id) = message_id {
            details.push((
                CborValue::Unsigned(8),
                CborValue::Text(format!("{message_id:?}")),
            ));
        }
        let economics = economics_from_proof(payload.and_then(|p| p.economic_proof.as_ref()))?;
        Self::emit_receipt_with(
            state,
            &actor,
            &secret_key,
            EV_POLICY_DECISION,
            CborValue::Map(details),
            economics,
        )
    }

    fn emit_kill_switch_receipt(&mut self, payload: &KillSwitchPayload) -> Result<()> {
        let actor = self.agent_did.clone();
        let secret_key = self.nodehello_template.secret_key;
        let state = match &mut self.receipt_state {
            Some(state) => state,
            None => return Ok(()),
        };
        if !state.emit_kill_switch {
            return Ok(());
        }
        let details = CborValue::Map(vec![
            (
                CborValue::Unsigned(0),
                CborValue::Unsigned(payload.action as u64),
            ),
            (
                CborValue::Unsigned(1),
                CborValue::Text(payload.reason.clone()),
            ),
            (CborValue::Unsigned(2), CborValue::Unsigned(payload.ts)),
            (
                CborValue::Unsigned(3),
                CborValue::Bytes(payload.nonce.clone()),
            ),
        ]);
        Self::emit_receipt_with(
            state,
            &actor,
            &secret_key,
            EV_GOVERNANCE_EVENT,
            details,
            CborValue::Map(Vec::new()),
        )
    }

    fn emit_receipt_spec(&mut self, spec: ReceiptSpec) -> Result<()> {
        let actor = self.agent_did.clone();
        let secret_key = self.nodehello_template.secret_key;
        let state = match &mut self.receipt_state {
            Some(state) => state,
            None => return Ok(()),
        };
        Self::emit_receipt_with(
            state,
            &actor,
            &secret_key,
            spec.event_type,
            spec.details,
            spec.economics,
        )
    }

    fn emit_receipt_with(
        state: &mut ReceiptState,
        actor: &str,
        secret_key: &[u8; 32],
        event_type: u64,
        details: CborValue,
        economics: CborValue,
    ) -> Result<()> {
        let receipt_id = new_receipt_id();
        let ts = unix_time();
        let auth = CborValue::Map(vec![
            (CborValue::Unsigned(0), CborValue::Array(Vec::new())),
            (CborValue::Unsigned(1), CborValue::Null),
            (
                CborValue::Unsigned(2),
                CborValue::Bytes(state.policy_hash.to_vec()),
            ),
        ]);
        let prev_hash = state.log.last_hash().to_vec();
        let seq = state.log.last_seq().saturating_add(1);
        let payload = CborValue::Map(vec![
            (CborValue::Unsigned(0), CborValue::Text(receipt_id)),
            (CborValue::Unsigned(1), CborValue::Unsigned(ts)),
            (CborValue::Unsigned(2), CborValue::Text(actor.to_string())),
            (CborValue::Unsigned(3), CborValue::Null),
            (CborValue::Unsigned(4), CborValue::Null),
            (
                CborValue::Unsigned(5),
                CborValue::Map(vec![
                    (CborValue::Unsigned(0), CborValue::Unsigned(event_type)),
                    (CborValue::Unsigned(1), details),
                ]),
            ),
            (CborValue::Unsigned(6), auth),
            (CborValue::Unsigned(7), economics),
            (CborValue::Unsigned(8), CborValue::Bytes(prev_hash)),
            (CborValue::Unsigned(9), CborValue::Unsigned(seq)),
        ]);

        let payload_cbor = encode_canonical(&payload)?;
        let hash = sha256(&payload_cbor);
        let signature = sign_ed25519_hash(secret_key, &hash)?;
        state
            .log
            .append(&payload_cbor, &signature)
            .context("append receipt")?;
        Ok(())
    }

    async fn validate_economic_proof(
        &self,
        payload: &PubSubEnvelopePayload,
        peer: &PeerId,
        message_id: &gossipsub::MessageId,
    ) -> Result<bool> {
        let Some(cmd_parts) = self.pubsub.economic_proof_validator_cmd() else {
            return Ok(true);
        };
        let proof = match payload.economic_proof.as_ref() {
            Some(proof) => proof,
            None => return Ok(false),
        };
        let proof_type = match proof {
            EconomicProof::OnChainTx { .. } => 1u64,
            EconomicProof::Voucher { .. } => 2u64,
        };
        let proof_cbor = encode_canonical(&proof.to_cbor())?;
        let request = EconomicProofValidationRequest {
            proof_cbor_hex: hex::encode(proof_cbor),
            proof_type,
            topic: payload.topic.clone(),
            sender: payload.sender.clone(),
            payload_type: payload.payload_type,
            seq: payload.seq,
            ts: payload.ts,
            message_id: format!("{message_id:?}"),
            peer_id: peer.to_string(),
        };
        let input = serde_json::to_vec(&request).context("encode economic proof request")?;
        let mut command = Command::new(&cmd_parts[0]);
        if cmd_parts.len() > 1 {
            command.args(&cmd_parts[1..]);
        }
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        let mut child = command.spawn().context("spawn economic proof validator")?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(&input)
                .await
                .context("write economic proof request")?;
        }
        let timeout = self.pubsub.economic_proof_validator_timeout_ms();
        let output =
            tokio::time::timeout(Duration::from_millis(timeout), child.wait_with_output()).await;
        match output {
            Ok(Ok(result)) => {
                if result.status.success() {
                    return Ok(true);
                }
                let stderr = String::from_utf8_lossy(&result.stderr);
                let stderr_trimmed = stderr.trim();
                if !stderr_trimmed.is_empty() {
                    warn!("economic proof validator rejected: {}", stderr_trimmed);
                }
                Ok(false)
            }
            Ok(Err(err)) => Err(err).context("economic proof validator failed"),
            Err(_) => {
                warn!("economic proof validator timeout");
                Ok(false)
            }
        }
    }

    fn maybe_publish_dht(&mut self) -> Result<()> {
        let publish_config = match &mut self.dht {
            Some(state) => {
                if StdInstant::now() < state.next_publish {
                    return Ok(());
                }
                state.next_publish = StdInstant::now() + state.interval;
                Some(state.config.clone())
            }
            None => None,
        };
        if let Some(config) = publish_config {
            self.publish_dht_records(&config)?;
        }
        Ok(())
    }

    fn publish_dht_records(&mut self, config: &DhtConfig) -> Result<()> {
        if self.kill_switch_engaged {
            warn!("kill switch engaged: skipping DHT publish");
            return Ok(());
        }
        if let Some(agent_cfg) = &config.agent_record {
            if let Some(bytes) = self.build_agent_record(agent_cfg)? {
                let expires = unix_time().saturating_add(agent_cfg.expires_sec);
                let expires_at = instant_from_unix(expires)?;
                self.put_record(agent_cfg.record_key.clone(), bytes, expires_at)?;
            }
        }
        for service_cfg in &config.service_records {
            let bytes = self.build_service_record(service_cfg)?;
            let expires = unix_time().saturating_add(service_cfg.expires_sec);
            let expires_at = instant_from_unix(expires)?;
            self.put_record(service_cfg.record_key.clone(), bytes, expires_at)?;
        }
        if let Some(community_cfg) = &config.community_record {
            let bytes = self.build_community_record(community_cfg)?;
            let expires = unix_time().saturating_add(community_cfg.expires_sec);
            let expires_at = instant_from_unix(expires)?;
            self.put_record(community_cfg.record_key.clone(), bytes, expires_at)?;
        }
        Ok(())
    }

    fn build_agent_record(&self, config: &AgentRecordConfig) -> Result<Option<Vec<u8>>> {
        if config.expires_sec == 0 {
            anyhow::bail!("agent record expires_sec must be > 0");
        }
        let mut addrs = Vec::new();
        for addr in &self.listen_addrs {
            let addr_with_peer = ensure_peer_in_addr(addr.clone(), *self.swarm.local_peer_id());
            addrs.push(addr_with_peer.to_string());
        }
        if addrs.is_empty() {
            warn!("no listen addresses yet; postponing agent record publish");
            return Ok(None);
        }
        let node_ids = vec![self.nodehello_template.node_id.clone()];
        let contact = anetsdk::Contact { node_ids, addrs };
        let mut pubkeys = Vec::new();
        if !config.agent_pubkeys_hex.is_empty() {
            for hex_key in &config.agent_pubkeys_hex {
                let bytes = hex::decode(hex_key).context("decode agent_pubkeys_hex")?;
                if bytes.len() != 32 {
                    anyhow::bail!("agent_pubkeys_hex must be 32 bytes");
                }
                pubkeys.push(bytes);
            }
        } else {
            pubkeys.push(self.nodehello_template.node_pubkey.clone());
        }
        let payload = AgentRecordPayload {
            agent_id: self.agent_did.clone(),
            agent_pubkeys: pubkeys,
            contact,
            capabilities: config.capabilities.clone(),
            expires: unix_time().saturating_add(config.expires_sec),
        };
        let secret = self.load_signing_key(config.signing_key_path.as_ref())?;
        Ok(Some(build_agent_record(&payload, &secret)?))
    }

    fn build_service_record(&self, config: &ServiceRecordConfig) -> Result<Vec<u8>> {
        if config.expires_sec == 0 {
            anyhow::bail!("service record expires_sec must be > 0");
        }
        if config.addrs.is_empty() {
            anyhow::bail!("service record requires at least one addr");
        }
        let pricing = match &config.pricing_cbor_path {
            Some(path) => {
                let bytes = std::fs::read(path)
                    .with_context(|| format!("read pricing {}", path.display()))?;
                Some(anetsdk::decode_canonical(&bytes)?)
            }
            None => None,
        };
        let payload = ServiceRecordPayload {
            provider_id: config
                .provider_id
                .clone()
                .unwrap_or_else(|| self.agent_did.clone()),
            service_type: config.service_type,
            addrs: config.addrs.clone(),
            required_credentials: config.required_credentials.clone(),
            pricing,
            expires: unix_time().saturating_add(config.expires_sec),
        };
        let secret = self.load_signing_key(config.signing_key_path.as_ref())?;
        Ok(build_service_record(&payload, &secret)?)
    }

    fn build_community_record(&self, config: &CommunityRecordConfig) -> Result<Vec<u8>> {
        if config.expires_sec == 0 {
            anyhow::bail!("community record expires_sec must be > 0");
        }
        let economics_bytes = std::fs::read(&config.economics_cbor_path)
            .with_context(|| format!("read economics {}", config.economics_cbor_path.display()))?;
        let governance_bytes = std::fs::read(&config.governance_cbor_path).with_context(|| {
            format!("read governance {}", config.governance_cbor_path.display())
        })?;
        let payload = CommunityRecordPayload {
            community_id: config.community_id.clone(),
            controller: config
                .controller
                .clone()
                .unwrap_or_else(|| self.agent_did.clone()),
            join_policy: config.join_policy,
            required_credentials: config.required_credentials.clone(),
            economics: anetsdk::decode_canonical(&economics_bytes)?,
            governance: anetsdk::decode_canonical(&governance_bytes)?,
            expires: unix_time().saturating_add(config.expires_sec),
        };
        let secret = self.load_signing_key(config.signing_key_path.as_ref())?;
        Ok(build_community_record(&payload, &secret)?)
    }

    fn load_signing_key(&self, path: Option<&std::path::PathBuf>) -> Result<Vec<u8>> {
        if let Some(path) = path {
            let data = std::fs::read_to_string(path)
                .with_context(|| format!("read signing key {}", path.display()))?;
            let decoded = BASE64.decode(data.trim()).context("decode signing key")?;
            if decoded.len() != 32 {
                anyhow::bail!("signing key must be 32 bytes");
            }
            return Ok(decoded);
        }
        Ok(self.nodehello_template.secret_key.to_vec())
    }

    fn is_kill_switch_payload(&self, payload_type: u16) -> bool {
        match &self.kill_switch {
            Some(kill) => kill.config.payload_type() == payload_type,
            None => false,
        }
    }

    fn is_agentmail_payload(&self, payload_type: u16) -> bool {
        payload_type == self.agentmail_config.payload_type()
    }

    fn handle_agentmail_payload(
        &mut self,
        payload: &PubSubEnvelopePayload,
        propagation_source: &PeerId,
        message_id: &gossipsub::MessageId,
    ) -> Result<()> {
        if !self.agentmail_config.enabled() {
            let _ = self.emit_policy_decision(
                "reject",
                "agentmail disabled",
                Some(payload),
                Some(propagation_source),
                Some(message_id),
            );
            return Ok(());
        }
        let Some(state) = &mut self.agentmail else {
            let _ = self.emit_policy_decision(
                "reject",
                "agentmail state unavailable",
                Some(payload),
                Some(propagation_source),
                Some(message_id),
            );
            return Ok(());
        };

        let sender = payload.sender.clone();
        let sender_pubkey = match state.resolve_sender_pubkey(&sender, self.tx_engine.as_ref()) {
            Some(pk) => pk,
            None => {
                let _ = self.emit_policy_decision(
                    "reject",
                    "unknown sender pubkey",
                    Some(payload),
                    Some(propagation_source),
                    Some(message_id),
                );
                return Ok(());
            }
        };

        let message_bytes = match encode_canonical(&payload.payload) {
            Ok(bytes) => bytes,
            Err(_) => {
                let _ = self.emit_policy_decision(
                    "reject",
                    "invalid agentmail payload",
                    Some(payload),
                    Some(propagation_source),
                    Some(message_id),
                );
                return Ok(());
            }
        };

        let message_payload = match verify_agentmail_message(&message_bytes, &sender_pubkey) {
            Ok(message) => message,
            Err(_) => {
                let _ = self.emit_policy_decision(
                    "reject",
                    "agentmail signature invalid",
                    Some(payload),
                    Some(propagation_source),
                    Some(message_id),
                );
                return Ok(());
            }
        };

        if state.config.enforce_sender_match() && message_payload.sender != sender {
            let _ = self.emit_policy_decision(
                "reject",
                "sender mismatch",
                Some(payload),
                Some(propagation_source),
                Some(message_id),
            );
            return Ok(());
        }

        let now = unix_time();
        let skew = (message_payload.ts as i64 - now as i64).abs();
        if skew > state.config.max_clock_skew_sec() {
            let _ = self.emit_policy_decision(
                "reject",
                "agentmail timestamp skew",
                Some(payload),
                Some(propagation_source),
                Some(message_id),
            );
            return Ok(());
        }
        if let Some(expires) = message_payload.expires {
            if expires < now {
                let _ = self.emit_policy_decision(
                    "reject",
                    "agentmail expired",
                    Some(payload),
                    Some(propagation_source),
                    Some(message_id),
                );
                return Ok(());
            }
        }

        match state
            .seen
            .record(&message_payload.message_id, message_payload.ts)
        {
            Ok(false) => {
                let _ = self.emit_policy_decision(
                    "reject",
                    "agentmail replay",
                    Some(payload),
                    Some(propagation_source),
                    Some(message_id),
                );
                return Ok(());
            }
            Ok(true) => {}
            Err(err) => {
                warn!("agentmail replay cache error: {err:?}");
                let _ = self.emit_policy_decision(
                    "reject",
                    "agentmail replay cache error",
                    Some(payload),
                    Some(propagation_source),
                    Some(message_id),
                );
                return Ok(());
            }
        }

        if state.deny_senders.contains(&sender) {
            let _ = self.emit_policy_decision(
                "reject",
                "sender denied",
                Some(payload),
                Some(propagation_source),
                Some(message_id),
            );
            return Ok(());
        }
        if !state.allow_senders.is_empty() && !state.allow_senders.contains(&sender) {
            let _ = self.emit_policy_decision(
                "reject",
                "sender not allowlisted",
                Some(payload),
                Some(propagation_source),
                Some(message_id),
            );
            return Ok(());
        }
        if state.config.require_postage_for_unknown()
            && !state.allow_senders.contains(&sender)
            && payload.economic_proof.is_none()
        {
            let _ = self.emit_policy_decision(
                "reject",
                "missing postage proof",
                Some(payload),
                Some(propagation_source),
                Some(message_id),
            );
            return Ok(());
        }

        if state.config.require_recipient()
            && !message_payload
                .recipients
                .iter()
                .any(|recipient| recipient == &self.agent_did)
        {
            let _ = self.emit_policy_decision(
                "reject",
                "recipient mismatch",
                Some(payload),
                Some(propagation_source),
                Some(message_id),
            );
            return Ok(());
        }

        let markdown_bytes = message_payload.markdown.as_bytes().len() as u64;
        if markdown_bytes > state.config.max_markdown_bytes() {
            let _ = self.emit_policy_decision(
                "reject",
                "markdown too large",
                Some(payload),
                Some(propagation_source),
                Some(message_id),
            );
            return Ok(());
        }

        if let Some(attachments) = &message_payload.attachments {
            if attachments.len() > state.config.max_attachments() {
                let _ = self.emit_policy_decision(
                    "reject",
                    "too many attachments",
                    Some(payload),
                    Some(propagation_source),
                    Some(message_id),
                );
                return Ok(());
            }
            let mut total_size: u64 = 0;
            for attachment in attachments {
                total_size = total_size.saturating_add(attachment.size_bytes);
                if attachment.size_bytes > state.config.max_attachment_bytes() {
                    let _ = self.emit_policy_decision(
                        "reject",
                        "attachment too large",
                        Some(payload),
                        Some(propagation_source),
                        Some(message_id),
                    );
                    return Ok(());
                }
            }
            if total_size > state.config.max_total_attachment_bytes() {
                let _ = self.emit_policy_decision(
                    "reject",
                    "attachments exceed size limit",
                    Some(payload),
                    Some(propagation_source),
                    Some(message_id),
                );
                return Ok(());
            }
        }

        if let Err(err) = state.inbox.append(&message_bytes) {
            warn!("agentmail inbox append failed: {err:?}");
            let _ = self.emit_policy_decision(
                "reject",
                "agentmail inbox write failed",
                Some(payload),
                Some(propagation_source),
                Some(message_id),
            );
            return Ok(());
        }

        self.log_pubsub_payload(payload);
        if let Err(err) = self.emit_policy_decision(
            "accept",
            "agentmail accepted",
            Some(payload),
            Some(propagation_source),
            Some(message_id),
        ) {
            warn!("receipt emit failed: {err:?}");
        }
        Ok(())
    }

    fn handle_kill_switch_payload(&mut self, envelope: &PubSubEnvelopePayload) -> Result<()> {
        let kill_switch = match &mut self.kill_switch {
            Some(kill) => kill,
            None => return Ok(()),
        };
        if envelope.topic != kill_switch.config.topic() {
            anyhow::bail!("kill switch topic mismatch");
        }
        let payload = parse_kill_switch_payload(&envelope.payload)?;
        verify_kill_switch_payload(
            &payload,
            &kill_switch.pubkey,
            kill_switch.config.max_clock_skew_sec(),
        )?;
        if payload.nonce.len() != 16 {
            anyhow::bail!("invalid kill switch nonce length");
        }
        if kill_switch.seen_nonces.contains(&payload.nonce) {
            anyhow::bail!("kill switch replay detected");
        }
        kill_switch.seen_nonces.insert(payload.nonce.clone());
        kill_switch.nonce_order.push_back(payload.nonce.clone());
        while kill_switch.nonce_order.len() > kill_switch.config.replay_window() {
            if let Some(old) = kill_switch.nonce_order.pop_front() {
                kill_switch.seen_nonces.remove(&old);
            }
        }

        match payload.action {
            0 => {
                if !self.kill_switch_engaged {
                    self.kill_switch_engaged = true;
                    warn!("kill switch engaged: {}", payload.reason);
                }
            }
            1 => {
                if !kill_switch.config.allow_release() {
                    anyhow::bail!("kill switch release disabled by config");
                }
                if self.kill_switch_engaged {
                    self.kill_switch_engaged = false;
                    warn!("kill switch released: {}", payload.reason);
                }
            }
            _ => anyhow::bail!("invalid kill switch action"),
        }
        if let Err(err) = self.emit_kill_switch_receipt(&payload) {
            warn!("receipt emit failed: {err:?}");
        }
        Ok(())
    }
}

fn unix_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs()
}

fn new_receipt_id() -> String {
    let mut bytes = [0u8; 16];
    OsRng.fill_bytes(&mut bytes);
    BASE64.encode(bytes)
}

fn economics_from_proof(proof: Option<&EconomicProof>) -> Result<CborValue> {
    let Some(proof) = proof else {
        return Ok(CborValue::Map(Vec::new()));
    };
    let proof_type = match proof {
        EconomicProof::OnChainTx { .. } => 1u64,
        EconomicProof::Voucher { .. } => 2u64,
    };
    let proof_cbor = encode_canonical(&proof.to_cbor())?;
    let proof_hash = sha256(&proof_cbor);
    Ok(CborValue::Map(vec![
        (CborValue::Unsigned(0), CborValue::Unsigned(proof_type)),
        (
            CborValue::Unsigned(1),
            CborValue::Bytes(proof_hash.to_vec()),
        ),
    ]))
}

impl RateLimiter {
    fn allow(&mut self, sender: &str, msg_bytes: usize, now: u64) -> bool {
        let window = self
            .state
            .entry(sender.to_string())
            .or_insert(SenderWindow {
                window_start: now,
                count: 0,
                bytes: 0,
            });
        if now.saturating_sub(window.window_start) >= self.window_sec {
            window.window_start = now;
            window.count = 0;
            window.bytes = 0;
        }
        let next_count = window.count.saturating_add(1);
        let next_bytes = window.bytes.saturating_add(msg_bytes as u64);
        if let Some(max_messages) = self.max_messages {
            if next_count > max_messages {
                return false;
            }
        }
        if let Some(max_bytes) = self.max_bytes {
            if next_bytes > max_bytes {
                return false;
            }
        }
        window.count = next_count;
        window.bytes = next_bytes;
        true
    }
}

fn peer_id_from_ed25519(pk_bytes: &[u8]) -> Result<PeerId> {
    let public = identity::ed25519::PublicKey::try_from_bytes(pk_bytes)
        .context("invalid ed25519 public key")?;
    let public_key = identity::PublicKey::from(public);
    Ok(PeerId::from_public_key(&public_key))
}

fn keypair_from_secret(mut secret: [u8; 32]) -> Result<identity::Keypair> {
    let keypair = identity::Keypair::ed25519_from_bytes(&mut secret)
        .context("failed to build libp2p keypair")?;
    Ok(keypair)
}

fn build_dht_state(config: &DhtConfig) -> Result<Option<DhtState>> {
    if !config.enabled() {
        return Ok(None);
    }
    let mut ttls = Vec::new();
    if let Some(agent) = &config.agent_record {
        ttls.push(agent.expires_sec);
    }
    for service in &config.service_records {
        ttls.push(service.expires_sec);
    }
    if let Some(community) = &config.community_record {
        ttls.push(community.expires_sec);
    }
    if ttls.is_empty() {
        anyhow::bail!("dht enabled but no records configured");
    }
    let min_ttl = *ttls.iter().min().unwrap_or(&0);
    if min_ttl < 60 {
        anyhow::bail!("dht record expires_sec must be at least 60");
    }
    let interval_sec = config.publish_interval_sec(min_ttl / 2);
    if interval_sec == 0 {
        anyhow::bail!("publish_interval_sec must be > 0");
    }
    Ok(Some(DhtState {
        config: config.clone(),
        interval: Duration::from_secs(interval_sec),
        next_publish: StdInstant::now(),
    }))
}

fn ensure_peer_in_addr(addr: Multiaddr, peer: PeerId) -> Multiaddr {
    let mut has_peer = false;
    for proto in addr.iter() {
        if matches!(proto, libp2p::multiaddr::Protocol::P2p(_)) {
            has_peer = true;
            break;
        }
    }
    if has_peer {
        addr
    } else {
        addr.clone().with_p2p(peer).unwrap_or(addr)
    }
}

fn instant_from_unix(expires_ts: u64) -> Result<Option<std::time::Instant>> {
    let now = unix_time();
    if expires_ts <= now {
        return Ok(None);
    }
    let delta = expires_ts - now;
    Ok(Some(
        std::time::Instant::now() + std::time::Duration::from_secs(delta),
    ))
}

fn build_kill_switch(config: &KillSwitchConfig) -> Result<Option<KillSwitchState>> {
    if !config.enabled() {
        return Ok(None);
    }
    let pubkey_hex = config
        .pubkey_hex
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("kill switch enabled without pubkey"))?;
    let pubkey = hex::decode(pubkey_hex).context("decode kill switch pubkey")?;
    if pubkey.len() != 32 {
        anyhow::bail!("kill switch pubkey must be 32 bytes");
    }
    Ok(Some(KillSwitchState {
        config: config.clone(),
        pubkey,
        seen_nonces: HashSet::new(),
        nonce_order: VecDeque::new(),
    }))
}

fn build_agentmail_state(
    config: &AgentMailConfig,
    state_dir: &std::path::Path,
) -> Result<Option<AgentMailState>> {
    if !config.enabled() {
        return Ok(None);
    }
    let inbox_path = config.inbox_path_or_default(state_dir);
    if let Some(parent) = inbox_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create agentmail inbox dir {}", parent.display()))?;
    }
    let inbox = AgentMailInbox::open(&inbox_path)?;

    let seen_path = config.seen_path_or_default(state_dir);
    if let Some(parent) = seen_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create agentmail seen dir {}", parent.display()))?;
    }
    let seen = AgentMailSeen::load(
        &seen_path,
        config.retention_sec(),
        config.max_seen_entries(),
    )?;

    let sender_pubkeys = parse_sender_pubkeys(config.sender_pubkeys())?;
    let allow_senders: HashSet<String> = config.allow_senders().iter().cloned().collect();
    let deny_senders: HashSet<String> = config.deny_senders().iter().cloned().collect();
    for sender in &allow_senders {
        if deny_senders.contains(sender) {
            anyhow::bail!("agentmail sender present in both allow and deny lists");
        }
    }

    Ok(Some(AgentMailState {
        config: config.clone(),
        sender_pubkeys,
        allow_senders,
        deny_senders,
        inbox,
        seen,
    }))
}

fn build_rate_limiter(config: &RateLimitConfig) -> Result<Option<RateLimiter>> {
    if !config.enabled() {
        return Ok(None);
    }
    let window_sec = config
        .window_sec()
        .ok_or_else(|| anyhow::anyhow!("rate limits enabled without window_sec"))?;
    let max_messages = config.max_messages();
    let max_bytes = config.max_bytes();
    if max_messages.is_none() && max_bytes.is_none() {
        anyhow::bail!("rate limits enabled without max_messages or max_bytes");
    }
    Ok(Some(RateLimiter {
        window_sec,
        max_messages,
        max_bytes,
        state: HashMap::new(),
    }))
}

fn parse_sender_pubkeys(entries: &[SenderKeyConfig]) -> Result<HashMap<String, Vec<u8>>> {
    let mut map = HashMap::new();
    for entry in entries {
        let pubkey = hex::decode(&entry.pubkey_hex)
            .with_context(|| format!("decode sender pubkey {}", entry.did))?;
        if pubkey.len() != 32 {
            anyhow::bail!("sender pubkey must be 32 bytes");
        }
        map.insert(entry.did.clone(), pubkey);
    }
    Ok(map)
}

fn build_receipt_state(
    config: &Config,
    state_dir: &std::path::Path,
) -> Result<Option<ReceiptState>> {
    if !config.receipts.enabled() {
        return Ok(None);
    }
    let path = config.receipts.path_or_default(state_dir);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create receipt dir {}", parent.display()))?;
    }
    let log = ReceiptLog::open(&path).context("open receipt log")?;
    let policy_hash = build_policy_hash(
        &config.pubsub,
        &config.handshake,
        &config.kill_switch,
        &config.tx,
        &config.rate_limits,
        &config.agentmail,
    )?;
    Ok(Some(ReceiptState {
        log,
        policy_hash,
        emit_policy_accepts: config.receipts.emit_policy_accepts(),
        emit_policy_denies: config.receipts.emit_policy_denies(),
        emit_kill_switch: config.receipts.emit_kill_switch(),
    }))
}

fn build_policy_hash(
    pubsub: &PubSubConfig,
    handshake: &HandshakeConfig,
    kill_switch: &KillSwitchConfig,
    tx: &TxConfig,
    rate_limits: &RateLimitConfig,
    agentmail: &crate::config::AgentMailConfig,
) -> Result<[u8; 32]> {
    let mut entries = Vec::new();
    entries.push((
        CborValue::Unsigned(0),
        CborValue::Bool(pubsub.require_economic_proof()),
    ));
    entries.push((
        CborValue::Unsigned(1),
        CborValue::Bool(pubsub.verify_signatures()),
    ));
    entries.push((
        CborValue::Unsigned(2),
        CborValue::Bool(pubsub.economic_proof_fail_open()),
    ));
    entries.push((
        CborValue::Unsigned(3),
        CborValue::Unsigned(pubsub.economic_proof_validator_timeout_ms()),
    ));
    entries.push((
        CborValue::Unsigned(4),
        CborValue::Bool(pubsub.economic_proof_validator_cmd().is_some()),
    ));
    entries.push((
        CborValue::Unsigned(5),
        CborValue::Bool(kill_switch.enabled()),
    ));
    entries.push((CborValue::Unsigned(6), CborValue::Text(kill_switch.topic())));
    entries.push((
        CborValue::Unsigned(7),
        CborValue::Unsigned(kill_switch.payload_type() as u64),
    ));
    entries.push((
        CborValue::Unsigned(8),
        CborValue::Bool(kill_switch.allow_release()),
    ));
    entries.push((
        CborValue::Unsigned(9),
        skew_to_cbor(kill_switch.max_clock_skew_sec()),
    ));
    entries.push((
        CborValue::Unsigned(10),
        CborValue::Unsigned(kill_switch.replay_window() as u64),
    ));
    entries.push((
        CborValue::Unsigned(11),
        skew_to_cbor(handshake.max_clock_skew_sec()),
    ));
    entries.push((
        CborValue::Unsigned(12),
        CborValue::Bool(handshake.require_peer_id_match()),
    ));
    entries.push((CborValue::Unsigned(13), CborValue::Bool(tx.enabled())));
    if let Some(payload_type) = tx.pubsub_payload_type() {
        entries.push((
            CborValue::Unsigned(14),
            CborValue::Unsigned(payload_type as u64),
        ));
    }
    entries.push((
        CborValue::Unsigned(15),
        CborValue::Unsigned(tx.sender_pubkeys().len() as u64),
    ));
    entries.push((
        CborValue::Unsigned(16),
        CborValue::Bool(tx.escrow.enabled()),
    ));
    entries.push((
        CborValue::Unsigned(17),
        CborValue::Unsigned(tx.escrow.arbitrators.len() as u64),
    ));
    entries.push((
        CborValue::Unsigned(18),
        CborValue::Bool(rate_limits.enabled()),
    ));
    if let Some(window) = rate_limits.window_sec() {
        entries.push((CborValue::Unsigned(19), CborValue::Unsigned(window)));
    }
    if let Some(max_messages) = rate_limits.max_messages() {
        entries.push((CborValue::Unsigned(20), CborValue::Unsigned(max_messages)));
    }
    if let Some(max_bytes) = rate_limits.max_bytes() {
        entries.push((CborValue::Unsigned(21), CborValue::Unsigned(max_bytes)));
    }
    entries.push((
        CborValue::Unsigned(22),
        CborValue::Bool(tx.identity.enabled()),
    ));
    entries.push((
        CborValue::Unsigned(23),
        CborValue::Bool(tx.identity.allow_register()),
    ));
    entries.push((
        CborValue::Unsigned(24),
        CborValue::Bool(tx.identity.allow_rotate()),
    ));
    entries.push((
        CborValue::Unsigned(25),
        CborValue::Bool(tx.identity.allow_revoke()),
    ));
    entries.push((
        CborValue::Unsigned(26),
        skew_to_cbor(tx.identity.max_clock_skew_sec()),
    ));
    entries.push((
        CborValue::Unsigned(27),
        CborValue::Bool(tx.budget.enabled()),
    ));
    if let Some(window) = tx.budget.window_sec() {
        entries.push((CborValue::Unsigned(28), CborValue::Unsigned(window)));
    }
    entries.push((
        CborValue::Unsigned(29),
        CborValue::Unsigned(tx.budget.caps().len() as u64),
    ));
    entries.push((
        CborValue::Unsigned(30),
        CborValue::Bool(tx.skill_registry.enabled()),
    ));
    entries.push((
        CborValue::Unsigned(31),
        CborValue::Bool(tx.skill_registry.allow_publish()),
    ));
    entries.push((
        CborValue::Unsigned(32),
        CborValue::Bool(tx.skill_registry.allow_update()),
    ));
    entries.push((
        CborValue::Unsigned(33),
        CborValue::Bool(tx.skill_registry.allow_revoke()),
    ));
    entries.push((
        CborValue::Unsigned(34),
        skew_to_cbor(tx.skill_registry.max_clock_skew_sec()),
    ));
    entries.push((
        CborValue::Unsigned(35),
        CborValue::Bool(tx.work_registry.enabled()),
    ));
    entries.push((
        CborValue::Unsigned(36),
        CborValue::Bool(tx.work_registry.allow_offer_publish()),
    ));
    entries.push((
        CborValue::Unsigned(37),
        CborValue::Bool(tx.work_registry.allow_agreement_publish()),
    ));
    entries.push((
        CborValue::Unsigned(38),
        CborValue::Bool(tx.work_registry.allow_agreement_update()),
    ));
    entries.push((
        CborValue::Unsigned(39),
        CborValue::Bool(tx.work_registry.allow_agreement_close()),
    ));
    entries.push((
        CborValue::Unsigned(40),
        skew_to_cbor(tx.work_registry.max_clock_skew_sec()),
    ));
    entries.push((
        CborValue::Unsigned(41),
        CborValue::Bool(agentmail.enabled()),
    ));
    entries.push((
        CborValue::Unsigned(42),
        CborValue::Unsigned(agentmail.payload_type() as u64),
    ));
    entries.push((
        CborValue::Unsigned(43),
        CborValue::Bool(agentmail.require_recipient()),
    ));
    entries.push((
        CborValue::Unsigned(44),
        CborValue::Bool(agentmail.enforce_sender_match()),
    ));
    entries.push((
        CborValue::Unsigned(45),
        CborValue::Bool(agentmail.require_postage_for_unknown()),
    ));
    entries.push((
        CborValue::Unsigned(46),
        skew_to_cbor(agentmail.max_clock_skew_sec()),
    ));
    entries.push((
        CborValue::Unsigned(47),
        CborValue::Unsigned(agentmail.max_markdown_bytes()),
    ));
    entries.push((
        CborValue::Unsigned(48),
        CborValue::Unsigned(agentmail.max_attachments() as u64),
    ));
    entries.push((
        CborValue::Unsigned(49),
        CborValue::Unsigned(agentmail.max_attachment_bytes()),
    ));
    entries.push((
        CborValue::Unsigned(50),
        CborValue::Unsigned(agentmail.max_total_attachment_bytes()),
    ));
    entries.push((
        CborValue::Unsigned(51),
        CborValue::Unsigned(agentmail.allow_senders().len() as u64),
    ));
    entries.push((
        CborValue::Unsigned(52),
        CborValue::Unsigned(agentmail.deny_senders().len() as u64),
    ));
    entries.push((
        CborValue::Unsigned(53),
        CborValue::Unsigned(agentmail.retention_sec()),
    ));
    entries.push((
        CborValue::Unsigned(54),
        CborValue::Unsigned(agentmail.max_seen_entries() as u64),
    ));
    let policy = CborValue::Map(entries);
    let encoded = encode_canonical(&policy)?;
    Ok(sha256(&encoded))
}

fn skew_to_cbor(skew: i64) -> CborValue {
    if skew >= 0 {
        CborValue::Unsigned(skew as u64)
    } else {
        CborValue::Negative(skew)
    }
}

fn parse_kill_switch_payload(value: &CborValue) -> Result<KillSwitchPayload> {
    let entries = match value {
        CborValue::Map(entries) => entries.clone(),
        _ => anyhow::bail!("kill switch payload must be map"),
    };
    let mut action: Option<u8> = None;
    let mut reason: Option<String> = None;
    let mut ts: Option<u64> = None;
    let mut nonce: Option<Vec<u8>> = None;
    let mut signature: Option<Vec<u8>> = None;

    for (k, v) in entries {
        if let CborValue::Unsigned(n) = k {
            match n {
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

    Ok(KillSwitchPayload {
        action: action.ok_or_else(|| anyhow::anyhow!("kill switch action missing"))?,
        reason: reason.ok_or_else(|| anyhow::anyhow!("kill switch reason missing"))?,
        ts: ts.ok_or_else(|| anyhow::anyhow!("kill switch ts missing"))?,
        nonce: nonce.ok_or_else(|| anyhow::anyhow!("kill switch nonce missing"))?,
        signature: signature.ok_or_else(|| anyhow::anyhow!("kill switch signature missing"))?,
    })
}

fn verify_kill_switch_payload(
    payload: &KillSwitchPayload,
    pubkey: &[u8],
    max_skew: i64,
) -> Result<()> {
    if payload.signature.len() != 64 {
        anyhow::bail!("kill switch signature length invalid");
    }
    if payload.nonce.len() != 16 {
        anyhow::bail!("kill switch nonce length invalid");
    }
    let now = unix_time() as i64;
    let skew = (payload.ts as i64 - now).abs();
    if skew > max_skew {
        anyhow::bail!("kill switch timestamp outside window");
    }
    let payload_map = CborValue::Map(vec![
        (
            CborValue::Unsigned(0),
            CborValue::Unsigned(payload.action as u64),
        ),
        (
            CborValue::Unsigned(1),
            CborValue::Text(payload.reason.clone()),
        ),
        (CborValue::Unsigned(2), CborValue::Unsigned(payload.ts)),
        (
            CborValue::Unsigned(3),
            CborValue::Bytes(payload.nonce.clone()),
        ),
    ]);
    let payload_cbor = anetsdk::encode_canonical(&payload_map)?;
    let hash = sha256(&payload_cbor);
    verify_ed25519_hash(pubkey, &hash, &payload.signature)?;
    Ok(())
}

impl AgentMailState {
    fn resolve_sender_pubkey(&self, did: &str, tx_engine: Option<&TxEngine>) -> Option<Vec<u8>> {
        if let Some(pk) = self.sender_pubkeys.get(did) {
            return Some(pk.clone());
        }
        tx_engine.and_then(|engine| engine.resolve_pubkey(did))
    }
}

impl AgentMailInbox {
    fn open(path: &PathBuf) -> Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(path)
            .with_context(|| format!("open agentmail inbox {}", path.display()))?;
        Ok(Self {
            path: path.clone(),
            file,
        })
    }

    fn append(&mut self, message: &[u8]) -> Result<()> {
        if message.len() > u32::MAX as usize {
            anyhow::bail!("agentmail message too large for inbox log");
        }
        let len = message.len() as u32;
        self.file
            .write_all(&len.to_be_bytes())
            .with_context(|| format!("write agentmail inbox {}", self.path.display()))?;
        self.file
            .write_all(message)
            .with_context(|| format!("write agentmail inbox {}", self.path.display()))?;
        self.file
            .sync_all()
            .with_context(|| format!("sync agentmail inbox {}", self.path.display()))?;
        Ok(())
    }
}

impl AgentMailSeen {
    fn load(path: &PathBuf, retention_sec: u64, max_entries: usize) -> Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(path)
            .with_context(|| format!("open agentmail seen {}", path.display()))?;
        let reader = BufReader::new(file);
        let now = unix_time();
        let mut items: Vec<(u64, String)> = Vec::new();
        for line in reader.lines() {
            let line = line.with_context(|| format!("read agentmail seen {}", path.display()))?;
            let mut parts = line.splitn(2, '\t');
            let ts_str = match parts.next() {
                Some(val) => val,
                None => continue,
            };
            let id = match parts.next() {
                Some(val) => val.trim(),
                None => continue,
            };
            if id.is_empty() {
                continue;
            }
            let ts: u64 = match ts_str.parse() {
                Ok(val) => val,
                Err(_) => continue,
            };
            if now.saturating_sub(ts) <= retention_sec {
                items.push((ts, id.to_string()));
            }
        }
        items.sort_by_key(|(ts, _)| *ts);
        let mut entries = HashMap::new();
        let mut order = VecDeque::new();
        for (ts, id) in items {
            if entries.contains_key(&id) {
                continue;
            }
            entries.insert(id.clone(), ts);
            order.push_back(id);
        }
        let mut seen = AgentMailSeen {
            path: path.clone(),
            retention_sec,
            max_entries,
            entries,
            order,
        };
        seen.prune(now)?;
        Ok(seen)
    }

    fn record(&mut self, message_id: &str, ts: u64) -> Result<bool> {
        if self.entries.contains_key(message_id) {
            return Ok(false);
        }
        self.entries.insert(message_id.to_string(), ts);
        self.order.push_back(message_id.to_string());
        self.append_line(ts, message_id)?;
        self.prune(unix_time())?;
        Ok(true)
    }

    fn append_line(&self, ts: u64, message_id: &str) -> Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .with_context(|| format!("open agentmail seen {}", self.path.display()))?;
        writeln!(file, "{ts}\t{message_id}")
            .with_context(|| format!("write agentmail seen {}", self.path.display()))?;
        file.sync_all()
            .with_context(|| format!("sync agentmail seen {}", self.path.display()))?;
        Ok(())
    }

    fn prune(&mut self, now: u64) -> Result<()> {
        let mut pruned = false;
        while let Some(id) = self.order.front() {
            let ts = self.entries.get(id).copied().unwrap_or(0);
            if now.saturating_sub(ts) > self.retention_sec || self.entries.len() > self.max_entries
            {
                let id = self.order.pop_front().expect("front exists");
                self.entries.remove(&id);
                pruned = true;
            } else {
                break;
            }
        }
        if pruned {
            self.persist()?;
        }
        Ok(())
    }

    fn persist(&self) -> Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.path)
            .with_context(|| format!("open agentmail seen {}", self.path.display()))?;
        for id in &self.order {
            if let Some(ts) = self.entries.get(id) {
                writeln!(file, "{ts}\t{id}")
                    .with_context(|| format!("write agentmail seen {}", self.path.display()))?;
            }
        }
        file.sync_all()
            .with_context(|| format!("sync agentmail seen {}", self.path.display()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AgentMailConfig, Config, FeaturesConfig, HandshakeConfig, PubSubConfig};
    use crate::keys::generate_keypair;
    use base64::engine::general_purpose::STANDARD as BASE64;
    use base64::Engine;
    use std::path::PathBuf;
    use tokio::time::{Duration, Instant};

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn handshake_completes_between_two_nodes() -> Result<()> {
        let keys_a = generate_keypair();
        let keys_b = generate_keypair();
        let chain_id = BASE64.encode(keys_a.verifying_key.to_bytes());
        let agent_did_a = format!(
            "did:anet:agent:{}",
            BASE64.encode(keys_a.verifying_key.to_bytes())
        );
        let agent_did_b = format!(
            "did:anet:agent:{}",
            BASE64.encode(keys_b.verifying_key.to_bytes())
        );

        let temp_base =
            std::env::temp_dir().join(format!("agentmesh-test-{}", rand::random::<u64>()));
        let config_a = Config {
            chain_id: chain_id.clone(),
            agent_did: agent_did_a,
            key_path: PathBuf::new(),
            node_id: None,
            state_dir: Some(temp_base.join("a")),
            listen_addrs: vec!["/ip4/127.0.0.1/udp/0/quic-v1".to_string()],
            bootstrap: vec![],
            protocols: vec![],
            transports: vec![],
            roles: vec![],
            features: FeaturesConfig::default(),
            pubsub: PubSubConfig::default(),
            handshake: HandshakeConfig::default(),
            kill_switch: crate::config::KillSwitchConfig::default(),
            receipts: crate::config::ReceiptConfig::default(),
            tx: crate::config::TxConfig::default(),
            rate_limits: crate::config::RateLimitConfig::default(),
            dht: crate::config::DhtConfig::default(),
            agentmail: AgentMailConfig::default(),
        };
        let config_b = Config {
            chain_id,
            agent_did: agent_did_b,
            key_path: PathBuf::new(),
            node_id: None,
            state_dir: Some(temp_base.join("b")),
            listen_addrs: vec!["/ip4/127.0.0.1/udp/0/quic-v1".to_string()],
            bootstrap: vec![],
            protocols: vec![],
            transports: vec![],
            roles: vec![],
            features: FeaturesConfig::default(),
            pubsub: PubSubConfig::default(),
            handshake: HandshakeConfig::default(),
            kill_switch: crate::config::KillSwitchConfig::default(),
            receipts: crate::config::ReceiptConfig::default(),
            tx: crate::config::TxConfig::default(),
            rate_limits: crate::config::RateLimitConfig::default(),
            dht: crate::config::DhtConfig::default(),
            agentmail: AgentMailConfig::default(),
        };

        let mut node_a = build_mesh(config_a, keys_a)?;
        let mut node_b = build_mesh(config_b, keys_b)?;

        wait_for_listen(&mut node_a).await?;
        let addr_b = wait_for_listen(&mut node_b).await?;

        let b_peer = *node_b.swarm.local_peer_id();
        let dial_addr = addr_b
            .with_p2p(b_peer)
            .map_err(|_| anyhow::anyhow!("invalid dial addr"))?;
        node_a.swarm.dial(dial_addr)?;

        let a_peer = *node_a.swarm.local_peer_id();
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            tokio::select! {
                event = node_a.swarm.select_next_some() => {
                    node_a.handle_event(event).await?;
                }
                event = node_b.swarm.select_next_some() => {
                    node_b.handle_event(event).await?;
                }
                _ = tokio::time::sleep(Duration::from_millis(10)) => {}
            }
            if node_a.peer_hellos.contains_key(&b_peer) && node_b.peer_hellos.contains_key(&a_peer)
            {
                return Ok(());
            }
        }

        anyhow::bail!("handshake did not complete in time");
    }

    async fn wait_for_listen(node: &mut MeshNode) -> Result<Multiaddr> {
        loop {
            let event = node.swarm.select_next_some().await;
            let listen_addr = match &event {
                SwarmEvent::NewListenAddr { address, .. } => Some(address.clone()),
                _ => None,
            };
            node.handle_event(event).await?;
            if let Some(address) = listen_addr {
                return Ok(address);
            }
        }
    }
}
