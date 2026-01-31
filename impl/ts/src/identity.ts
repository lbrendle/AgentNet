import { CborMap, CborValue } from "./cbor.js";

export interface IdentityRegisterPayload {
  agentId: string;
  pkEd25519: Uint8Array;
  pkX25519: Uint8Array;
  created: number;
}

export interface IdentityRotatePayload {
  agentId: string;
  pkEd25519: Uint8Array;
  pkX25519: Uint8Array;
  ts: number;
}

export interface CredentialRevokePayload {
  issuer: string;
  credentialIdHash: Uint8Array;
  ts: number;
}

export function parseIdentityRegisterPayload(value: CborValue): IdentityRegisterPayload {
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

export function parseIdentityRotatePayload(value: CborValue): IdentityRotatePayload {
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

export function parseCredentialRevokePayload(value: CborValue): CredentialRevokePayload {
  const entries = expectMap(value);
  const cred = expectBytes(getRequired(entries, 1));
  if (cred.length !== 32) throw new Error("credentialIdHash must be 32 bytes");
  return {
    issuer: expectText(getRequired(entries, 0)),
    credentialIdHash: cred,
    ts: expectU64(getRequired(entries, 2)),
  };
}

export function identityRegisterPayloadToCbor(payload: IdentityRegisterPayload): CborValue {
  return {
    entries: [
      [0, payload.agentId],
      [1, payload.pkEd25519],
      [2, payload.pkX25519],
      [3, payload.created],
    ],
  } satisfies CborMap;
}

export function identityRotatePayloadToCbor(payload: IdentityRotatePayload): CborValue {
  return {
    entries: [
      [0, payload.agentId],
      [1, payload.pkEd25519],
      [2, payload.pkX25519],
      [3, payload.ts],
    ],
  } satisfies CborMap;
}

export function credentialRevokePayloadToCbor(payload: CredentialRevokePayload): CborValue {
  return {
    entries: [
      [0, payload.issuer],
      [1, payload.credentialIdHash],
      [2, payload.ts],
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

function expectBytes(value: CborValue): Uint8Array {
  if (value instanceof Uint8Array) return value;
  throw new Error("expected bytes");
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
