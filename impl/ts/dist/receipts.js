import { decodeCanonical } from "./cbor.js";
import { sha256, verifyEd25519Hash } from "./crypto.js";
import { openSync, existsSync, writeFileSync, fstatSync, readSync, writeSync, fsyncSync } from "node:fs";
export class ReceiptLog {
    path;
    file;
    lastHash;
    lastSeq;
    constructor(path, file, lastHash, lastSeq) {
        this.path = path;
        this.file = file;
        this.lastHash = lastHash;
        this.lastSeq = lastSeq;
    }
    static open(path) {
        if (!existsSync(path)) {
            writeFileSync(path, Buffer.alloc(0));
        }
        const fd = openSync(path, "r+");
        const log = new ReceiptLog(path, fd, new Uint8Array(32), 0);
        log.replay();
        return log;
    }
    append(payload, signature) {
        return this.appendInternal(payload, signature, undefined);
    }
    appendVerified(payload, signature, publicKey) {
        return this.appendInternal(payload, signature, publicKey);
    }
    getLastHash() {
        return this.lastHash;
    }
    getLastSeq() {
        return this.lastSeq;
    }
    replay() {
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
    appendInternal(payload, signature, publicKey) {
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
    writeRecord(payload, signature) {
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
function parseReceiptPayload(payload) {
    const decoded = decodeCanonical(payload);
    if (typeof decoded !== "object" || decoded === null || !("entries" in decoded)) {
        throw new Error("expected receipt payload map");
    }
    const entries = decoded.entries;
    let prevHash = null;
    let seq = null;
    for (const [k, v] of entries) {
        const key = typeof k === "bigint" ? Number(k) : k;
        if (typeof key === "number" && key === 8) {
            if (!(v instanceof Uint8Array))
                throw new Error("prev_hash must be bytes");
            prevHash = v;
        }
        if (typeof key === "number" && key === 9) {
            if (typeof v === "bigint") {
                const asNumber = Number(v);
                if (!Number.isSafeInteger(asNumber))
                    throw new Error("seq must be safe int");
                seq = asNumber;
            }
            else if (typeof v === "number") {
                seq = v;
            }
            else {
                throw new Error("seq must be int");
            }
        }
    }
    if (prevHash === null || seq === null) {
        throw new Error("missing receipt fields");
    }
    return { prevHash, seq };
}
