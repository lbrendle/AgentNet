import { CborMap, CborValue, decodeCanonical, encodeCanonical } from "./cbor.js";
import { sha256, signEd25519Hash, verifyEd25519Hash } from "./crypto.js";

const SKILL_SIG_KEY = 16;
const SANDBOX_MIN = 1;
const SANDBOX_MAX = 5;

export interface SkillArtifact {
  kind: number;
  digest: Uint8Array;
  size: number;
  uris: string[];
}

export interface SkillManifestPayload {
  skillId: string;
  author: string;
  name: string;
  version: string;
  summary: string;
  license: string;
  capabilities: string[];
  permissions: string[];
  sandboxClass: number;
  endpoints?: string[];
  artifacts?: SkillArtifact[];
  requirements?: string[];
  pricing?: CborValue;
  attestations?: CborValue;
  metadata?: CborValue;
  ts: number;
}

export interface SkillManifest {
  payload: SkillManifestPayload;
  signature: Uint8Array;
}

export interface SkillPublishPayload {
  manifest: Uint8Array;
  ts: number;
}

export interface SkillUpdatePayload {
  skillId: string;
  prevManifestHash: Uint8Array;
  manifest: Uint8Array;
  ts: number;
}

export interface SkillRevokePayload {
  skillId: string;
  manifestHash: Uint8Array;
  reason: string;
  ts: number;
}

