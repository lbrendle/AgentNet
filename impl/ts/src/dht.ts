import { CborMap, CborValue, decodeCanonical, encodeCanonical } from "./cbor.js";
import { sha256, signEd25519Hash, verifyEd25519Hash } from "./crypto.js";

export interface Contact {
  nodeIds: string[];
  addrs: string[];
}

export interface AgentRecordPayload {
  agentId: string;
  agentPubkeys: Uint8Array[];
  contact: Contact;
  capabilities: string[];
  expires: number;
}

export interface AgentRecord {
  payload: AgentRecordPayload;
  signature: Uint8Array;
}

export interface ServiceRecordPayload {
  providerId: string;
  serviceType: number;
  addrs: string[];
  requiredCredentials?: string[];
  pricing?: CborValue;
  expires: number;
}

export interface ServiceRecord {
  payload: ServiceRecordPayload;
  signature: Uint8Array;
}

export interface CommunityRecordPayload {
  communityId: string;
  controller: string;
  joinPolicy: number;
  requiredCredentials?: string[];
  economics: CborValue;
  governance: CborValue;
  expires: number;
}

export interface CommunityRecord {
  payload: CommunityRecordPayload;
  signature: Uint8Array;
}

export function parseContact(value: CborValue): Contact {
  const entries = expectMap(value);
  return {
    nodeIds: expectTextArray(getRequired(entries, 0)),
    addrs: expectTextArray(getRequired(entries, 1)),
  };
}

export function parseAgentRecordPayload(value: CborValue): AgentRecordPayload {
  const entries = expectMap(value);
  return {
    agentId: expectText(getRequired(entries, 0)),
    agentPubkeys: expectBytesArrayLen(getRequired(entries, 1), 32),
    contact: parseContact(getRequired(entries, 2)),
    capabilities: expectTextArray(getRequired(entries, 3)),
    expires: expectU64(getRequired(entries, 4)),
  };
}

export function parseAgentRecord(value: CborValue): AgentRecord {
  const [payload, signature] = splitSignedMap(value, 5);
  return { payload: parseAgentRecordPayload(payload), signature };
}

export function parseServiceRecordPayload(value: CborValue): ServiceRecordPayload {
  const entries = expectMap(value);
  const requiredCredentials = getOptional(entries, 3);
  const pricing = getOptional(entries, 4);
  return {
    providerId: expectText(getRequired(entries, 0)),
    serviceType: expectU16(getRequired(entries, 1)),
    addrs: expectTextArray(getRequired(entries, 2)),
    requiredCredentials: requiredCredentials ? expectTextArray(requiredCredentials) : undefined,
    pricing: pricing ?? undefined,
    expires: expectU64(getRequired(entries, 5)),
  };
}

export function parseServiceRecord(value: CborValue): ServiceRecord {
  const [payload, signature] = splitSignedMap(value, 6);
  return { payload: parseServiceRecordPayload(payload), signature };
}

export function parseCommunityRecordPayload(value: CborValue): CommunityRecordPayload {
  const entries = expectMap(value);
  const requiredCredentials = getOptional(entries, 3);
  return {
    communityId: expectText(getRequired(entries, 0)),
    controller: expectText(getRequired(entries, 1)),
    joinPolicy: expectU8(getRequired(entries, 2)),
    requiredCredentials: requiredCredentials ? expectTextArray(requiredCredentials) : undefined,
    economics: getRequired(entries, 4),
    governance: getRequired(entries, 5),
    expires: expectU64(getRequired(entries, 6)),
  };
}

export function parseCommunityRecord(value: CborValue): CommunityRecord {
  const [payload, signature] = splitSignedMap(value, 7);
  return { payload: parseCommunityRecordPayload(payload), signature };
}

export function buildAgentRecord(payload: AgentRecordPayload, secretKey: Uint8Array): Uint8Array {
  return buildSignedRecord(agentRecordPayloadToCbor(payload), 5, secretKey);
}

export function buildServiceRecord(payload: ServiceRecordPayload, secretKey: Uint8Array): Uint8Array {
  return buildSignedRecord(serviceRecordPayloadToCbor(payload), 6, secretKey);
}

