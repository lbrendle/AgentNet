import { decodeCanonical, encodeCanonical } from "./cbor.js";
import { sha256, signEd25519Hash, verifyEd25519Hash } from "./crypto.js";
export function economicProofOnChain(txHash) {
    return { kind: 1, data: txHash };
}
export function economicProofVoucher(voucher) {
    return { kind: 2, data: voucher };
}
export function parsePubSubPayload(value) {
    const entries = expectMap(value);
    const economicProofValue = getOptional(entries, 7);
    return {
        version: expectU8(getRequired(entries, 0)),
        topic: expectText(getRequired(entries, 1)),
        sender: expectText(getRequired(entries, 2)),
        ts: expectU64(getRequired(entries, 3)),
        seq: expectU64(getRequired(entries, 4)),
        payloadType: expectU16(getRequired(entries, 5)),
        payload: getRequired(entries, 6),
        economicProof: economicProofValue ? parseEconomicProof(economicProofValue) : undefined,
    };
}
export function parsePubSubEnvelope(value) {
    const [payload, signature] = splitSignedMap(value, 8);
    return { payload: parsePubSubPayload(payload), signature };
}
export function decodePubSubEnvelope(data) {
    return parsePubSubEnvelope(decodeCanonical(data));
}
export function buildPubSubEnvelope(payload, secretKey) {
    const payloadMap = payloadToCbor(payload);
    const payloadCbor = encodeCanonical(payloadMap);
    const digest = sha256(payloadCbor);
    const signature = signEd25519Hash(secretKey, digest);
    const full = withSignature(payloadMap, 8, signature);
    return encodeCanonical(full);
}
export function verifyPubSubEnvelope(data, publicKey) {
    const value = decodeCanonical(data);
    const [payload, signature] = splitSignedMap(value, 8);
    const payloadCbor = encodeCanonical(payload);
    const digest = sha256(payloadCbor);
    verifyEd25519Hash(publicKey, digest, signature);
    return parsePubSubPayload(payload);
}
function payloadToCbor(payload) {
    const entries = [
        [0, payload.version],
        [1, payload.topic],
        [2, payload.sender],
        [3, payload.ts],
        [4, payload.seq],
        [5, payload.payloadType],
        [6, payload.payload],
    ];
    if (payload.economicProof) {
        entries.push([7, economicProofToCbor(payload.economicProof)]);
    }
    return { entries };
}
function economicProofToCbor(proof) {
    return { entries: [[0, proof.kind], [1, proof.data]] };
}
function parseEconomicProof(value) {
    const entries = expectMap(value);
    const kind = expectU8(getRequired(entries, 0));
    const data = expectBytes(getRequired(entries, 1));
    if (kind === 1 && data.length !== 32) {
        throw new Error("invalid onchain tx hash length");
    }
    if (kind !== 1 && kind !== 2) {
        throw new Error("unsupported economic proof");
    }
    return { kind, data };
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
function expectBytes(value) {
    if (!(value instanceof Uint8Array))
        throw new Error("expected bytes");
    return value;
}
function expectU64(value) {
    const num = toNumber(value);
    if (typeof num !== "number" || num < 0 || !Number.isSafeInteger(num)) {
        throw new Error("expected unsigned");
    }
    return num;
}
function expectU16(value) {
    const num = toNumber(value);
    if (typeof num !== "number" || num < 0 || num > 0xffff) {
        throw new Error("expected u16");
    }
    return num;
}
function expectU8(value) {
    const num = toNumber(value);
    if (typeof num !== "number" || num < 0 || num > 0xff) {
        throw new Error("expected u8");
    }
    return num;
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
