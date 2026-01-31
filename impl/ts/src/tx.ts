import { CborMap, CborValue, decodeCanonical, encodeCanonical } from "./cbor.js";
import { sha256, signEd25519Hash, verifyEd25519Hash } from "./crypto.js";

export interface TxEnvelopePayload {
  txType: number;
  sender: string;
  nonce: number;
  fee: number;
  payload: CborValue;
}

export interface TxEnvelope {
  payload: TxEnvelopePayload;
  signature: Uint8Array;
}

export function parseTxEnvelopePayload(value: CborValue): TxEnvelopePayload {
  const entries = expectMap(value);
  return {
    txType: expectU64(getRequired(entries, 0)),
    sender: expectText(getRequired(entries, 1)),
    nonce: expectU64(getRequired(entries, 2)),
    fee: expectU64(getRequired(entries, 3)),
    payload: getRequired(entries, 4),
  };
}

export function parseTxEnvelope(value: CborValue): TxEnvelope {
  const [payload, signature] = splitSignedMap(value, 5);
  return { payload: parseTxEnvelopePayload(payload), signature };
}

export function decodeTxEnvelope(data: Uint8Array): TxEnvelope {
  return parseTxEnvelope(decodeCanonical(data));
}

export function buildTxEnvelope(payload: TxEnvelopePayload, secretKey: Uint8Array): Uint8Array {
  const payloadMap = txEnvelopePayloadToCbor(payload);
  const payloadCbor = encodeCanonical(payloadMap);
  const digest = sha256(payloadCbor);
  const signature = signEd25519Hash(secretKey, digest);
  const full = withSignature(payloadMap, 5, signature);
  return encodeCanonical(full);
}

export function verifyTxEnvelope(data: Uint8Array, publicKey: Uint8Array): TxEnvelopePayload {
  const value = decodeCanonical(data);
  const [payload, signature] = splitSignedMap(value, 5);
  const payloadCbor = encodeCanonical(payload);
  const digest = sha256(payloadCbor);
  verifyEd25519Hash(publicKey, digest, signature);
  return parseTxEnvelopePayload(payload);
}

export function txEnvelopePayloadToCbor(payload: TxEnvelopePayload): CborValue {
  return {
    entries: [
      [0, payload.txType],
      [1, payload.sender],
      [2, payload.nonce],
      [3, payload.fee],
      [4, payload.payload],
    ],
  } satisfies CborMap;
}

function splitSignedMap(value: CborValue, sigKey: number): [CborValue, Uint8Array] {
  const entries = expectMap(value);
  const payloadEntries: [CborValue, CborValue][] = [];
  let signature: Uint8Array | null = null;
  for (const [k, v] of entries) {
    const key = toNumber(k);
    if (typeof key === "number" && key === sigKey) {
      if (signature) throw new Error("duplicate signature key");
      if (!(v instanceof Uint8Array)) throw new Error("signature must be bytes");
      signature = v;
      continue;
    }
    payloadEntries.push([k, v]);
  }
  if (!signature) throw new Error("missing signature");
  if (signature.length !== 64) throw new Error("invalid signature length");
  return [{ entries: payloadEntries }, signature];
}

function withSignature(payload: CborValue, sigKey: number, signature: Uint8Array): CborValue {
  const entries = expectMap(payload);
  return { entries: [...entries, [sigKey, signature]] } satisfies CborMap;
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
