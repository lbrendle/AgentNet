import { readFileSync, unlinkSync } from "node:fs";
import { ReceiptLog } from "./receipts.js";

function main(): void {
  const vectors = JSON.parse(readFileSync("../../spec/agentnet-test-vectors-v0.1.json", "utf-8"));
  const entry = vectors.vectors.find((v: any) => v.id === "TV5_ReceiptChain");
  if (!entry) throw new Error("missing receipt vector");
  const receipt1 = Buffer.from(entry.receipt1_payload_cbor_hex, "hex");
  const receipt2 = Buffer.from(entry.receipt2_payload_cbor_hex, "hex");
  const sig1 = Buffer.from(entry.receipt1_sig_hex, "hex");
  const sig2 = Buffer.from(entry.receipt2_sig_hex, "hex");

  const path = "/tmp/agentnet_receipts_ts.log";
  try {
    unlinkSync(path);
  } catch {
    // ignore
  }
  const log = ReceiptLog.open(path);
  log.append(receipt1, sig1);
  log.append(receipt2, sig2);

  const log2 = ReceiptLog.open(path);
  if (log2.getLastSeq() !== 2) {
    throw new Error("receipt log replay failed");
  }
  console.log("receipt log verification complete");
}

main();
