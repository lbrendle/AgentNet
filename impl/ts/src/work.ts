import { CborMap, CborValue, decodeCanonical, encodeCanonical } from "./cbor.js";
import { sha256, signEd25519Hash, verifyEd25519Hash } from "./crypto.js";

const WORK_SIG_KEY = 16;

export interface WorkMilestone {
  milestoneId: string;
  description: string;
  dueTs: number;
  amount: number;
  deliverableHash?: Uint8Array;
}

export interface WorkOfferPayload {
  offerId: string;
  issuer: string;
  title: string;
  summary: string;
  scope: string;
  budgetAmount: number;
  budgetCurrency: string;
  durationSec: number;
  deliverables: string[];
  requirements?: string[];
  ts: number;
  exp: number;
}

export interface WorkOffer {
  payload: WorkOfferPayload;
  signature: Uint8Array;
}

export interface WorkAgreementPayload {
  agreementId: string;
  offerId: string;
  issuer: string;
  counterparty: string;
  budgetAmount: number;
  budgetCurrency: string;
  startTs: number;
  endTs: number;
  deliverables: string[];
  milestones?: WorkMilestone[];
  escrowId?: string;
  disputePolicy?: CborValue;
  ts: number;
}

export interface WorkAgreement {
  payload: WorkAgreementPayload;
  signature: Uint8Array;
}

export interface WorkOfferPublishPayload {
  offer: Uint8Array;
  ts: number;
}

export interface WorkAgreementPublishPayload {
  agreement: Uint8Array;
  ts: number;
}

export interface WorkAgreementUpdatePayload {
  agreementId: string;
  prevAgreementHash: Uint8Array;
  agreement: Uint8Array;
  ts: number;
}

export interface WorkAgreementClosePayload {
  agreementId: string;
  agreementHash: Uint8Array;
  reason: string;
  ts: number;
}