export function parseSkillManifestPayload(value: CborValue): SkillManifestPayload {
  const entries = expectMap(value);
  const payload: SkillManifestPayload = {
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

export function parseSkillManifest(value: CborValue): SkillManifest {
  const [payload, signature] = splitSignedMap(value, SKILL_SIG_KEY);
  return { payload: parseSkillManifestPayload(payload), signature };
}

export function decodeSkillManifest(data: Uint8Array): SkillManifest {
  return parseSkillManifest(decodeCanonical(data));
}

export function buildSkillManifest(payload: SkillManifestPayload, secretKey: Uint8Array): Uint8Array {
  validatePayload(payload);
  const payloadMap = payloadToCbor(payload);
  const payloadBytes = encodeCanonical(payloadMap);
  const digest = sha256(payloadBytes);
  const signature = signEd25519Hash(secretKey, digest);
  const full = withSignature(payloadMap, SKILL_SIG_KEY, signature);
  return encodeCanonical(full);
}

export function verifySkillManifest(data: Uint8Array, publicKey: Uint8Array): SkillManifestPayload {
  const value = decodeCanonical(data);
  const [payload, signature] = splitSignedMap(value, SKILL_SIG_KEY);
  const payloadBytes = encodeCanonical(payload);
  const digest = sha256(payloadBytes);
  verifyEd25519Hash(publicKey, digest, signature);
  return parseSkillManifestPayload(payload);
}

export function parseSkillPublishPayload(value: CborValue): SkillPublishPayload {
  const entries = expectMap(value);
  const manifest = expectBytes(getRequired(entries, 0));
  const ts = expectU64(getRequired(entries, 1));
  if (ts === 0) throw new Error("timestamp required");
  decodeSkillManifest(manifest);
  return { manifest, ts };
}

export function parseSkillUpdatePayload(value: CborValue): SkillUpdatePayload {
  const entries = expectMap(value);
  const skillId = expectText(getRequired(entries, 0));
  const prevManifestHash = expectBytesLen(getRequired(entries, 1), 32);
  const manifest = expectBytes(getRequired(entries, 2));
  const ts = expectU64(getRequired(entries, 3));
  if (ts === 0) throw new Error("timestamp required");
  decodeSkillManifest(manifest);
  return { skillId, prevManifestHash, manifest, ts };
}

export function parseSkillRevokePayload(value: CborValue): SkillRevokePayload {
  const entries = expectMap(value);
  const skillId = expectText(getRequired(entries, 0));
  const manifestHash = expectBytesLen(getRequired(entries, 1), 32);
  const reason = expectText(getRequired(entries, 2));
  const ts = expectU64(getRequired(entries, 3));
  if (ts === 0) throw new Error("timestamp required");
  if (!reason.trim()) throw new Error("reason required");
  return { skillId, manifestHash, reason, ts };
}

export function skillPublishPayloadToCbor(payload: SkillPublishPayload): CborValue {
  if (payload.ts === 0) throw new Error("timestamp required");
  decodeSkillManifest(payload.manifest);
  return { entries: [[0, payload.manifest], [1, payload.ts]] } satisfies CborMap;
}

export function skillUpdatePayloadToCbor(payload: SkillUpdatePayload): CborValue {
  if (payload.ts === 0) throw new Error("timestamp required");
  ensureNonempty(payload.skillId, "skill id required");
  if (payload.prevManifestHash.length !== 32) throw new Error("invalid manifest hash length");
  decodeSkillManifest(payload.manifest);
  return {
    entries: [
      [0, payload.skillId],
      [1, payload.prevManifestHash],
      [2, payload.manifest],
      [3, payload.ts],
    ],
  } satisfies CborMap;
}

export function skillRevokePayloadToCbor(payload: SkillRevokePayload): CborValue {
  if (payload.ts === 0) throw new Error("timestamp required");
  ensureNonempty(payload.skillId, "skill id required");
  ensureNonempty(payload.reason, "reason required");
  if (payload.manifestHash.length !== 32) throw new Error("invalid manifest hash length");
  return {
    entries: [
      [0, payload.skillId],
      [1, payload.manifestHash],
      [2, payload.reason],
      [3, payload.ts],
    ],
  } satisfies CborMap;
}

function payloadToCbor(payload: SkillManifestPayload): CborValue {
  validatePayload(payload);
  const entries: [CborValue, CborValue][] = [
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
  return { entries } satisfies CborMap;
}

function artifactToCbor(artifact: SkillArtifact): CborValue {
  validateArtifact(artifact);
  return { entries: [[0, artifact.kind], [1, artifact.digest], [2, artifact.size], [3, [...artifact.uris]]] } satisfies CborMap;
}

function parseArtifacts(value: CborValue): SkillArtifact[] {
  if (!Array.isArray(value) || value.length === 0) {
    throw new Error("artifacts required");
  }
  return value.map(parseArtifact);
}

function parseArtifact(value: CborValue): SkillArtifact {
  const entries = expectMap(value);
  const artifact: SkillArtifact = {
    kind: expectU8(getRequired(entries, 0)),
    digest: expectBytesLen(getRequired(entries, 1), 32),
    size: expectU64(getRequired(entries, 2)),
    uris: expectTextArray(getRequired(entries, 3)),
  };
  validateArtifact(artifact);
  return artifact;
}

function validatePayload(payload: SkillManifestPayload): void {
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
    if (payload.artifacts.length === 0) throw new Error("artifacts required");
    payload.artifacts.forEach(validateArtifact);
  }
  if (payload.requirements) {
    ensureListItems(payload.requirements, "requirements required");
  }
  if (!payload.endpoints && !payload.artifacts) {
    throw new Error("skill requires endpoints or artifacts");
  }
}

function validateArtifact(artifact: SkillArtifact): void {
  if (artifact.kind <= 0) throw new Error("artifact kind required");
  if (artifact.digest.length !== 32) throw new Error("artifact digest must be 32 bytes");
  if (artifact.size <= 0) throw new Error("artifact size required");
  ensureListNonempty(artifact.uris, "artifact uris required");
}

function ensureNonempty(value: string, message: string): void {
  if (!value || value.trim().length === 0) {
    throw new Error(message);
  }
}

function ensureListNonempty(values: string[], message: string): void {
  if (!values || values.length === 0) {
    throw new Error(message);
  }
  ensureListItems(values, message);
}

function ensureListItems(values: string[], message: string): void {
  for (const item of values) {
    if (!item || item.trim().length === 0) {
      throw new Error(message);
    }
  }
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
  if (Array.isArray(value)) return value.map(expectText);
  throw new Error("expected array of text");
}

function expectBytesLen(value: CborValue, length: number): Uint8Array {
  if (!(value instanceof Uint8Array)) throw new Error("expected bytes");
  if (value.length !== length) throw new Error("invalid length");
  return value;
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
