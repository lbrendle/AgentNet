import { decodeCanonical, encodeCanonical } from "./cbor.js";
import { sha256, signEd25519Hash, verifyEd25519Hash } from "./crypto.js";
import { validateMarkdownProfile } from "./markdown.js";
const AGENTMAIL_SIG_KEY = 14;
const AGENTMAIL_VERSION = 1;
export function parseAgentMailPayload(value) {
    const entries = expectMap(value);
    const payload = {
        version: expectU8(getRequired(entries, 0)),
        messageId: expectText(getRequired(entries, 1)),
        sender: expectText(getRequired(entries, 2)),
        recipients: expectTextArray(getRequired(entries, 3)),
        threadId: getOptional(entries, 4) ? expectText(getRequired(entries, 4)) : undefined,
        replyTo: getOptional(entries, 5) ? expectText(getRequired(entries, 5)) : undefined,
        subject: getOptional(entries, 6) ? expectText(getRequired(entries, 6)) : undefined,
        markdown: expectText(getRequired(entries, 7)),
        attachments: getOptional(entries, 8) ? parseAttachments(getRequired(entries, 8)) : undefined,
        intentHashes: getOptional(entries, 9) ? expectHashArray(getRequired(entries, 9)) : undefined,
        receiptHashes: getOptional(entries, 10) ? expectHashArray(getRequired(entries, 10)) : undefined,
        metadata: getOptional(entries, 11) ?? undefined,
        ts: expectU64(getRequired(entries, 12)),
        expires: getOptional(entries, 13) ? expectU64(getRequired(entries, 13)) : undefined,
    };
    validatePayload(payload);
    return payload;
}
export function parseAgentMailMessage(value) {
    const [payload, signature] = splitSignedMap(value, AGENTMAIL_SIG_KEY);
    return { payload: parseAgentMailPayload(payload), signature };
}
export function decodeAgentMailMessage(data) {
    return parseAgentMailMessage(decodeCanonical(data));
}
export function buildAgentMailMessage(payload, secretKey) {
    validatePayload(payload);
    const payloadMap = agentmailPayloadToCbor(payload);
    const payloadBytes = encodeCanonical(payloadMap);
    const digest = sha256(payloadBytes);
    const signature = signEd25519Hash(secretKey, digest);
    const full = withSignature(payloadMap, AGENTMAIL_SIG_KEY, signature);
    return encodeCanonical(full);
}
export function verifyAgentMailMessage(data, publicKey) {
    const value = decodeCanonical(data);
    const [payload, signature] = splitSignedMap(value, AGENTMAIL_SIG_KEY);
    const payloadBytes = encodeCanonical(payload);
    const digest = sha256(payloadBytes);
    verifyEd25519Hash(publicKey, digest, signature);
    return parseAgentMailPayload(payload);
}
function agentmailPayloadToCbor(payload) {
    validatePayload(payload);
    const entries = [];
    entries.push([0, payload.version]);
    entries.push([1, payload.messageId]);
    entries.push([2, payload.sender]);
    entries.push([3, payload.recipients.slice()]);
    if (payload.threadId)
        entries.push([4, payload.threadId]);
    if (payload.replyTo)
        entries.push([5, payload.replyTo]);
    if (payload.subject)
        entries.push([6, payload.subject]);
    entries.push([7, payload.markdown]);
    if (payload.attachments)
        entries.push([8, payload.attachments.map(attachmentToCbor)]);
    if (payload.intentHashes)
        entries.push([9, payload.intentHashes.map((h) => new Uint8Array(h))]);
    if (payload.receiptHashes)
        entries.push([10, payload.receiptHashes.map((h) => new Uint8Array(h))]);
    if (payload.metadata !== undefined)
        entries.push([11, payload.metadata]);
    entries.push([12, payload.ts]);
    if (payload.expires !== undefined)
        entries.push([13, payload.expires]);
    return { entries };
}
function attachmentToCbor(attachment) {
    validateAttachment(attachment);
    const entries = [];
    entries.push([0, new Uint8Array(attachment.contentHash)]);
    entries.push([1, attachment.sizeBytes]);
    entries.push([2, attachment.mime]);
    if (attachment.retrieval)
        entries.push([3, attachment.retrieval.slice()]);
    return { entries };
}
function parseAttachments(value) {
    if (!Array.isArray(value))
        throw new Error("expected attachment array");
    if (value.length === 0)
        throw new Error("attachments empty");
    return value.map(parseAttachment);
}
function parseAttachment(value) {
    const entries = expectMap(value);
    const attachment = {
        contentHash: expectBytesLen(getRequired(entries, 0), 32),
        sizeBytes: expectU64(getRequired(entries, 1)),
        mime: expectText(getRequired(entries, 2)),
        retrieval: getOptional(entries, 3) ? expectTextArray(getRequired(entries, 3)) : undefined,
    };
    validateAttachment(attachment);
    return attachment;
}
function expectHashArray(value) {
    if (!Array.isArray(value))
        throw new Error("expected hash array");
    if (value.length === 0)
        throw new Error("hash array empty");
    return value.map((item) => expectBytesLen(item, 32));
}
function validatePayload(payload) {
    if (payload.version !== AGENTMAIL_VERSION)
        throw new Error("unsupported agentmail version");
    ensureNonempty(payload.messageId, "message_id required");
    ensureNonempty(payload.sender, "sender required");
    ensureListNonempty(payload.recipients, "recipients required");
    if (payload.threadId)
        ensureNonempty(payload.threadId, "thread_id required");
    if (payload.replyTo)
        ensureNonempty(payload.replyTo, "reply_to required");
    if (payload.subject)
        ensureNonempty(payload.subject, "subject required");
    ensureNonempty(payload.markdown, "markdown required");
    validateMarkdownProfile(payload.markdown);
    ensurePositive(payload.ts, "timestamp required");
    if (payload.expires !== undefined && payload.expires < payload.ts) {
        throw new Error("expires before timestamp");
    }
    if (payload.attachments) {
        if (payload.attachments.length === 0)
            throw new Error("attachments empty");
        payload.attachments.forEach(validateAttachment);
    }
    if (payload.intentHashes) {
        if (payload.intentHashes.length === 0)
            throw new Error("intent hashes empty");
        payload.intentHashes.forEach((hash) => {
            if (hash.length !== 32)
                throw new Error("intent hash length invalid");
        });
    }
    if (payload.receiptHashes) {
        if (payload.receiptHashes.length === 0)
            throw new Error("receipt hashes empty");
        payload.receiptHashes.forEach((hash) => {
            if (hash.length !== 32)
                throw new Error("receipt hash length invalid");
        });
    }
}
function validateAttachment(attachment) {
    if (attachment.contentHash.length !== 32)
        throw new Error("attachment hash length invalid");
    ensurePositive(attachment.sizeBytes, "attachment size required");
    ensureNonempty(attachment.mime, "attachment mime required");
    if (attachment.retrieval) {
        ensureListNonempty(attachment.retrieval, "attachment retrieval invalid");
    }
}
function splitSignedMap(value, sigKey) {
    const entries = expectMap(value);
    const payloadEntries = [];
    let signature = null;
    for (const [k, v] of entries) {
        const key = toNumber(k);
        if (typeof key === "number" && key === sigKey) {
            if (signature)
                throw new Error("duplicate signature key");
            if (!(v instanceof Uint8Array))
                throw new Error("signature must be bytes");
            signature = v;
            continue;
        }
        payloadEntries.push([k, v]);
    }
    if (!signature)
        throw new Error("missing signature");
    if (signature.length !== 64)
        throw new Error("invalid signature length");
    return [{ entries: payloadEntries }, signature];
}
function withSignature(payload, sigKey, signature) {
    const entries = expectMap(payload);
    return { entries: [...entries, [sigKey, signature]] };
}
function expectMap(value) {
    if (typeof value === "object" && value !== null && "entries" in value) {
        return value.entries;
    }
    throw new Error("expected map");
}
function getRequired(entries, key) {
    for (const [k, v] of entries) {
        const idx = toNumber(k);
        if (typeof idx === "number" && idx === key) {
            return v;
        }
    }
    throw new Error("missing required key");
}
function getOptional(entries, key) {
    for (const [k, v] of entries) {
        const idx = toNumber(k);
        if (typeof idx === "number" && idx === key) {
            return v;
        }
    }
    return null;
}
function expectText(value) {
    if (typeof value === "string")
        return value;
    throw new Error("expected text");
}
function expectTextArray(value) {
    if (Array.isArray(value))
        return value.map(expectText);
    throw new Error("expected array of text");
}
function expectU64(value) {
    const num = toNumber(value);
    if (typeof num !== "number" || num < 0 || !Number.isSafeInteger(num)) {
        throw new Error("expected unsigned");
    }
    return num;
}
function expectU8(value) {
    const num = expectU64(value);
    if (num > 255)
        throw new Error("expected u8");
    return num;
}
function expectBytesLen(value, length) {
    if (!(value instanceof Uint8Array))
        throw new Error("expected bytes");
    if (value.length !== length)
        throw new Error("invalid length");
    return value;
}
function ensureNonempty(value, message) {
    if (!value || value.trim().length === 0)
        throw new Error(message);
}
function ensureListNonempty(values, message) {
    if (!values || values.length === 0)
        throw new Error(message);
    ensureListItems(values, message);
}
function ensureListItems(values, message) {
    for (const item of values) {
        if (!item || item.trim().length === 0)
            throw new Error(message);
    }
}
function ensurePositive(value, message) {
    if (!Number.isFinite(value) || value <= 0)
        throw new Error(message);
}
function toNumber(value) {
    if (typeof value === "number")
        return value;
    if (typeof value === "bigint") {
        const num = Number(value);
        if (!Number.isSafeInteger(num))
            throw new Error("integer overflow");
        return num;
    }
    return null;
}
