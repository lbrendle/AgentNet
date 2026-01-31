use crate::config::{Config, HandshakeConfig, PubSubConfig};
use crate::keys::KeyMaterial;
use anyhow::{Context, Result};
use anetsdk::{
    build_nodehello, verify_nodehello, verify_pubsub_envelope, CborValue, NodeHelloPayload,
    PubSubEnvelopePayload,
};
use async_trait::async_trait;
use ::futures::prelude::*;
use libp2p::gossipsub::{self, IdentTopic, MessageAuthenticity, ValidationMode};
use libp2p::identify;
use libp2p::kad::{self, store::MemoryStore};
use libp2p::request_response::{self};
use libp2p::swarm::{NetworkBehaviour, StreamProtocol, Swarm, SwarmEvent};
use libp2p::{identity, Multiaddr, PeerId};
use libp2p::core::transport::Transport;
use libp2p::core::muxing::StreamMuxerBox;
use rand::rngs::OsRng;
use rand::RngCore;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::time::Duration;
use tracing::{info, warn};

const HANDSHAKE_PROTOCOL: &str = "agentnet/handshake/1.0.0";
const HANDSHAKE_STREAM_PROTOCOL: &str = "/agentnet/handshake/1.0.0";
const MAX_HANDSHAKE_MSG_BYTES: usize = 1024 * 1024;

#[derive(Clone, Default)]
struct NodeHelloCodec();

#[async_trait]
impl request_response::Codec for NodeHelloCodec {
    type Protocol = StreamProtocol;
    type Request = Vec<u8>;
    type Response = Vec<u8>;

