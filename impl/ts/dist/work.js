import { decodeCanonical, encodeCanonical } from "./cbor.js";
import { sha256, signEd25519Hash, verifyEd25519Hash } from "./crypto.js";
const WORK_SIG_KEY = 16;
export function parseWorkOfferPayload(value) {
    const entries = expectMap(value);
    const payload = {
        offerId: expectText(getRequired(entries, 0)),
        issuer: expectText(getRequired(entries, 1)),
        title: expectText(getRequired(entries, 2)),
        summary: expectText(getRequired(entries, 3)),
        scope: expectText(getRequired(entries, 4)),
        budgetAmount: expectU64(getRequired(entries, 5)),
        budgetCurrency: expectText(getRequired(entries, 6)),
        durationSec: expectU64(getRequired(entries, 7)),
        deliverables: expectTextArray(getRequired(entries, 8)),
        requirements: getOptional(entries, 9) ? expectTextArray(getRequired(entries, 9)) : undefined,
        ts: expectU64(getRequired(entries, 10)),
        exp: expectU64(getRequired(entries, 11)),
    };
    validateOffer(payload);
    return payload;
}
export function parseWorkOffer(value) {
    const [payload, signature] = splitSignedMap(value, WORK_SIG_KEY);
    return { payload: parseWorkOfferPayload(payload), signature };
}
export function decodeWorkOffer(data) {
    return parseWorkOffer(decodeCanonical(data));
}
export function buildWorkOffer(payload, secretKey) {
    validateOffer(payload);
    const payloadMap = workOfferToCbor(payload);
    const payloadBytes = encodeCanonical(payloadMap);
    const digest = sha256(payloadBytes);
    const signature = signEd25519Hash(secretKey, digest);
    const full = withSignature(payloadMap, WORK_SIG_KEY, signature);
    return encodeCanonical(full);
}
export function verifyWorkOffer(data, publicKey) {
    const value = decodeCanonical(data);
    const [payload, signature] = splitSignedMap(value, WORK_SIG_KEY);
    const payloadBytes = encodeCanonical(payload);
    const digest = sha256(payloadBytes);
    verifyEd25519Hash(publicKey, digest, signature);
    return parseWorkOfferPayload(payload);
}
export function parseWorkAgreementPayload(value) {
    const entries = expectMap(value);
    const payload = {
        agreementId: expectText(getRequired(entries, 0)),
        offerId: expectText(getRequired(entries, 1)),
        issuer: expectText(getRequired(entries, 2)),
        counterparty: expectText(getRequired(entries, 3)),
        budgetAmount: expectU64(getRequired(entries, 4)),
        budgetCurrency: expectText(getRequired(entries, 5)),
        startTs: expectU64(getRequired(entries, 6)),
        endTs: expectU64(getRequired(entries, 7)),
        deliverables: expectTextArray(getRequired(entries, 8)),
        milestones: getOptional(entries, 9) ? parseMilestones(getRequired(entries, 9)) : undefined,
        escrowId: getOptional(entries, 10) ? expectText(getRequired(entries, 10)) : undefined,
        disputePolicy: getOptional(entries, 11) ?? undefined,
        ts: expectU64(getRequired(entries, 12)),
    };
    validateAgreement(payload);
    return payload;
}
export function parseWorkAgreement(value) {
    const [payload, signature] = splitSignedMap(value, WORK_SIG_KEY);
    return { payload: parseWorkAgreementPayload(payload), signature };
}
export function decodeWorkAgreement(data) {
    return parseWorkAgreement(decodeCanonical(data));
}
export function buildWorkAgreement(payload, secretKey) {
    validateAgreement(payload);
    const payloadMap = workAgreementToCbor(payload);
    const payloadBytes = encodeCanonical(payloadMap);
    const digest = sha256(payloadBytes);
    const signature = signEd25519Hash(secretKey, digest);
    const full = withSignature(payloadMap, WORK_SIG_KEY, signature);
    return encodeCanonical(full);
}
export function verifyWorkAgreement(data, publicKey) {
    const value = decodeCanonical(data);
    const [payload, signature] = splitSignedMap(value, WORK_SIG_KEY);
    const payloadBytes = encodeCanonical(payload);
    const digest = sha256(payloadBytes);
    verifyEd25519Hash(publicKey, digest, signature);
    return parseWorkAgreementPayload(payload);
}
export function parseWorkOfferPublishPayload(value) {
    const entries = expectMap(value);
    const offer = expectBytes(getRequired(entries, 0));
    const ts = expectU64(getRequired(entries, 1));
    if (ts === 0)
        throw new Error("timestamp required");
    decodeWorkOffer(offer);
    return { offer, ts };
}
export function parseWorkAgreementPublishPayload(value) {
    const entries = expectMap(value);
    const agreement = expectBytes(getRequired(entries, 0));
    const ts = expectU64(getRequired(entries, 1));
    if (ts === 0)
        throw new Error("timestamp required");
    decodeWorkAgreement(agreement);
    return { agreement, ts };
}
export function parseWorkAgreementUpdatePayload(value) {
    const entries = expectMap(value);
    const agreementId = expectText(getRequired(entries, 0));
    const prevAgreementHash = expectBytesLen(getRequired(entries, 1), 32);
    const agreement = expectBytes(getRequired(entries, 2));
    const ts = expectU64(getRequired(entries, 3));
    if (ts === 0)
        throw new Error("timestamp required");
    decodeWorkAgreement(agreement);
    return { agreementId, prevAgreementHash, agreement, ts };
}
export function parseWorkAgreementClosePayload(value) {
    const entries = expectMap(value);
    const agreementId = expectText(getRequired(entries, 0));
    const agreementHash = expectBytesLen(getRequired(entries, 1), 32);
    const reason = expectText(getRequired(entries, 2));
    const ts = expectU64(getRequired(entries, 3));
    if (ts === 0)
        throw new Error("timestamp required");
    if (!reason.trim())
        throw new Error("reason required");
    return { agreementId, agreementHash, reason, ts };
}
export function workOfferPublishPayloadToCbor(payload) {
    if (payload.ts === 0)
        throw new Error("timestamp required");
    decodeWorkOffer(payload.offer);
    return { entries: [[0, payload.offer], [1, payload.ts]] };
}
export function workAgreementPublishPayloadToCbor(payload) {
    if (payload.ts === 0)
        throw new Error("timestamp required");
    decodeWorkAgreement(payload.agreement);
    return { entries: [[0, payload.agreement], [1, payload.ts]] };
}
export function workAgreementUpdatePayloadToCbor(payload) {
    if (payload.ts === 0)
        throw new Error("timestamp required");
    ensureNonempty(payload.agreementId, "agreement id required");
    if (payload.prevAgreementHash.length !== 32)
        throw new Error("invalid agreement hash length");
    decodeWorkAgreement(payload.agreement);
    return {
        entries: [
            [0, payload.agreementId],
            [1, payload.prevAgreementHash],
            [2, payload.agreement],
            [3, payload.ts],
        ],
    };
}
export function workAgreementClosePayloadToCbor(payload) {
    if (payload.ts === 0)
        throw new Error("timestamp required");
    ensureNonempty(payload.agreementId, "agreement id required");
    ensureNonempty(payload.reason, "reason required");
    if (payload.agreementHash.length !== 32)
        throw new Error("invalid agreement hash length");
    return {
        entries: [
            [0, payload.agreementId],
            [1, payload.agreementHash],
            [2, payload.reason],
            [3, payload.ts],
        ],
    };
}
function workOfferToCbor(payload) {
    validateOffer(payload);
    const entries = [
        [0, payload.offerId],
        [1, payload.issuer],
        [2, payload.title],
        [3, payload.summary],
        [4, payload.scope],
        [5, payload.budgetAmount],
        [6, payload.budgetCurrency],
        [7, payload.durationSec],
        [8, [...payload.deliverables]],
        [10, payload.ts],
        [11, payload.exp],
    ];
    if (payload.requirements)
        entries.push([9, [...payload.requirements]]);
    return { entries };
}
function workAgreementToCbor(payload) {
    validateAgreement(payload);
    const entries = [
        [0, payload.agreementId],
        [1, payload.offerId],
        [2, payload.issuer],
        [3, payload.counterparty],
        [4, payload.budgetAmount],
        [5, payload.budgetCurrency],
        [6, payload.startTs],
        [7, payload.endTs],
        [8, [...payload.deliverables]],
        [12, payload.ts],
    ];
    if (payload.milestones) {
        entries.push([9, payload.milestones.map(milestoneToCbor)]);
    }
    if (payload.escrowId)
        entries.push([10, payload.escrowId]);
    if (payload.disputePolicy)
        entries.push([11, payload.disputePolicy]);
    return { entries };
}
function milestoneToCbor(milestone) {
    validateMilestone(milestone);
    const entries = [
        [0, milestone.milestoneId],
        [1, milestone.description],
        [2, milestone.dueTs],
        [3, milestone.amount],
    ];
    if (milestone.deliverableHash) {
        if (milestone.deliverableHash.length !== 32)
            throw new Error("deliverable hash must be 32 bytes");
        entries.push([4, milestone.deliverableHash]);
    }
    return { entries };
}
function parseMilestones(value) {
    if (!Array.isArray(value) || value.length === 0)
        throw new Error("milestones required");
    return value.map(parseMilestone);
}
function parseMilestone(value) {
    const entries = expectMap(value);
    const milestone = {
        milestoneId: expectText(getRequired(entries, 0)),
        description: expectText(getRequired(entries, 1)),
        dueTs: expectU64(getRequired(entries, 2)),
        amount: expectU64(getRequired(entries, 3)),
        deliverableHash: getOptional(entries, 4) ? expectBytesLen(getRequired(entries, 4), 32) : undefined,
    };
    validateMilestone(milestone);
    return milestone;
}
function validateOffer(payload) {
    ensureNonempty(payload.offerId, "offer id required");
    ensureNonempty(payload.issuer, "issuer required");
    ensureNonempty(payload.title, "title required");
    ensureNonempty(payload.summary, "summary required");
    ensureNonempty(payload.scope, "scope required");
    ensurePositive(payload.budgetAmount, "budget amount required");
    ensureNonempty(payload.budgetCurrency, "budget currency required");
    ensurePositive(payload.durationSec, "duration required");
    ensureListNonempty(payload.deliverables, "deliverables required");
    if (payload.requirements)
        ensureListItems(payload.requirements, "requirements required");
    ensurePositive(payload.ts, "timestamp required");
    if (payload.exp <= payload.ts)
        throw new Error("expiry must be after timestamp");
}
function validateAgreement(payload) {
    ensureNonempty(payload.agreementId, "agreement id required");
    ensureNonempty(payload.offerId, "offer id required");
    ensureNonempty(payload.issuer, "issuer required");
    ensureNonempty(payload.counterparty, "counterparty required");
    ensurePositive(payload.budgetAmount, "budget amount required");
    ensureNonempty(payload.budgetCurrency, "budget currency required");
    ensurePositive(payload.startTs, "start_ts required");
    ensurePositive(payload.endTs, "end_ts required");
    if (payload.endTs <= payload.startTs)
        throw new Error("end_ts must be after start_ts");
    ensureListNonempty(payload.deliverables, "deliverables required");
    if (payload.milestones) {
        if (payload.milestones.length === 0)
            throw new Error("milestones required");
        payload.milestones.forEach(validateMilestone);
    }
    if (payload.escrowId)
        ensureNonempty(payload.escrowId, "escrow id required");
    ensurePositive(payload.ts, "timestamp required");
}
function validateMilestone(milestone) {
    ensureNonempty(milestone.milestoneId, "milestone id required");
    ensureNonempty(milestone.description, "milestone description required");
    ensurePositive(milestone.dueTs, "milestone due_ts required");
    ensurePositive(milestone.amount, "milestone amount required");
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
