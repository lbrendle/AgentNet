import { CborMap, CborValue, decodeCanonical, encodeCanonical } from "./cbor.js";
import { sha256, signEd25519Hash, verifyEd25519Hash } from "./crypto.js";

export interface EconomicProof {
  kind: number;
  data: Uint8Array;
}

export interface PubSubEnvelopePayload {
  version: number;
  topic: string;
  sender: string;
  ts: number;
  seq: number;
  payloadType: number;
  payload: CborValue;
  economicProof?: EconomicProof;
}

export interface PubSubEnvelope {
  payload: PubSubEnvelopePayload;
  signature: Uint8Array;
}

export function economicProofOnChain(txHash: Uint8Array): EconomicProof {
  return { kind: 1, data: txHash };
}

export function economicProofVoucher(voucher: Uint8Array): EconomicProof {
  return { kind: 2, data: voucher };
}

export function parsePubSubPayload(value: CborValue): PubSubEnvelopePayload {
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

export function parsePubSubEnvelope(value: CborValue): PubSubEnvelope {
  const [payload, signature] = splitSignedMap(value, 8);
  return { payload: parsePubSubPayload(payload), signature };
}

export function decodePubSubEnvelope(data: Uint8Array): PubSubEnvelope {
  return parsePubSubEnvelope(decodeCanonical(data));
}

export function buildPubSubEnvelope(payload: PubSubEnvelopePayload, secretKey: Uint8Array): Uint8Array {
  const payloadMap = payloadToCbor(payload);
  const payloadCbor = encodeCanonical(payloadMap);
  const digest = sha256(payloadCbor);
  const signature = signEd25519Hash(secretKey, digest);
  const full = withSignature(payloadMap, 8, signature);
  return encodeCanonical(full);
}

export function verifyPubSubEnvelope(data: Uint8Array, publicKey: Uint8Array): PubSubEnvelopePayload {
  const value = decodeCanonical(data);
  const [payload, signature] = splitSignedMap(value, 8);
  const payloadCbor = encodeCanonical(payload);
  const digest = sha256(payloadCbor);
  verifyEd25519Hash(publicKey, digest, signature);
  return parsePubSubPayload(payload);
}

function payloadToCbor(payload: PubSubEnvelopePayload): CborValue {
  const entries: [CborValue, CborValue][] = [
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
  return { entries } satisfies CborMap;
}

function economicProofToCbor(proof: EconomicProof): CborValue {
  return { entries: [[0, proof.kind], [1, proof.data]] } satisfies CborMap;
}

function parseEconomicProof(value: CborValue): EconomicProof {
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
    if (typeof idx === "number" && idx === key) {
      return v;
    }
  }
  throw new Error("missing required key");
}

function getOptional(entries: [CborValue, CborValue][], key: number): CborValue | null {
  for (const [k, v] of entries) {
    const idx = toNumber(k);
    if (typeof idx === "number" && idx === key) {
      return v;
    }
  }
  return null;
}

function expectText(value: CborValue): string {
  if (typeof value === "string") return value;
  throw new Error("expected text");
}

function expectBytes(value: CborValue): Uint8Array {
  if (!(value instanceof Uint8Array)) throw new Error("expected bytes");
  return value;
}

function expectU64(value: CborValue): number {
  const num = toNumber(value);
  if (typeof num !== "number" || num < 0 || !Number.isSafeInteger(num)) {
    throw new Error("expected unsigned");
  }
  return num;
}

function expectU16(value: CborValue): number {
  const num = toNumber(value);
  if (typeof num !== "number" || num < 0 || num > 0xffff) {
    throw new Error("expected u16");
  }
  return num;
}

function expectU8(value: CborValue): number {
  const num = toNumber(value);
  if (typeof num !== "number" || num < 0 || num > 0xff) {
    throw new Error("expected u8");
  }
  return num;
}

function toNumber(value: CborValue): number | null {
  if (typeof value === "number") return value;
  if (typeof value === "bigint") {
    const num = Number(value);
    if (!Number.isSafeInteger(num)) throw new Error("integer overflow");
    return num;
  }
  return null;
}
