import { readFileSync } from "node:fs";
import { decodeCanonical, encodeCanonical } from "./cbor.js";
import { sha256, verifyEd25519Hash } from "./crypto.js";

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
    } else {
      throw new Error(`unknown vector id: ${id}`);
    }
  }

  console.log("vector verification complete");
}

main();
