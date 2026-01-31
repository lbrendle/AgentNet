use crate::markdown::validate_markdown_profile;
use crate::schema::{
    expect_bytes_len, expect_map, expect_text, expect_text_array, expect_u64, expect_u8,
    get_optional, get_required,
};
use crate::signed::{split_signed_map, with_signature};
use crate::{
    decode_canonical, encode_canonical, sha256, sign_ed25519_hash, verify_ed25519_hash, CborValue,
    Error,
};

const AGENTMAIL_SIG_KEY: u64 = 14;
const AGENTMAIL_VERSION: u8 = 1;

#[derive(Debug, Clone)]
pub struct AgentMailAttachment {
    pub content_hash: Vec<u8>,
    pub size_bytes: u64,
    pub mime: String,
    pub retrieval: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct AgentMailMessagePayload {
    pub version: u8,
    pub message_id: String,
    pub sender: String,
    pub recipients: Vec<String>,
    pub thread_id: Option<String>,
    pub reply_to: Option<String>,
    pub subject: Option<String>,
    pub markdown: String,
    pub attachments: Option<Vec<AgentMailAttachment>>,
    pub intent_hashes: Option<Vec<Vec<u8>>>,
    pub receipt_hashes: Option<Vec<Vec<u8>>>,
    pub metadata: Option<CborValue>,
    pub ts: u64,
    pub expires: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct AgentMailMessage {
    pub payload: AgentMailMessagePayload,
    pub signature: Vec<u8>,
}

pub fn parse_agentmail_payload(value: &CborValue) -> Result<AgentMailMessagePayload, Error> {
    let map = expect_map(value)?;
    let version = expect_u8(get_required(&map, 0)?)?;
    let message_id = expect_text(get_required(&map, 1)?)?;
    let sender = expect_text(get_required(&map, 2)?)?;
    let recipients = expect_text_array(get_required(&map, 3)?)?;
    let thread_id = match get_optional(&map, 4) {
        Some(value) => Some(expect_text(value)?),
        None => None,
    };
    let reply_to = match get_optional(&map, 5) {
        Some(value) => Some(expect_text(value)?),
        None => None,
    };
    let subject = match get_optional(&map, 6) {
        Some(value) => Some(expect_text(value)?),
        None => None,
    };
    let markdown = expect_text(get_required(&map, 7)?)?;
    let attachments = match get_optional(&map, 8) {
        Some(value) => Some(parse_attachments(value)?),
        None => None,
    };
    let intent_hashes = match get_optional(&map, 9) {
        Some(value) => Some(expect_hash_array(value)?),
        None => None,
    };
    let receipt_hashes = match get_optional(&map, 10) {
        Some(value) => Some(expect_hash_array(value)?),
        None => None,
    };
    let metadata = get_optional(&map, 11).cloned();
    let ts = expect_u64(get_required(&map, 12)?)?;
    let expires = match get_optional(&map, 13) {
        Some(value) => Some(expect_u64(value)?),
        None => None,
    };

    let payload = AgentMailMessagePayload {
        version,
        message_id,
        sender,
        recipients,
        thread_id,
        reply_to,
        subject,
        markdown,
        attachments,
        intent_hashes,
        receipt_hashes,
        metadata,
        ts,
        expires,
    };
    payload.validate()?;
    Ok(payload)
}

pub fn parse_agentmail_message(value: &CborValue) -> Result<AgentMailMessage, Error> {
    let (payload_entries, signature) = split_signed_map(value, AGENTMAIL_SIG_KEY)?;
    let payload_value = CborValue::Map(payload_entries);
    let payload = parse_agentmail_payload(&payload_value)?;
    Ok(AgentMailMessage { payload, signature })
}

pub fn decode_agentmail_message(data: &[u8]) -> Result<AgentMailMessage, Error> {
    let value = decode_canonical(data)?;
    parse_agentmail_message(&value)
}

pub fn build_agentmail_message(
    payload: &AgentMailMessagePayload,
    secret_key: &[u8],
) -> Result<Vec<u8>, Error> {
    payload.validate()?;
    let payload_value = payload.to_cbor()?;
    let payload_cbor = encode_canonical(&payload_value)?;
    let hash = sha256(&payload_cbor);
    let sig = sign_ed25519_hash(secret_key, &hash)?;
    let full = with_signature(&payload_value, AGENTMAIL_SIG_KEY, sig)?;
    encode_canonical(&full)
}

pub fn verify_agentmail_message(
    data: &[u8],
    public_key: &[u8],
) -> Result<AgentMailMessagePayload, Error> {
    let value = decode_canonical(data)?;
    let (payload_entries, signature) = split_signed_map(&value, AGENTMAIL_SIG_KEY)?;
    let payload_value = CborValue::Map(payload_entries);
    let payload_cbor = encode_canonical(&payload_value)?;
    let hash = sha256(&payload_cbor);
    verify_ed25519_hash(public_key, &hash, &signature)?;
    parse_agentmail_payload(&payload_value)
}

impl AgentMailAttachment {
    pub fn to_cbor(&self) -> Result<CborValue, Error> {
        if self.content_hash.len() != 32 {
            return Err(Error::Cbor("invalid attachment hash length"));
        }
        if self.size_bytes == 0 {
            return Err(Error::Cbor("attachment size required"));
        }
        if self.mime.trim().is_empty() {
            return Err(Error::Cbor("attachment mime required"));
        }
        let mut entries = Vec::new();
        entries.push((
            CborValue::Unsigned(0),
            CborValue::Bytes(self.content_hash.clone()),
        ));
        entries.push((CborValue::Unsigned(1), CborValue::Unsigned(self.size_bytes)));
        entries.push((CborValue::Unsigned(2), CborValue::Text(self.mime.clone())));
        if let Some(addrs) = &self.retrieval {
            if addrs.is_empty() {
                return Err(Error::Cbor("attachment retrieval empty"));
            }
            entries.push((
                CborValue::Unsigned(3),
                CborValue::Array(addrs.iter().map(|s| CborValue::Text(s.clone())).collect()),
            ));
        }
        Ok(CborValue::Map(entries))
    }
}

impl AgentMailMessagePayload {
    pub fn to_cbor(&self) -> Result<CborValue, Error> {
        self.validate()?;
        let mut entries = Vec::new();
        entries.push((
            CborValue::Unsigned(0),
            CborValue::Unsigned(self.version as u64),
        ));
        entries.push((
            CborValue::Unsigned(1),
            CborValue::Text(self.message_id.clone()),
        ));
        entries.push((CborValue::Unsigned(2), CborValue::Text(self.sender.clone())));
        entries.push((
            CborValue::Unsigned(3),
            CborValue::Array(
                self.recipients
                    .iter()
                    .map(|s| CborValue::Text(s.clone()))
                    .collect(),
            ),
        ));
        if let Some(thread_id) = &self.thread_id {
            entries.push((CborValue::Unsigned(4), CborValue::Text(thread_id.clone())));
        }
        if let Some(reply_to) = &self.reply_to {
            entries.push((CborValue::Unsigned(5), CborValue::Text(reply_to.clone())));
        }
        if let Some(subject) = &self.subject {
            entries.push((CborValue::Unsigned(6), CborValue::Text(subject.clone())));
        }
        entries.push((
            CborValue::Unsigned(7),
            CborValue::Text(self.markdown.clone()),
        ));
        if let Some(attachments) = &self.attachments {
            let mut items = Vec::with_capacity(attachments.len());
            for attachment in attachments {
                items.push(attachment.to_cbor()?);
            }
            entries.push((CborValue::Unsigned(8), CborValue::Array(items)));
        }
        if let Some(intent_hashes) = &self.intent_hashes {
            entries.push((
                CborValue::Unsigned(9),
                CborValue::Array(
                    intent_hashes
                        .iter()
                        .map(|h| CborValue::Bytes(h.clone()))
                        .collect(),
                ),
            ));
        }
        if let Some(receipt_hashes) = &self.receipt_hashes {
            entries.push((
                CborValue::Unsigned(10),
                CborValue::Array(
                    receipt_hashes
                        .iter()
                        .map(|h| CborValue::Bytes(h.clone()))
                        .collect(),
                ),
            ));
        }
        if let Some(metadata) = &self.metadata {
            entries.push((CborValue::Unsigned(11), metadata.clone()));
        }
        entries.push((CborValue::Unsigned(12), CborValue::Unsigned(self.ts)));
        if let Some(expires) = self.expires {
            entries.push((CborValue::Unsigned(13), CborValue::Unsigned(expires)));
        }
        Ok(CborValue::Map(entries))
    }

    pub fn validate(&self) -> Result<(), Error> {
        if self.version != AGENTMAIL_VERSION {
            return Err(Error::Cbor("unsupported agentmail version"));
        }
        if self.message_id.trim().is_empty() {
            return Err(Error::Cbor("message_id required"));
        }
        if self.sender.trim().is_empty() {
            return Err(Error::Cbor("sender required"));
        }
        if self.recipients.is_empty() {
            return Err(Error::Cbor("recipients required"));
        }
        for recipient in &self.recipients {
            if recipient.trim().is_empty() {
                return Err(Error::Cbor("recipient required"));
            }
        }
        if let Some(thread_id) = &self.thread_id {
            if thread_id.trim().is_empty() {
                return Err(Error::Cbor("thread_id required"));
            }
        }
        if let Some(reply_to) = &self.reply_to {
            if reply_to.trim().is_empty() {
                return Err(Error::Cbor("reply_to required"));
            }
        }
        if let Some(subject) = &self.subject {
            if subject.trim().is_empty() {
                return Err(Error::Cbor("subject required"));
            }
        }
        if self.markdown.trim().is_empty() {
            return Err(Error::Cbor("markdown required"));
        }
        validate_markdown_profile(&self.markdown)?;
        if self.ts == 0 {
            return Err(Error::Cbor("timestamp required"));
        }
        if let Some(expires) = self.expires {
            if expires < self.ts {
                return Err(Error::Cbor("expires before timestamp"));
            }
        }
        if let Some(attachments) = &self.attachments {
            if attachments.is_empty() {
                return Err(Error::Cbor("attachments empty"));
            }
            for attachment in attachments {
                if attachment.content_hash.len() != 32 {
                    return Err(Error::Cbor("attachment hash length invalid"));
                }
                if attachment.size_bytes == 0 {
                    return Err(Error::Cbor("attachment size required"));
                }
                if attachment.mime.trim().is_empty() {
                    return Err(Error::Cbor("attachment mime required"));
                }
                if let Some(addrs) = &attachment.retrieval {
                    if addrs.is_empty() {
                        return Err(Error::Cbor("attachment retrieval empty"));
                    }
                    for addr in addrs {
                        if addr.trim().is_empty() {
                            return Err(Error::Cbor("attachment retrieval invalid"));
                        }
                    }
                }
            }
        }
        if let Some(hashes) = &self.intent_hashes {
            if hashes.is_empty() {
                return Err(Error::Cbor("intent hashes empty"));
            }
            for hash in hashes {
                if hash.len() != 32 {
                    return Err(Error::Cbor("intent hash length invalid"));
                }
            }
        }
        if let Some(hashes) = &self.receipt_hashes {
            if hashes.is_empty() {
                return Err(Error::Cbor("receipt hashes empty"));
            }
            for hash in hashes {
                if hash.len() != 32 {
                    return Err(Error::Cbor("receipt hash length invalid"));
                }
            }
        }
        Ok(())
    }
}

fn parse_attachments(value: &CborValue) -> Result<Vec<AgentMailAttachment>, Error> {
    match value {
        CborValue::Array(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                out.push(parse_attachment(item)?);
            }
            Ok(out)
        }
        _ => Err(Error::Cbor("expected attachment array")),
    }
}

fn parse_attachment(value: &CborValue) -> Result<AgentMailAttachment, Error> {
    let map = expect_map(value)?;
    let content_hash = expect_bytes_len(get_required(&map, 0)?, 32)?;
    let size_bytes = expect_u64(get_required(&map, 1)?)?;
    let mime = expect_text(get_required(&map, 2)?)?;
    let retrieval = match get_optional(&map, 3) {
        Some(value) => Some(expect_text_array(value)?),
        None => None,
    };
    let attachment = AgentMailAttachment {
        content_hash,
        size_bytes,
        mime,
        retrieval,
    };
    if attachment.size_bytes == 0 {
        return Err(Error::Cbor("attachment size required"));
    }
    if attachment.mime.trim().is_empty() {
        return Err(Error::Cbor("attachment mime required"));
    }
    if let Some(addrs) = &attachment.retrieval {
        if addrs.is_empty() {
            return Err(Error::Cbor("attachment retrieval empty"));
        }
        for addr in addrs {
            if addr.trim().is_empty() {
                return Err(Error::Cbor("attachment retrieval invalid"));
            }
        }
    }
    Ok(attachment)
}

fn expect_hash_array(value: &CborValue) -> Result<Vec<Vec<u8>>, Error> {
    match value {
        CborValue::Array(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                out.push(expect_bytes_len(item, 32)?);
            }
            Ok(out)
        }
        _ => Err(Error::Cbor("expected hash array")),
    }
}
