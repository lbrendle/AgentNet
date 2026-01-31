import { readFileSync } from "node:fs";
import { decodeCanonical, encodeCanonical } from "./cbor.js";
import { sha256, verifyEd25519Hash } from "./crypto.js";
import {
  parseSkillManifestPayload,
  parseSkillPublishPayload,
  parseSkillUpdatePayload,
  parseSkillRevokePayload,
  verifySkillManifest,
} from "./skill.js";
import {
  parseWorkOfferPayload,
  parseWorkAgreementPayload,
  parseWorkOfferPublishPayload,
  parseWorkAgreementPublishPayload,
  parseWorkAgreementUpdatePayload,
  parseWorkAgreementClosePayload,
  verifyWorkOffer,
  verifyWorkAgreement,
} from "./work.js";
import { parseAgentMailPayload, parseAgentMailMessage } from "./agentmail.js";

interface VectorEntry {
  id: string;
  object_cbor_hex?: string;
  sha256_hex?: string;
  signature_hex?: string;

  approval_payload_cbor_hex?: string;
  approval_payload_sha256_hex?: string;
  approval_signature_hex?: string;
  approval_full_object_cbor_hex?: string;
  intent_hash_hex?: string;

  grant_payload_cbor_hex?: string;
  grant_payload_sha256_hex?: string;
  grant_signature_hex?: string;
  grant_full_object_cbor_hex?: string;

  nodehello_payload_cbor_hex?: string;
  nodehello_payload_sha256_hex?: string;
  nodehello_signature_hex?: string;

  receipt1_payload_cbor_hex?: string;
  receipt1_hash_hex?: string;
  receipt1_sig_hex?: string;
  receipt2_payload_cbor_hex?: string;
  receipt2_hash_hex?: string;
  receipt2_sig_hex?: string;
  receipt2_prev_hash_hex?: string;

  tx_envelope_payload_cbor_hex?: string;
  tx_envelope_payload_sha256_hex?: string;
  tx_signature_hex?: string;

  skill_manifest_full_object_cbor_hex?: string;
  work_offer_full_object_cbor_hex?: string;
  work_agreement_full_object_cbor_hex?: string;

  skill_publish_payload_cbor_hex?: string;
  skill_publish_payload_sha256_hex?: string;
  skill_update_payload_cbor_hex?: string;
  skill_update_payload_sha256_hex?: string;
  skill_revoke_payload_cbor_hex?: string;
  skill_revoke_payload_sha256_hex?: string;

  work_offer_payload_cbor_hex?: string;
  work_offer_payload_sha256_hex?: string;
  work_offer_signature_hex?: string;

  work_agreement_payload_cbor_hex?: string;
  work_agreement_payload_sha256_hex?: string;
  work_agreement_signature_hex?: string;

  work_offer_publish_payload_cbor_hex?: string;
  work_offer_publish_payload_sha256_hex?: string;
  work_agreement_publish_payload_cbor_hex?: string;
  work_agreement_publish_payload_sha256_hex?: string;
  work_agreement_update_payload_cbor_hex?: string;
  work_agreement_update_payload_sha256_hex?: string;
  work_agreement_close_payload_cbor_hex?: string;
  work_agreement_close_payload_sha256_hex?: string;

  kill_switch_payload_cbor_hex?: string;
  kill_switch_payload_sha256_hex?: string;
  kill_switch_signature_hex?: string;
  kill_switch_full_object_cbor_hex?: string;

  agentmail_payload_cbor_hex?: string;
  agentmail_payload_sha256_hex?: string;
  agentmail_signature_hex?: string;
  agentmail_full_object_cbor_hex?: string;
}

interface VectorsFile {
  ed25519_public_key_hex: string;
  vectors: VectorEntry[];
}

function decodeHex(value: string | undefined, field: string): Uint8Array {
  if (!value) throw new Error(`missing ${field}`);
  return Uint8Array.from(Buffer.from(value, "hex"));
}

