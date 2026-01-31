import { decodeCanonical, encodeCanonical } from "./cbor.js";
import { sha256, signEd25519Hash, verifyEd25519Hash } from "./crypto.js";
const SKILL_SIG_KEY = 16;
const SANDBOX_MIN = 1;
const SANDBOX_MAX = 5;
export function parseSkillManifestPayload(value) {
    const entries = expectMap(value);
    const payload = {
        skillId: expectText(getRequired(entries, 0)),
        author: expectText(getRequired(entries, 1)),
        name: expectText(getRequired(entries, 2)),
        version: expectText(getRequired(entries, 3)),
        summary: expectText(getRequired(entries, 4)),
        license: expectText(getRequired(entries, 5)),
        capabilities: expectTextArray(getRequired(entries, 6)),
        permissions: expectTextArray(getRequired(entries, 7)),
        sandboxClass: expectU16(getRequired(entries, 8)),
        endpoints: getOptional(entries, 9) ? expectTextArray(getRequired(entries, 9)) : undefined,
        artifacts: getOptional(entries, 10) ? parseArtifacts(getRequired(entries, 10)) : undefined,
        requirements: getOptional(entries, 11) ? expectTextArray(getRequired(entries, 11)) : undefined,
        pricing: getOptional(entries, 12) ?? undefined,
        attestations: getOptional(entries, 13) ?? undefined,
        metadata: getOptional(entries, 14) ?? undefined,
        ts: expectU64(getRequired(entries, 15)),
    };
    validatePayload(payload);
    return payload;
}
export function parseSkillManifest(value) {
    const [payload, signature] = splitSignedMap(value, SKILL_SIG_KEY);
    return { payload: parseSkillManifestPayload(payload), signature };
}
export function decodeSkillManifest(data) {
    return parseSkillManifest(decodeCanonical(data));
}
export function buildSkillManifest(payload, secretKey) {
    validatePayload(payload);
    const payloadMap = payloadToCbor(payload);
    const payloadBytes = encodeCanonical(payloadMap);
    const digest = sha256(payloadBytes);
    const signature = signEd25519Hash(secretKey, digest);
    const full = withSignature(payloadMap, SKILL_SIG_KEY, signature);
    return encodeCanonical(full);
}
export function verifySkillManifest(data, publicKey) {
    const value = decodeCanonical(data);
    const [payload, signature] = splitSignedMap(value, SKILL_SIG_KEY);
    const payloadBytes = encodeCanonical(payload);
    const digest = sha256(payloadBytes);
    verifyEd25519Hash(publicKey, digest, signature);
    return parseSkillManifestPayload(payload);
}
export function parseSkillPublishPayload(value) {
    const entries = expectMap(value);
    const manifest = expectBytes(getRequired(entries, 0));
    const ts = expectU64(getRequired(entries, 1));
    if (ts === 0)
        throw new Error("timestamp required");
    decodeSkillManifest(manifest);
    return { manifest, ts };
}
export function parseSkillUpdatePayload(value) {
    const entries = expectMap(value);
    const skillId = expectText(getRequired(entries, 0));
    const prevManifestHash = expectBytesLen(getRequired(entries, 1), 32);
    const manifest = expectBytes(getRequired(entries, 2));
    const ts = expectU64(getRequired(entries, 3));
    if (ts === 0)
        throw new Error("timestamp required");
    decodeSkillManifest(manifest);
    return { skillId, prevManifestHash, manifest, ts };
}
export function parseSkillRevokePayload(value) {
    const entries = expectMap(value);
    const skillId = expectText(getRequired(entries, 0));
    const manifestHash = expectBytesLen(getRequired(entries, 1), 32);
    const reason = expectText(getRequired(entries, 2));
    const ts = expectU64(getRequired(entries, 3));
    if (ts === 0)
        throw new Error("timestamp required");
    if (!reason.trim())
        throw new Error("reason required");
    return { skillId, manifestHash, reason, ts };
}
export function skillPublishPayloadToCbor(payload) {
    if (payload.ts === 0)
        throw new Error("timestamp required");
    decodeSkillManifest(payload.manifest);
    return { entries: [[0, payload.manifest], [1, payload.ts]] };
}
export function skillUpdatePayloadToCbor(payload) {
    if (payload.ts === 0)
        throw new Error("timestamp required");
    ensureNonempty(payload.skillId, "skill id required");
    if (payload.prevManifestHash.length !== 32)
        throw new Error("invalid manifest hash length");
    decodeSkillManifest(payload.manifest);
    return {
        entries: [
            [0, payload.skillId],
            [1, payload.prevManifestHash],
            [2, payload.manifest],
            [3, payload.ts],
        ],
    };
}
export function skillRevokePayloadToCbor(payload) {
    if (payload.ts === 0)
        throw new Error("timestamp required");
    ensureNonempty(payload.skillId, "skill id required");
    ensureNonempty(payload.reason, "reason required");
    if (payload.manifestHash.length !== 32)
        throw new Error("invalid manifest hash length");
    return {
        entries: [
            [0, payload.skillId],
            [1, payload.manifestHash],
            [2, payload.reason],
            [3, payload.ts],
        ],
    };
}
function payloadToCbor(payload) {
    validatePayload(payload);
    const entries = [
        [0, payload.skillId],
        [1, payload.author],
        [2, payload.name],
        [3, payload.version],
        [4, payload.summary],
        [5, payload.license],
        [6, [...payload.capabilities]],
        [7, [...payload.permissions]],
        [8, payload.sandboxClass],
        [15, payload.ts],
    ];
    if (payload.endpoints) {
        entries.push([9, [...payload.endpoints]]);
    }
    if (payload.artifacts) {
        entries.push([10, payload.artifacts.map(artifactToCbor)]);
    }
    if (payload.requirements) {
        entries.push([11, [...payload.requirements]]);
    }
    if (payload.pricing) {
        entries.push([12, payload.pricing]);
    }
    if (payload.attestations) {
        entries.push([13, payload.attestations]);
    }
    if (payload.metadata) {
        entries.push([14, payload.metadata]);
    }
    return { entries };
}
function artifactToCbor(artifact) {
    validateArtifact(artifact);
    return { entries: [[0, artifact.kind], [1, artifact.digest], [2, artifact.size], [3, [...artifact.uris]]] };
}
function parseArtifacts(value) {
    if (!Array.isArray(value) || value.length === 0) {
        throw new Error("artifacts required");
    }
    return value.map(parseArtifact);
}
function parseArtifact(value) {
    const entries = expectMap(value);
    const artifact = {
        kind: expectU8(getRequired(entries, 0)),
        digest: expectBytesLen(getRequired(entries, 1), 32),
        size: expectU64(getRequired(entries, 2)),
        uris: expectTextArray(getRequired(entries, 3)),
    };
    validateArtifact(artifact);
    return artifact;
}
function validatePayload(payload) {
    ensureNonempty(payload.skillId, "skill id required");
    ensureNonempty(payload.author, "author required");
    ensureNonempty(payload.name, "name required");
    ensureNonempty(payload.version, "version required");
    ensureNonempty(payload.summary, "summary required");
    ensureNonempty(payload.license, "license required");
    ensureListNonempty(payload.capabilities, "capabilities required");
    ensureListItems(payload.permissions, "permissions required");
    if (payload.sandboxClass < SANDBOX_MIN || payload.sandboxClass > SANDBOX_MAX) {
        throw new Error("invalid sandbox class");
    }
    if (payload.endpoints) {
        ensureListNonempty(payload.endpoints, "endpoints required");
    }
    if (payload.artifacts) {
        if (payload.artifacts.length === 0)
            throw new Error("artifacts required");
        payload.artifacts.forEach(validateArtifact);
    }
    if (payload.requirements) {
        ensureListItems(payload.requirements, "requirements required");
    }
    if (!payload.endpoints && !payload.artifacts) {
        throw new Error("skill requires endpoints or artifacts");
    }
}
function validateArtifact(artifact) {
    if (artifact.kind <= 0)
        throw new Error("artifact kind required");
    if (artifact.digest.length !== 32)
        throw new Error("artifact digest must be 32 bytes");
    if (artifact.size <= 0)
        throw new Error("artifact size required");
    ensureListNonempty(artifact.uris, "artifact uris required");
}
function ensureNonempty(value, message) {
    if (!value || value.trim().length === 0) {
        throw new Error(message);
    }
}
function ensureListNonempty(values, message) {
    if (!values || values.length === 0) {
        throw new Error(message);
    }
    ensureListItems(values, message);
}
function ensureListItems(values, message) {
    for (const item of values) {
        if (!item || item.trim().length === 0) {
            throw new Error(message);
        }
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
function expectBytesLen(value, length) {
    if (!(value instanceof Uint8Array))
        throw new Error("expected bytes");
    if (value.length !== length)
        throw new Error("invalid length");
    return value;
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
