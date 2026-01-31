import { decodeCanonical, encodeCanonical } from "./cbor.js";
import { sha256, signEd25519Hash, verifyEd25519Hash } from "./crypto.js";
export function parseTxEnvelopePayload(value) {
    const entries = expectMap(value);
    return {
        txType: expectU64(getRequired(entries, 0)),
        sender: expectText(getRequired(entries, 1)),
        nonce: expectU64(getRequired(entries, 2)),
        fee: expectU64(getRequired(entries, 3)),
        payload: getRequired(entries, 4),
    };
}
export function parseTxEnvelope(value) {
    const [payload, signature] = splitSignedMap(value, 5);
    return { payload: parseTxEnvelopePayload(payload), signature };
}
export function decodeTxEnvelope(data) {
    return parseTxEnvelope(decodeCanonical(data));
}
export function buildTxEnvelope(payload, secretKey) {
    const payloadMap = txEnvelopePayloadToCbor(payload);
    const payloadCbor = encodeCanonical(payloadMap);
    const digest = sha256(payloadCbor);
    const signature = signEd25519Hash(secretKey, digest);
    const full = withSignature(payloadMap, 5, signature);
    return encodeCanonical(full);
}
export function verifyTxEnvelope(data, publicKey) {
    const value = decodeCanonical(data);
    const [payload, signature] = splitSignedMap(value, 5);
    const payloadCbor = encodeCanonical(payload);
    const digest = sha256(payloadCbor);
    verifyEd25519Hash(publicKey, digest, signature);
    return parseTxEnvelopePayload(payload);
}
export function txEnvelopePayloadToCbor(payload) {
    return {
        entries: [
            [0, payload.txType],
            [1, payload.sender],
            [2, payload.nonce],
            [3, payload.fee],
            [4, payload.payload],
        ],
    };
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
        if (typeof idx === "number" && idx === key)
            return v;
    }
    throw new Error("missing required key");
}
function expectText(value) {
    if (typeof value === "string")
        return value;
    throw new Error("expected text");
}
function expectU64(value) {
    if (typeof value === "bigint") {
        const asNumber = Number(value);
        if (!Number.isSafeInteger(asNumber))
            throw new Error("expected u64");
        return asNumber;
    }
    if (typeof value === "number" && Number.isSafeInteger(value) && value >= 0)
        return value;
    throw new Error("expected u64");
}
function toNumber(value) {
    if (typeof value === "number")
        return value;
    if (typeof value === "bigint") {
        const asNumber = Number(value);
        if (!Number.isSafeInteger(asNumber))
            return null;
        return asNumber;
    }
    return null;
}