export function buildCommunityRecord(payload: CommunityRecordPayload, secretKey: Uint8Array): Uint8Array {
  return buildSignedRecord(communityRecordPayloadToCbor(payload), 7, secretKey);
}

export function verifyAgentRecord(data: Uint8Array, publicKey: Uint8Array): AgentRecordPayload {
  return verifySignedRecord(data, 5, publicKey, parseAgentRecordPayload) as AgentRecordPayload;
}

export function verifyServiceRecord(data: Uint8Array, publicKey: Uint8Array): ServiceRecordPayload {
  return verifySignedRecord(data, 6, publicKey, parseServiceRecordPayload) as ServiceRecordPayload;
}

export function verifyCommunityRecord(data: Uint8Array, publicKey: Uint8Array): CommunityRecordPayload {
  return verifySignedRecord(data, 7, publicKey, parseCommunityRecordPayload) as CommunityRecordPayload;
}

function agentRecordPayloadToCbor(payload: AgentRecordPayload): CborValue {
  return {
    entries: [
      [0, payload.agentId],
      [1, payload.agentPubkeys.map((k) => Uint8Array.from(k))],
      [2, contactToCbor(payload.contact)],
      [3, payload.capabilities.map((v) => v)],
      [4, payload.expires],
    ],
  } satisfies CborMap;
}

function serviceRecordPayloadToCbor(payload: ServiceRecordPayload): CborValue {
  const entries: [CborValue, CborValue][] = [
    [0, payload.providerId],
    [1, payload.serviceType],
    [2, payload.addrs.map((v) => v)],
  ];
  if (payload.requiredCredentials) {
    entries.push([3, payload.requiredCredentials.map((v) => v)]);
  }
  if (payload.pricing !== undefined) {
    entries.push([4, payload.pricing]);
  }
  entries.push([5, payload.expires]);
  return { entries } satisfies CborMap;
}

function communityRecordPayloadToCbor(payload: CommunityRecordPayload): CborValue {
  const entries: [CborValue, CborValue][] = [
    [0, payload.communityId],
    [1, payload.controller],
    [2, payload.joinPolicy],
  ];
  if (payload.requiredCredentials) {
    entries.push([3, payload.requiredCredentials.map((v) => v)]);
  }
  entries.push([4, payload.economics]);
  entries.push([5, payload.governance]);
  entries.push([6, payload.expires]);
  return { entries } satisfies CborMap;
}

function contactToCbor(contact: Contact): CborValue {
  return {
    entries: [
      [0, contact.nodeIds.map((v) => v)],
      [1, contact.addrs.map((v) => v)],
    ],
  } satisfies CborMap;
}

function buildSignedRecord(payload: CborValue, sigKey: number, secretKey: Uint8Array): Uint8Array {
  const payloadCbor = encodeCanonical(payload);
  const digest = sha256(payloadCbor);
  const signature = signEd25519Hash(secretKey, digest);
  const full = withSignature(payload, sigKey, signature);
  return encodeCanonical(full);
}

function verifySignedRecord(
  data: Uint8Array,
  sigKey: number,
  publicKey: Uint8Array,
  parser: (value: CborValue) => unknown,
): unknown {
  const value = decodeCanonical(data);
  const [payload, signature] = splitSignedMap(value, sigKey);
  const payloadCbor = encodeCanonical(payload);
  const digest = sha256(payloadCbor);
  verifyEd25519Hash(publicKey, digest, signature);
  return parser(payload);
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

function expectTextArray(value: CborValue): string[] {
  if (!Array.isArray(value)) throw new Error("expected text array");
  return value.map((item) => expectText(item));
}

function expectBytesArrayLen(value: CborValue, len: number): Uint8Array[] {
  if (!Array.isArray(value)) throw new Error("expected bytes array");
  return value.map((item) => expectBytesLen(item, len));
}

function expectBytesLen(value: CborValue, len: number): Uint8Array {
  if (!(value instanceof Uint8Array)) throw new Error("expected bytes");
  if (value.length !== len) throw new Error("invalid length");
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
