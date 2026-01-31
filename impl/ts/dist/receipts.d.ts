export interface ReceiptRecord {
    payload: Uint8Array;
    receiptHash: Uint8Array;
    signature: Uint8Array;
}
export declare class ReceiptLog {
    private path;
    private file;
    private lastHash;
    private lastSeq;
    constructor(path: string, file: number, lastHash: Uint8Array, lastSeq: number);
    static open(path: string): ReceiptLog;
    append(payload: Uint8Array, signature: Uint8Array): ReceiptRecord;
    appendVerified(payload: Uint8Array, signature: Uint8Array, publicKey: Uint8Array): ReceiptRecord;
    getLastHash(): Uint8Array;
    getLastSeq(): number;
    private replay;
    private appendInternal;
    private writeRecord;
}
