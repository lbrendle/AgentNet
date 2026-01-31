import { CborMap, CborValue } from "./cbor.js";

export interface TransferPayload {
  fromDid: string;
  toDid: string;
  amount: number;
  currency: string;
  ts: number;
}

export interface PostagePayload {
  payer: string;
  amount: number;
  currency: string;
  purpose: string;
  ts: number;
}

export function parseTransferPayload(value: CborValue): TransferPayload {
  const entries = expectMap(value);
  return {
    fromDid: expectText(getRequired(entries, 0)),
    toDid: expectText(getRequired(entries, 1)),
    amount: expectU64(getRequired(entries, 2)),
    currency: expectText(getRequired(entries, 3)),
    ts: expectU64(getRequired(entries, 4)),
  };
}

export function parsePostagePayload(value: CborValue): PostagePayload {
  const entries = expectMap(value);
  return {
    payer: expectText(getRequired(entries, 0)),
    amount: expectU64(getRequired(entries, 1)),
    currency: expectText(getRequired(entries, 2)),
    purpose: expectText(getRequired(entries, 3)),
    ts: expectU64(getRequired(entries, 4)),
  };
}

export function transferPayloadToCbor(payload: TransferPayload): CborValue {
  return {
    entries: [
      [0, payload.fromDid],
      [1, payload.toDid],
      [2, payload.amount],
      [3, payload.currency],
      [4, payload.ts],
    ],
  } satisfies CborMap;
}

export function postagePayloadToCbor(payload: PostagePayload): CborValue {
  return {
    entries: [
      [0, payload.payer],
      [1, payload.amount],
      [2, payload.currency],
      [3, payload.purpose],
      [4, payload.ts],
    ],
  } satisfies CborMap;
}

function expectMap(value: CborValue): [CborValue, CborValue][] {
  if (typeof value === "object" && value !== null && "entries" in value) {
    return (value as CborMap).entries;
  }
  throw new Error("expected map");
}

function getRequired(entries: [CborValue, CborValue][], key: number): CborValue {
  for (const [k, v] of entries) {
    const idx = toNumber(k);
    if (typeof idx === "number" && idx === key) return v;
  }
  throw new Error("missing required key");
}

function expectText(value: CborValue): string {
  if (typeof value === "string") return value;
  throw new Error("expected text");
}

function expectU64(value: CborValue): number {
  if (typeof value === "bigint") {
    const asNumber = Number(value);
    if (!Number.isSafeInteger(asNumber)) throw new Error("expected u64");
    return asNumber;
  }
  if (typeof value === "number" && Number.isSafeInteger(value) && value >= 0) return value;
  throw new Error("expected u64");
}

function toNumber(value: CborValue): number | null {
  if (typeof value === "number") return value;
  if (typeof value === "bigint") {
    const asNumber = Number(value);
    if (!Number.isSafeInteger(asNumber)) return null;
    return asNumber;
  }
  return null;
}