export function parseWorkOfferPayload(value: CborValue): WorkOfferPayload {
  const entries = expectMap(value);
  const payload: WorkOfferPayload = {
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

export function parseWorkOffer(value: CborValue): WorkOffer {
  const [payload, signature] = splitSignedMap(value, WORK_SIG_KEY);
  return { payload: parseWorkOfferPayload(payload), signature };
}

export function decodeWorkOffer(data: Uint8Array): WorkOffer {
  return parseWorkOffer(decodeCanonical(data));
}

export function buildWorkOffer(payload: WorkOfferPayload, secretKey: Uint8Array): Uint8Array {
  validateOffer(payload);
  const payloadMap = workOfferToCbor(payload);
  const payloadBytes = encodeCanonical(payloadMap);
  const digest = sha256(payloadBytes);
  const signature = signEd25519Hash(secretKey, digest);
  const full = withSignature(payloadMap, WORK_SIG_KEY, signature);
  return encodeCanonical(full);
}

export function verifyWorkOffer(data: Uint8Array, publicKey: Uint8Array): WorkOfferPayload {
  const value = decodeCanonical(data);
  const [payload, signature] = splitSignedMap(value, WORK_SIG_KEY);
  const payloadBytes = encodeCanonical(payload);
  const digest = sha256(payloadBytes);
  verifyEd25519Hash(publicKey, digest, signature);
  return parseWorkOfferPayload(payload);
}

export function parseWorkAgreementPayload(value: CborValue): WorkAgreementPayload {
  const entries = expectMap(value);
  const payload: WorkAgreementPayload = {
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

export function parseWorkAgreement(value: CborValue): WorkAgreement {
  const [payload, signature] = splitSignedMap(value, WORK_SIG_KEY);
  return { payload: parseWorkAgreementPayload(payload), signature };
}

export function decodeWorkAgreement(data: Uint8Array): WorkAgreement {
  return parseWorkAgreement(decodeCanonical(data));
}

export function buildWorkAgreement(payload: WorkAgreementPayload, secretKey: Uint8Array): Uint8Array {
  validateAgreement(payload);
  const payloadMap = workAgreementToCbor(payload);
  const payloadBytes = encodeCanonical(payloadMap);
  const digest = sha256(payloadBytes);
  const signature = signEd25519Hash(secretKey, digest);
  const full = withSignature(payloadMap, WORK_SIG_KEY, signature);
  return encodeCanonical(full);
}

export function verifyWorkAgreement(data: Uint8Array, publicKey: Uint8Array): WorkAgreementPayload {
  const value = decodeCanonical(data);
  const [payload, signature] = splitSignedMap(value, WORK_SIG_KEY);
  const payloadBytes = encodeCanonical(payload);
  const digest = sha256(payloadBytes);
  verifyEd25519Hash(publicKey, digest, signature);
  return parseWorkAgreementPayload(payload);
}

export function parseWorkOfferPublishPayload(value: CborValue): WorkOfferPublishPayload {
  const entries = expectMap(value);
  const offer = expectBytes(getRequired(entries, 0));
  const ts = expectU64(getRequired(entries, 1));
  if (ts === 0) throw new Error("timestamp required");
  decodeWorkOffer(offer);
  return { offer, ts };
}

export function parseWorkAgreementPublishPayload(value: CborValue): WorkAgreementPublishPayload {
  const entries = expectMap(value);
  const agreement = expectBytes(getRequired(entries, 0));
  const ts = expectU64(getRequired(entries, 1));
  if (ts === 0) throw new Error("timestamp required");
  decodeWorkAgreement(agreement);
  return { agreement, ts };
}

export function parseWorkAgreementUpdatePayload(value: CborValue): WorkAgreementUpdatePayload {
  const entries = expectMap(value);
  const agreementId = expectText(getRequired(entries, 0));
  const prevAgreementHash = expectBytesLen(getRequired(entries, 1), 32);
  const agreement = expectBytes(getRequired(entries, 2));
  const ts = expectU64(getRequired(entries, 3));
  if (ts === 0) throw new Error("timestamp required");
  decodeWorkAgreement(agreement);
  return { agreementId, prevAgreementHash, agreement, ts };
}

export function parseWorkAgreementClosePayload(value: CborValue): WorkAgreementClosePayload {
  const entries = expectMap(value);
  const agreementId = expectText(getRequired(entries, 0));
  const agreementHash = expectBytesLen(getRequired(entries, 1), 32);
  const reason = expectText(getRequired(entries, 2));
  const ts = expectU64(getRequired(entries, 3));
  if (ts === 0) throw new Error("timestamp required");
  if (!reason.trim()) throw new Error("reason required");
  return { agreementId, agreementHash, reason, ts };
}

export function workOfferPublishPayloadToCbor(payload: WorkOfferPublishPayload): CborValue {
  if (payload.ts === 0) throw new Error("timestamp required");
  decodeWorkOffer(payload.offer);
  return { entries: [[0, payload.offer], [1, payload.ts]] } satisfies CborMap;
}

export function workAgreementPublishPayloadToCbor(payload: WorkAgreementPublishPayload): CborValue {
  if (payload.ts === 0) throw new Error("timestamp required");
  decodeWorkAgreement(payload.agreement);
  return { entries: [[0, payload.agreement], [1, payload.ts]] } satisfies CborMap;
}

export function workAgreementUpdatePayloadToCbor(payload: WorkAgreementUpdatePayload): CborValue {
  if (payload.ts === 0) throw new Error("timestamp required");
  ensureNonempty(payload.agreementId, "agreement id required");
  if (payload.prevAgreementHash.length !== 32) throw new Error("invalid agreement hash length");
  decodeWorkAgreement(payload.agreement);
  return {
    entries: [
      [0, payload.agreementId],
      [1, payload.prevAgreementHash],
      [2, payload.agreement],
      [3, payload.ts],
    ],
  } satisfies CborMap;
}

export function workAgreementClosePayloadToCbor(payload: WorkAgreementClosePayload): CborValue {
  if (payload.ts === 0) throw new Error("timestamp required");
  ensureNonempty(payload.agreementId, "agreement id required");
  ensureNonempty(payload.reason, "reason required");
  if (payload.agreementHash.length !== 32) throw new Error("invalid agreement hash length");
  return {
    entries: [
      [0, payload.agreementId],
      [1, payload.agreementHash],
      [2, payload.reason],
      [3, payload.ts],
    ],
  } satisfies CborMap;
}

function workOfferToCbor(payload: WorkOfferPayload): CborValue {
  validateOffer(payload);
  const entries: [CborValue, CborValue][] = [
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
  if (payload.requirements) entries.push([9, [...payload.requirements]]);
  return { entries } satisfies CborMap;
}

function workAgreementToCbor(payload: WorkAgreementPayload): CborValue {
  validateAgreement(payload);
  const entries: [CborValue, CborValue][] = [
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
  if (payload.escrowId) entries.push([10, payload.escrowId]);
  if (payload.disputePolicy) entries.push([11, payload.disputePolicy]);
  return { entries } satisfies CborMap;
}

function milestoneToCbor(milestone: WorkMilestone): CborValue {
  validateMilestone(milestone);
  const entries: [CborValue, CborValue][] = [
    [0, milestone.milestoneId],
    [1, milestone.description],
    [2, milestone.dueTs],
    [3, milestone.amount],
  ];
  if (milestone.deliverableHash) {
    if (milestone.deliverableHash.length !== 32) throw new Error("deliverable hash must be 32 bytes");
    entries.push([4, milestone.deliverableHash]);
  }
  return { entries } satisfies CborMap;
}

function parseMilestones(value: CborValue): WorkMilestone[] {
  if (!Array.isArray(value) || value.length === 0) throw new Error("milestones required");
  return value.map(parseMilestone);
}

function parseMilestone(value: CborValue): WorkMilestone {
  const entries = expectMap(value);
  const milestone: WorkMilestone = {
    milestoneId: expectText(getRequired(entries, 0)),
    description: expectText(getRequired(entries, 1)),
    dueTs: expectU64(getRequired(entries, 2)),
    amount: expectU64(getRequired(entries, 3)),
    deliverableHash: getOptional(entries, 4) ? expectBytesLen(getRequired(entries, 4), 32) : undefined,
  };
  validateMilestone(milestone);
  return milestone;
}

function validateOffer(payload: WorkOfferPayload): void {
  ensureNonempty(payload.offerId, "offer id required");
  ensureNonempty(payload.issuer, "issuer required");
  ensureNonempty(payload.title, "title required");
  ensureNonempty(payload.summary, "summary required");
  ensureNonempty(payload.scope, "scope required");
  ensurePositive(payload.budgetAmount, "budget amount required");
  ensureNonempty(payload.budgetCurrency, "budget currency required");
  ensurePositive(payload.durationSec, "duration required");
  ensureListNonempty(payload.deliverables, "deliverables required");
  if (payload.requirements) ensureListItems(payload.requirements, "requirements required");
  ensurePositive(payload.ts, "timestamp required");
  if (payload.exp <= payload.ts) throw new Error("expiry must be after timestamp");
}

function validateAgreement(payload: WorkAgreementPayload): void {
  ensureNonempty(payload.agreementId, "agreement id required");
  ensureNonempty(payload.offerId, "offer id required");
  ensureNonempty(payload.issuer, "issuer required");
  ensureNonempty(payload.counterparty, "counterparty required");
  ensurePositive(payload.budgetAmount, "budget amount required");
  ensureNonempty(payload.budgetCurrency, "budget currency required");
  ensurePositive(payload.startTs, "start_ts required");
  ensurePositive(payload.endTs, "end_ts required");
  if (payload.endTs <= payload.startTs) throw new Error("end_ts must be after start_ts");
  ensureListNonempty(payload.deliverables, "deliverables required");
  if (payload.milestones) {
    if (payload.milestones.length === 0) throw new Error("milestones required");
    payload.milestones.forEach(validateMilestone);
  }
  if (payload.escrowId) ensureNonempty(payload.escrowId, "escrow id required");
  ensurePositive(payload.ts, "timestamp required");
}

function validateMilestone(milestone: WorkMilestone): void {
  ensureNonempty(milestone.milestoneId, "milestone id required");
  ensureNonempty(milestone.description, "milestone description required");
  ensurePositive(milestone.dueTs, "milestone due_ts required");
  ensurePositive(milestone.amount, "milestone amount required");
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

function expectU64(value: CborValue): number {
  const num = toNumber(value);
  if (typeof num !== "number" || num < 0 || !Number.isSafeInteger(num)) {
    throw new Error("expected unsigned");
  }
  return num;
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

function ensureNonempty(value: string, message: string): void {
  if (!value || value.trim().length === 0) throw new Error(message);
}

function ensureListNonempty(values: string[], message: string): void {
  if (!values || values.length === 0) throw new Error(message);
  ensureListItems(values, message);
}

function ensureListItems(values: string[], message: string): void {
  for (const item of values) {
    if (!item || item.trim().length === 0) throw new Error(message);
  }
}

function ensurePositive(value: number, message: string): void {
  if (!Number.isFinite(value) || value <= 0) throw new Error(message);
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