function ensureRoundtrip(label: string, cborBytes: Uint8Array): void {
  const value = decodeCanonical(cborBytes);
  const encoded = encodeCanonical(value);
  if (Buffer.from(encoded).compare(Buffer.from(cborBytes)) !== 0) {
    throw new Error(`${label} canonical roundtrip mismatch`);
  }
}

function verifyHashAndSig(label: string, publicKey: Uint8Array, cborBytes: Uint8Array, expectedHash: Uint8Array, signature: Uint8Array): void {
  const digest = sha256(cborBytes);
  if (Buffer.from(digest).compare(Buffer.from(expectedHash)) !== 0) {
    throw new Error(`${label} hash mismatch`);
  }
  verifyEd25519Hash(publicKey, digest, signature);
}

type KillSwitchParts = {
  action: bigint;
  reason: string;
  ts: bigint;
  nonce: Uint8Array;
  signature?: Uint8Array;
};

function isCborMapValue(value: unknown): value is { entries: [unknown, unknown][] } {
  return typeof value === "object" && value !== null && "entries" in value;
}

function toBigInt(value: number | bigint): bigint {
  if (typeof value === "bigint") return value;
  if (!Number.isInteger(value)) throw new Error("invalid integer");
  return BigInt(value);
}

function parseKillSwitchMap(value: unknown): KillSwitchParts {
  if (!isCborMapValue(value)) {
    throw new Error("kill switch map must be cbor map");
  }
  let action: bigint | null = null;
  let reason: string | null = null;
  let ts: bigint | null = null;
  let nonce: Uint8Array | null = null;
  let signature: Uint8Array | undefined;

  for (const [key, val] of value.entries) {
    if (typeof key !== "number" && typeof key !== "bigint") continue;
    const keyBig = toBigInt(key);
    if (keyBig === 0n) {
      if (typeof val === "number" || typeof val === "bigint") {
        const valBig = toBigInt(val);
        if (valBig >= 0n && valBig <= 255n) {
          action = valBig;
        }
      }
    } else if (keyBig === 1n && typeof val === "string") {
      reason = val;
    } else if (keyBig === 2n) {
      if (typeof val === "number" || typeof val === "bigint") {
        const valBig = toBigInt(val);
        if (valBig >= 0n) ts = valBig;
      }
    } else if (keyBig === 3n && val instanceof Uint8Array) {
      nonce = val;
    } else if (keyBig === 4n && val instanceof Uint8Array) {
      signature = val;
    }
  }

  if (action === null) throw new Error("kill switch action missing");
  if (reason === null) throw new Error("kill switch reason missing");
  if (ts === null) throw new Error("kill switch ts missing");
  if (nonce === null) throw new Error("kill switch nonce missing");
  return { action, reason, ts, nonce, signature };
}

function ensureKillSwitchParts(label: string, parts: KillSwitchParts, requireSignature: boolean): void {
  if (parts.action !== 0n && parts.action !== 1n) {
    throw new Error(`${label} invalid action`);
  }
  if (parts.nonce.length !== 16) {
    throw new Error(`${label} nonce length invalid`);
  }
  if (requireSignature) {
    if (!parts.signature) throw new Error(`${label} signature missing`);
    if (parts.signature.length !== 64) throw new Error(`${label} signature length invalid`);
  }
}

