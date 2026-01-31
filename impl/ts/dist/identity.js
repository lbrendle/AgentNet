export function parseIdentityRegisterPayload(value) {
    const entries = expectMap(value);
    const pkEd = expectBytes(getRequired(entries, 1));
    const pkX = expectBytes(getRequired(entries, 2));
    if (pkEd.length !== 32 || pkX.length !== 32) {
        throw new Error("invalid public key length");
    }
    return {
        agentId: expectText(getRequired(entries, 0)),
        pkEd25519: pkEd,
        pkX25519: pkX,
        created: expectU64(getRequired(entries, 3)),
    };
}
export function parseIdentityRotatePayload(value) {
    const entries = expectMap(value);
    const pkEd = expectBytes(getRequired(entries, 1));
    const pkX = expectBytes(getRequired(entries, 2));
    if (pkEd.length !== 32 || pkX.length !== 32) {
        throw new Error("invalid public key length");
    }
    return {
        agentId: expectText(getRequired(entries, 0)),
        pkEd25519: pkEd,
        pkX25519: pkX,
        ts: expectU64(getRequired(entries, 3)),
    };
}
export function parseCredentialRevokePayload(value) {
    const entries = expectMap(value);
    const cred = expectBytes(getRequired(entries, 1));
    if (cred.length !== 32)
        throw new Error("credentialIdHash must be 32 bytes");
    return {
        issuer: expectText(getRequired(entries, 0)),
        credentialIdHash: cred,
        ts: expectU64(getRequired(entries, 2)),
    };
}
export function identityRegisterPayloadToCbor(payload) {
    return {
        entries: [
            [0, payload.agentId],
            [1, payload.pkEd25519],
            [2, payload.pkX25519],
            [3, payload.created],
        ],
    };
}
export function identityRotatePayloadToCbor(payload) {
    return {
        entries: [
            [0, payload.agentId],
            [1, payload.pkEd25519],
            [2, payload.pkX25519],
            [3, payload.ts],
        ],
    };
}
export function credentialRevokePayloadToCbor(payload) {
    return {
        entries: [
            [0, payload.issuer],
            [1, payload.credentialIdHash],
            [2, payload.ts],
        ],
    };
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
function expectBytes(value) {
    if (value instanceof Uint8Array)
        return value;
    throw new Error("expected bytes");
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
