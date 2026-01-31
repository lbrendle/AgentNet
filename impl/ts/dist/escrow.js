export function parseEscrowLockPayload(value) {
    const entries = expectMap(value);
    return {
        escrowId: expectText(getRequired(entries, 0)),
        payer: expectText(getRequired(entries, 1)),
        payee: expectText(getRequired(entries, 2)),
        amount: expectU64(getRequired(entries, 3)),
        currency: expectText(getRequired(entries, 4)),
        releaseCondition: getRequired(entries, 5),
        disputeWindowSec: expectU64(getRequired(entries, 6)),
        expiry: expectU64(getRequired(entries, 7)),
    };
}
export function parseEscrowReleasePayload(value) {
    const entries = expectMap(value);
    const evidence = expectBytes(getRequired(entries, 1));
    if (evidence.length !== 32) {
        throw new Error("evidenceReceiptHash must be 32 bytes");
    }
    return {
        escrowId: expectText(getRequired(entries, 0)),
        evidenceReceiptHash: evidence,
        ts: expectU64(getRequired(entries, 2)),
    };
}
export function parseEscrowDisputePayload(value) {
    const entries = expectMap(value);
    const evidence = expectBytes(getRequired(entries, 2));
    if (evidence.length !== 32) {
        throw new Error("evidenceAnchorOrReceipt must be 32 bytes");
    }
    return {
        escrowId: expectText(getRequired(entries, 0)),
        reason: expectText(getRequired(entries, 1)),
        evidenceAnchorOrReceipt: evidence,
        ts: expectU64(getRequired(entries, 3)),
    };
}
export function parseEscrowResolvePayload(value) {
    const entries = expectMap(value);
    const splitValue = getOptional(entries, 2);
    return {
        escrowId: expectText(getRequired(entries, 0)),
        outcome: expectU8(getRequired(entries, 1)),
        splitAmountToPayee: splitValue === null ? undefined : expectU64(splitValue),
        ts: expectU64(getRequired(entries, 3)),
    };
}
export function escrowLockPayloadToCbor(payload) {
    return {
        entries: [
            [0, payload.escrowId],
            [1, payload.payer],
            [2, payload.payee],
            [3, payload.amount],
            [4, payload.currency],
            [5, payload.releaseCondition],
            [6, payload.disputeWindowSec],
            [7, payload.expiry],
        ],
    };
}
export function escrowReleasePayloadToCbor(payload) {
    return {
        entries: [
            [0, payload.escrowId],
            [1, payload.evidenceReceiptHash],
            [2, payload.ts],
        ],
    };
}
export function escrowDisputePayloadToCbor(payload) {
    return {
        entries: [
            [0, payload.escrowId],
            [1, payload.reason],
            [2, payload.evidenceAnchorOrReceipt],
            [3, payload.ts],
        ],
    };
}
export function escrowResolvePayloadToCbor(payload) {
    const entries = [
        [0, payload.escrowId],
        [1, payload.outcome],
        [3, payload.ts],
    ];
    if (payload.splitAmountToPayee !== undefined) {
        entries.push([2, payload.splitAmountToPayee]);
    }
    return { entries };
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
function getOptional(entries, key) {
    for (const [k, v] of entries) {
        const idx = toNumber(k);
        if (typeof idx === "number" && idx === key)
            return v;
    }
    return null;
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
        if (!Number.isSafeInteger(asNumber)) {
            throw new Error("expected u64");
        }
        return asNumber;
    }
    if (typeof value === "number" && Number.isSafeInteger(value) && value >= 0)
        return value;
    throw new Error("expected u64");
}
function expectU8(value) {
    const num = expectU64(value);
    if (num > 0xff)
        throw new Error("expected u8");
    return num;
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