    async fn read_request<T>(&mut self, _: &Self::Protocol, io: &mut T) -> std::io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut buf = Vec::new();
        io.take(MAX_HANDSHAKE_MSG_BYTES as u64).read_to_end(&mut buf).await?;
        Ok(buf)
    }

    async fn read_response<T>(&mut self, _: &Self::Protocol, io: &mut T) -> std::io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut buf = Vec::new();
        io.take(MAX_HANDSHAKE_MSG_BYTES as u64).read_to_end(&mut buf).await?;
        Ok(buf)
    }

    async fn write_request<T>(&mut self, _: &Self::Protocol, io: &mut T, data: Self::Request) -> std::io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        io.write_all(data.as_ref()).await?;
        Ok(())
    }

    async fn write_response<T>(&mut self, _: &Self::Protocol, io: &mut T, data: Self::Response) -> std::io::Result<()>
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
    agent_did: String,
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
        build_nodehello(&payload, &self.secret_key)
            .context("build nodehello")
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
        gossipsub_config
            .build()
            .context("build gossipsub config")?,
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

    let identify = identify::Behaviour::new(identify::Config::new(HANDSHAKE_STREAM_PROTOCOL.to_string(), libp2p_keypair.public()));

    let behaviour = MeshBehaviour {
        gossipsub,
        kademlia,
        request_response,
        identify,
    };

    let quic_config = libp2p::quic::Config::new(&libp2p_keypair);
    let transport = libp2p::quic::tokio::Transport::new(quic_config)
        .map(|(peer_id, conn), _| (peer_id, StreamMuxerBox::new(conn)))
        .boxed();
    let mut swarm = Swarm::new(
        transport,
        behaviour,
        peer_id,
        libp2p::swarm::Config::with_tokio_executor(),
    );

    for addr in &config.listen_addrs {
        let multiaddr: Multiaddr = addr.parse().with_context(|| format!("invalid listen addr {addr}"))?;
        swarm.listen_on(multiaddr).with_context(|| format!("listen on {addr}"))?;
    }

    for addr in &config.bootstrap {
        let multiaddr: Multiaddr = addr.parse().with_context(|| format!("invalid bootstrap addr {addr}"))?;
        swarm.dial(multiaddr).with_context(|| format!("dial {addr}"))?;
    }

    let node_id = config.node_id.clone().unwrap_or_else(|| peer_id.to_string());
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

    Ok(MeshNode {
        swarm,
        nodehello_template,
        handshake: config.handshake,
        pubsub: config.pubsub,
        agent_did: config.agent_did.clone(),
        peer_hellos: HashMap::new(),
    })
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

        info!("mesh node running (agent_did={})", self.agent_did);

        loop {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    info!("shutdown requested");
                    break;
                }
                event = self.swarm.select_next_some() => {
                    self.handle_event(event).await?;
                }
            }
        }

        Ok(())
    }

    async fn handle_event(&mut self, event: SwarmEvent<MeshBehaviourEvent>) -> Result<()> {
        match event {
            SwarmEvent::Behaviour(event) => {
                self.handle_behaviour_event(event).await?;
            }
            SwarmEvent::NewListenAddr { address, .. } => {
                info!("listening on {address}");
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
                if let gossipsub::Event::Message { propagation_source, message_id, message } = event {
                    self.handle_gossipsub_message(propagation_source, message_id, message).await?;
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

    fn handle_request_response(&mut self, event: request_response::Event<Vec<u8>, Vec<u8>>) -> Result<()> {
        match event {
            request_response::Event::Message { peer, message } => match message {
                request_response::Message::Request { request, channel, .. } => {
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
            return Ok(());
        }

        if self.pubsub.verify_signatures() {
            let peer = self.peer_hellos.get(&propagation_source).expect("checked");
            match verify_pubsub_envelope(&message.data, &peer.node_pubkey) {
                Ok(payload) => {
                    if self.pubsub.require_economic_proof() && payload.economic_proof.is_none() {
                        warn!("dropping pubsub msg {message_id}: missing economic proof");
                        return Ok(());
                    }
                    self.log_pubsub_payload(&payload);
                }
                Err(err) => {
                    warn!("invalid pubsub envelope from {propagation_source}: {err:?}");
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
            payload.topic,
            payload.sender,
            payload.payload_type,
            payload.seq,
            payload.ts
        );
    }
}

fn unix_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, FeaturesConfig, HandshakeConfig, PubSubConfig};
    use crate::keys::generate_keypair;
    use base64::engine::general_purpose::STANDARD as BASE64;
    use base64::Engine;
    use std::path::PathBuf;
    use tokio::time::{Instant, Duration};

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn handshake_completes_between_two_nodes() -> Result<()> {
        let keys_a = generate_keypair();
        let keys_b = generate_keypair();
        let chain_id = BASE64.encode(keys_a.verifying_key.to_bytes());
        let agent_did_a = format!("did:anet:agent:{}", BASE64.encode(keys_a.verifying_key.to_bytes()));
        let agent_did_b = format!("did:anet:agent:{}", BASE64.encode(keys_b.verifying_key.to_bytes()));

        let config_a = Config {
            chain_id: chain_id.clone(),
            agent_did: agent_did_a,
            key_path: PathBuf::new(),
            node_id: None,
            listen_addrs: vec!["/ip4/127.0.0.1/udp/0/quic-v1".to_string()],
            bootstrap: vec![],
            protocols: vec![],
            roles: vec![],
            features: FeaturesConfig::default(),
            pubsub: PubSubConfig::default(),
            handshake: HandshakeConfig::default(),
        };
        let config_b = Config {
            chain_id,
            agent_did: agent_did_b,
            key_path: PathBuf::new(),
            node_id: None,
            listen_addrs: vec!["/ip4/127.0.0.1/udp/0/quic-v1".to_string()],
            bootstrap: vec![],
            protocols: vec![],
            roles: vec![],
            features: FeaturesConfig::default(),
            pubsub: PubSubConfig::default(),
            handshake: HandshakeConfig::default(),
        };

        let mut node_a = build_mesh(config_a, keys_a)?;
        let mut node_b = build_mesh(config_b, keys_b)?;

        wait_for_listen(&mut node_a).await?;
        let addr_b = wait_for_listen(&mut node_b).await?;

        let b_peer = *node_b.swarm.local_peer_id();
        let dial_addr = addr_b.with_p2p(b_peer).map_err(|_| anyhow::anyhow!("invalid dial addr"))?;
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
            if node_a.peer_hellos.contains_key(&b_peer) && node_b.peer_hellos.contains_key(&a_peer) {
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
