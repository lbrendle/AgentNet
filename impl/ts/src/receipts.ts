import { decodeCanonical } from "./cbor.js";
import { sha256, verifyEd25519Hash } from "./crypto.js";
import { openSync, existsSync, writeFileSync, fstatSync, readSync, writeSync, fsyncSync, unlinkSync } from "node:fs";

export interface ReceiptRecord {
  payload: Uint8Array;
  receiptHash: Uint8Array;
  signature: Uint8Array;
}

export class ReceiptLog {
  private file: number;
  private lastHash: Uint8Array;
  private lastSeq: number;

  constructor(private path: string, file: number, lastHash: Uint8Array, lastSeq: number) {
    this.file = file;
    this.lastHash = lastHash;
    this.lastSeq = lastSeq;
  }

  static open(path: string): ReceiptLog {
    if (!existsSync(path)) {
      writeFileSync(path, Buffer.alloc(0));
    }
    const fd = openSync(path, "r+");
    const log = new ReceiptLog(path, fd, new Uint8Array(32), 0);
    log.replay();
    return log;
  }

  append(payload: Uint8Array, signature: Uint8Array): ReceiptRecord {
    return this.appendInternal(payload, signature, undefined);
  }

  appendVerified(payload: Uint8Array, signature: Uint8Array, publicKey: Uint8Array): ReceiptRecord {
    return this.appendInternal(payload, signature, publicKey);
  }

  getLastHash(): Uint8Array {
    return this.lastHash;
  }

  getLastSeq(): number {
    return this.lastSeq;
  }

  private replay(): void {
    let position = 0;
    const stats = fstatSync(this.file);
    while (position < stats.size) {
      const lenBuf = Buffer.alloc(4);
      readSync(this.file, lenBuf, 0, 4, position);
      position += 4;
      const payloadLen = lenBuf.readUInt32BE(0);
      const payload = Buffer.alloc(payloadLen);
      readSync(this.file, payload, 0, payloadLen, position);
      position += payloadLen;
      const sigLenBuf = Buffer.alloc(4);
      readSync(this.file, sigLenBuf, 0, 4, position);
      position += 4;
      const sigLen = sigLenBuf.readUInt32BE(0);
      const signature = Buffer.alloc(sigLen);
      readSync(this.file, signature, 0, sigLen, position);
      position += sigLen;

      const receipt = parseReceiptPayload(payload);
      if (receipt.seq !== this.lastSeq + 1) {
        throw new Error("receipt sequence mismatch");
      }
      if (Buffer.from(receipt.prevHash).compare(Buffer.from(this.lastHash)) !== 0) {
        throw new Error("receipt prev_hash mismatch");
      }
      this.lastHash = sha256(payload);
      this.lastSeq = receipt.seq;
    }
  }

  private appendInternal(payload: Uint8Array, signature: Uint8Array, publicKey?: Uint8Array): ReceiptRecord {
    const receipt = parseReceiptPayload(payload);
    if (receipt.seq !== this.lastSeq + 1) {
      throw new Error("receipt sequence mismatch");
    }
    if (Buffer.from(receipt.prevHash).compare(Buffer.from(this.lastHash)) !== 0) {
      throw new Error("receipt prev_hash mismatch");
    }
    const receiptHash = sha256(payload);
    if (publicKey) {
      verifyEd25519Hash(publicKey, receiptHash, signature);
    }
    this.writeRecord(payload, signature);
    this.lastHash = receiptHash;
    this.lastSeq = receipt.seq;
    return { payload, receiptHash, signature };
  }

  private writeRecord(payload: Uint8Array, signature: Uint8Array): void {
    const lenBuf = Buffer.alloc(4);
    lenBuf.writeUInt32BE(payload.length, 0);
    writeSync(this.file, lenBuf);
    writeSync(this.file, Buffer.from(payload));
    const sigLenBuf = Buffer.alloc(4);
    sigLenBuf.writeUInt32BE(signature.length, 0);
    writeSync(this.file, sigLenBuf);
    writeSync(this.file, Buffer.from(signature));
    fsyncSync(this.file);
  }
}

function parseReceiptPayload(payload: Uint8Array): { prevHash: Uint8Array; seq: number } {
  const decoded = decodeCanonical(payload);
  if (typeof decoded !== "object" || decoded === null || !("entries" in decoded)) {
    throw new Error("expected receipt payload map");
  }
  const entries = (decoded as any).entries as [any, any][];
  let prevHash: Uint8Array | null = null;
  let seq: number | null = null;
  for (const [k, v] of entries) {
    const key = typeof k === "bigint" ? Number(k) : k;
    if (typeof key === "number" && key === 8) {
      if (!(v instanceof Uint8Array)) throw new Error("prev_hash must be bytes");
      prevHash = v;
    }
    if (typeof key === "number" && key === 9) {
      if (typeof v === "bigint") {
        const asNumber = Number(v);
        if (!Number.isSafeInteger(asNumber)) throw new Error("seq must be safe int");
        seq = asNumber;
      } else if (typeof v === "number") {
        seq = v;
      } else {
        throw new Error("seq must be int");
      }
    }
  }
  if (prevHash === null || seq === null) {
    throw new Error("missing receipt fields");
  }
  return { prevHash, seq };
}