function main(): void {
  const path = process.argv[2];
  if (!path) {
    throw new Error("usage: agentnet-vectors <path-to-vectors.json>");
  }
  const data = JSON.parse(readFileSync(path, "utf-8")) as VectorsFile;
  const publicKey = Uint8Array.from(Buffer.from(data.ed25519_public_key_hex, "hex"));

  let actionIntentHash: Uint8Array | null = null;

  for (const entry of data.vectors) {
    const id = entry.id;
    if (id === "TV1_ActionIntent") {
      const cbor = decodeHex(entry.object_cbor_hex, "object_cbor_hex");
      const expectedHash = decodeHex(entry.sha256_hex, "sha256_hex");
      const signature = decodeHex(entry.signature_hex, "signature_hex");
      verifyHashAndSig(id, publicKey, cbor, expectedHash, signature);
      ensureRoundtrip(id, cbor);
      actionIntentHash = expectedHash;
    } else if (id === "TV2_Approval") {
      const payload = decodeHex(entry.approval_payload_cbor_hex, "approval_payload_cbor_hex");
      const expectedHash = decodeHex(entry.approval_payload_sha256_hex, "approval_payload_sha256_hex");
      const signature = decodeHex(entry.approval_signature_hex, "approval_signature_hex");
      const intentHash = decodeHex(entry.intent_hash_hex, "intent_hash_hex");
      if (entry.approval_full_object_cbor_hex) {
        const full = decodeHex(entry.approval_full_object_cbor_hex, "approval_full_object_cbor_hex");
        ensureRoundtrip("TV2_Approval full object", full);
      }
      verifyHashAndSig(id, publicKey, payload, expectedHash, signature);
      ensureRoundtrip(id, payload);
      if (actionIntentHash && Buffer.from(intentHash).compare(Buffer.from(actionIntentHash)) !== 0) {
        throw new Error("TV2_Approval intent hash mismatch");
      }
    } else if (id === "TV3_Grant") {
      const payload = decodeHex(entry.grant_payload_cbor_hex, "grant_payload_cbor_hex");
      const expectedHash = decodeHex(entry.grant_payload_sha256_hex, "grant_payload_sha256_hex");
      const signature = decodeHex(entry.grant_signature_hex, "grant_signature_hex");
      if (entry.grant_full_object_cbor_hex) {
        const full = decodeHex(entry.grant_full_object_cbor_hex, "grant_full_object_cbor_hex");
        ensureRoundtrip("TV3_Grant full object", full);
      }
      verifyHashAndSig(id, publicKey, payload, expectedHash, signature);
      ensureRoundtrip(id, payload);
    } else if (id === "TV4_NodeHello") {
      const payload = decodeHex(entry.nodehello_payload_cbor_hex, "nodehello_payload_cbor_hex");
      const expectedHash = decodeHex(entry.nodehello_payload_sha256_hex, "nodehello_payload_sha256_hex");
      const signature = decodeHex(entry.nodehello_signature_hex, "nodehello_signature_hex");
      verifyHashAndSig(id, publicKey, payload, expectedHash, signature);
      ensureRoundtrip(id, payload);
    } else if (id === "TV5_ReceiptChain") {
      const receipt1 = decodeHex(entry.receipt1_payload_cbor_hex, "receipt1_payload_cbor_hex");
      const receipt1Hash = decodeHex(entry.receipt1_hash_hex, "receipt1_hash_hex");
      const receipt1Sig = decodeHex(entry.receipt1_sig_hex, "receipt1_sig_hex");
      const receipt2 = decodeHex(entry.receipt2_payload_cbor_hex, "receipt2_payload_cbor_hex");
      const receipt2Hash = decodeHex(entry.receipt2_hash_hex, "receipt2_hash_hex");
      const receipt2Sig = decodeHex(entry.receipt2_sig_hex, "receipt2_sig_hex");
      const receipt2Prev = decodeHex(entry.receipt2_prev_hash_hex, "receipt2_prev_hash_hex");
      verifyHashAndSig("TV5_ReceiptChain receipt1", publicKey, receipt1, receipt1Hash, receipt1Sig);
      verifyHashAndSig("TV5_ReceiptChain receipt2", publicKey, receipt2, receipt2Hash, receipt2Sig);
      if (Buffer.from(receipt2Prev).compare(Buffer.from(receipt1Hash)) !== 0) {
        throw new Error("TV5_ReceiptChain prev hash mismatch");
      }
      ensureRoundtrip("TV5_ReceiptChain receipt1", receipt1);
      ensureRoundtrip("TV5_ReceiptChain receipt2", receipt2);
    } else if (id === "TV6_EscrowLockTx") {
      const payload = decodeHex(entry.tx_envelope_payload_cbor_hex, "tx_envelope_payload_cbor_hex");
      const expectedHash = decodeHex(entry.tx_envelope_payload_sha256_hex, "tx_envelope_payload_sha256_hex");
      const signature = decodeHex(entry.tx_signature_hex, "tx_signature_hex");
      verifyHashAndSig(id, publicKey, payload, expectedHash, signature);
      ensureRoundtrip(id, payload);
    } else if (id === "TV7_KillSwitch") {
      const payload = decodeHex(entry.kill_switch_payload_cbor_hex, "kill_switch_payload_cbor_hex");
      const expectedHash = decodeHex(entry.kill_switch_payload_sha256_hex, "kill_switch_payload_sha256_hex");
      const signature = decodeHex(entry.kill_switch_signature_hex, "kill_switch_signature_hex");
      const full = decodeHex(entry.kill_switch_full_object_cbor_hex, "kill_switch_full_object_cbor_hex");
      verifyHashAndSig(id, publicKey, payload, expectedHash, signature);
      ensureRoundtrip("TV7_KillSwitch payload", payload);
      ensureRoundtrip("TV7_KillSwitch full object", full);

      const payloadValue = decodeCanonical(payload);
      const payloadParts = parseKillSwitchMap(payloadValue);
      if (payloadParts.signature) {
        throw new Error("TV7_KillSwitch payload must not include signature");
      }
      ensureKillSwitchParts("TV7_KillSwitch payload", payloadParts, false);

      const fullValue = decodeCanonical(full);
      const fullParts = parseKillSwitchMap(fullValue);
      ensureKillSwitchParts("TV7_KillSwitch full object", fullParts, true);
      if (!fullParts.signature || Buffer.from(fullParts.signature).compare(Buffer.from(signature)) !== 0) {
        throw new Error("TV7_KillSwitch signature mismatch");
      }
      if (
        payloadParts.action !== fullParts.action ||
        payloadParts.reason !== fullParts.reason ||
        payloadParts.ts !== fullParts.ts ||
        Buffer.from(payloadParts.nonce).compare(Buffer.from(fullParts.nonce)) !== 0
      ) {
        throw new Error("TV7_KillSwitch full object fields mismatch");
      }
      const reconstructed = encodeCanonical({
        entries: [
          [0n, payloadParts.action],
          [1n, payloadParts.reason],
          [2n, payloadParts.ts],
          [3n, payloadParts.nonce],
        ],
      });
      if (Buffer.from(reconstructed).compare(Buffer.from(payload)) !== 0) {
        throw new Error("TV7_KillSwitch payload reconstruction mismatch");
      }
    } else if (id === "TV8_SkillManifest") {
      const payload = decodeHex(entry.object_cbor_hex, "object_cbor_hex");
      const expectedHash = decodeHex(entry.sha256_hex, "sha256_hex");
      const signature = decodeHex(entry.signature_hex, "signature_hex");
      verifyHashAndSig(id, publicKey, payload, expectedHash, signature);
      ensureRoundtrip(id, payload);
      parseSkillManifestPayload(decodeCanonical(payload));
      if (entry.skill_manifest_full_object_cbor_hex) {
        const full = decodeHex(entry.skill_manifest_full_object_cbor_hex, "skill_manifest_full_object_cbor_hex");
        ensureRoundtrip("TV8_SkillManifest full object", full);
        verifySkillManifest(full, publicKey);
      }
    } else if (id === "TV9_WorkOffer") {
      const payload = decodeHex(entry.work_offer_payload_cbor_hex, "work_offer_payload_cbor_hex");
      const expectedHash = decodeHex(entry.work_offer_payload_sha256_hex, "work_offer_payload_sha256_hex");
      const signature = decodeHex(entry.work_offer_signature_hex, "work_offer_signature_hex");
      verifyHashAndSig(id, publicKey, payload, expectedHash, signature);
      ensureRoundtrip(id, payload);
      parseWorkOfferPayload(decodeCanonical(payload));
      if (entry.work_offer_full_object_cbor_hex) {
        const full = decodeHex(entry.work_offer_full_object_cbor_hex, "work_offer_full_object_cbor_hex");
        ensureRoundtrip("TV9_WorkOffer full object", full);
        verifyWorkOffer(full, publicKey);
      }
    } else if (id === "TV10_WorkAgreement") {
      const payload = decodeHex(entry.work_agreement_payload_cbor_hex, "work_agreement_payload_cbor_hex");
      const expectedHash = decodeHex(entry.work_agreement_payload_sha256_hex, "work_agreement_payload_sha256_hex");
      const signature = decodeHex(entry.work_agreement_signature_hex, "work_agreement_signature_hex");
      verifyHashAndSig(id, publicKey, payload, expectedHash, signature);
      ensureRoundtrip(id, payload);
      parseWorkAgreementPayload(decodeCanonical(payload));
      if (entry.work_agreement_full_object_cbor_hex) {
        const full = decodeHex(entry.work_agreement_full_object_cbor_hex, "work_agreement_full_object_cbor_hex");
        ensureRoundtrip("TV10_WorkAgreement full object", full);
        verifyWorkAgreement(full, publicKey);
      }
    } else if (id === "TV11_SkillPublishPayload") {
      const payload = decodeHex(entry.skill_publish_payload_cbor_hex, "skill_publish_payload_cbor_hex");
      const expectedHash = decodeHex(entry.skill_publish_payload_sha256_hex, "skill_publish_payload_sha256_hex");
      const digest = sha256(payload);
      if (Buffer.from(digest).compare(Buffer.from(expectedHash)) !== 0) {
        throw new Error("TV11_SkillPublishPayload hash mismatch");
      }
      ensureRoundtrip("TV11_SkillPublishPayload", payload);
      parseSkillPublishPayload(decodeCanonical(payload));
    } else if (id === "TV12_SkillUpdatePayload") {
      const payload = decodeHex(entry.skill_update_payload_cbor_hex, "skill_update_payload_cbor_hex");
      const expectedHash = decodeHex(entry.skill_update_payload_sha256_hex, "skill_update_payload_sha256_hex");
      const digest = sha256(payload);
      if (Buffer.from(digest).compare(Buffer.from(expectedHash)) !== 0) {
        throw new Error("TV12_SkillUpdatePayload hash mismatch");
      }
      ensureRoundtrip("TV12_SkillUpdatePayload", payload);
      parseSkillUpdatePayload(decodeCanonical(payload));
    } else if (id === "TV13_SkillRevokePayload") {
      const payload = decodeHex(entry.skill_revoke_payload_cbor_hex, "skill_revoke_payload_cbor_hex");
      const expectedHash = decodeHex(entry.skill_revoke_payload_sha256_hex, "skill_revoke_payload_sha256_hex");
      const digest = sha256(payload);
      if (Buffer.from(digest).compare(Buffer.from(expectedHash)) !== 0) {
        throw new Error("TV13_SkillRevokePayload hash mismatch");
      }
      ensureRoundtrip("TV13_SkillRevokePayload", payload);
      parseSkillRevokePayload(decodeCanonical(payload));
    } else if (id === "TV14_WorkOfferPublishPayload") {
      const payload = decodeHex(entry.work_offer_publish_payload_cbor_hex, "work_offer_publish_payload_cbor_hex");
      const expectedHash = decodeHex(entry.work_offer_publish_payload_sha256_hex, "work_offer_publish_payload_sha256_hex");
      const digest = sha256(payload);
      if (Buffer.from(digest).compare(Buffer.from(expectedHash)) !== 0) {
        throw new Error("TV14_WorkOfferPublishPayload hash mismatch");
      }
      ensureRoundtrip("TV14_WorkOfferPublishPayload", payload);
      parseWorkOfferPublishPayload(decodeCanonical(payload));
    } else if (id === "TV15_WorkAgreementPublishPayload") {
      const payload = decodeHex(entry.work_agreement_publish_payload_cbor_hex, "work_agreement_publish_payload_cbor_hex");
      const expectedHash = decodeHex(entry.work_agreement_publish_payload_sha256_hex, "work_agreement_publish_payload_sha256_hex");
      const digest = sha256(payload);
      if (Buffer.from(digest).compare(Buffer.from(expectedHash)) !== 0) {
        throw new Error("TV15_WorkAgreementPublishPayload hash mismatch");
      }
      ensureRoundtrip("TV15_WorkAgreementPublishPayload", payload);
      parseWorkAgreementPublishPayload(decodeCanonical(payload));
    } else if (id === "TV16_WorkAgreementUpdatePayload") {
      const payload = decodeHex(entry.work_agreement_update_payload_cbor_hex, "work_agreement_update_payload_cbor_hex");
      const expectedHash = decodeHex(entry.work_agreement_update_payload_sha256_hex, "work_agreement_update_payload_sha256_hex");
      const digest = sha256(payload);
      if (Buffer.from(digest).compare(Buffer.from(expectedHash)) !== 0) {
        throw new Error("TV16_WorkAgreementUpdatePayload hash mismatch");
      }
      ensureRoundtrip("TV16_WorkAgreementUpdatePayload", payload);
      parseWorkAgreementUpdatePayload(decodeCanonical(payload));
    } else if (id === "TV17_WorkAgreementClosePayload") {
      const payload = decodeHex(entry.work_agreement_close_payload_cbor_hex, "work_agreement_close_payload_cbor_hex");
      const expectedHash = decodeHex(entry.work_agreement_close_payload_sha256_hex, "work_agreement_close_payload_sha256_hex");
      const digest = sha256(payload);
      if (Buffer.from(digest).compare(Buffer.from(expectedHash)) !== 0) {
        throw new Error("TV17_WorkAgreementClosePayload hash mismatch");
      }
      ensureRoundtrip("TV17_WorkAgreementClosePayload", payload);
      parseWorkAgreementClosePayload(decodeCanonical(payload));
    } else if (id === "TV18_AgentMailMessage") {
      const payload = decodeHex(entry.agentmail_payload_cbor_hex, "agentmail_payload_cbor_hex");
      const expectedHash = decodeHex(entry.agentmail_payload_sha256_hex, "agentmail_payload_sha256_hex");
      const signature = decodeHex(entry.agentmail_signature_hex, "agentmail_signature_hex");
      const digest = sha256(payload);
      if (Buffer.from(digest).compare(Buffer.from(expectedHash)) !== 0) {
        throw new Error("TV18_AgentMailMessage hash mismatch");
      }
      verifyEd25519Hash(publicKey, digest, signature);
      ensureRoundtrip("TV18_AgentMailMessage payload", payload);
      parseAgentMailPayload(decodeCanonical(payload));
      if (entry.agentmail_full_object_cbor_hex) {
        const full = decodeHex(entry.agentmail_full_object_cbor_hex, "agentmail_full_object_cbor_hex");
        ensureRoundtrip("TV18_AgentMailMessage full object", full);
        const message = parseAgentMailMessage(decodeCanonical(full));
        if (Buffer.from(message.signature).compare(Buffer.from(signature)) !== 0) {
          throw new Error("TV18_AgentMailMessage signature mismatch");
        }
      }
    } else {
      throw new Error(`unknown vector id: ${id}`);
    }
  }

  console.log("vector verification complete");
}

main();
