import { CborValue } from "./cbor.js";
export interface TransferPayload {
    fromDid: string;
    toDid: string;
    amount: number;
    currency: string;
    ts: number;
}
export interface PostagePayload {
    payer: string;
    amount: number;
    currency: string;
    purpose: string;
    ts: number;
}
export declare function parseTransferPayload(value: CborValue): TransferPayload;
export declare function parsePostagePayload(value: CborValue): PostagePayload;
export declare function transferPayloadToCbor(payload: TransferPayload): CborValue;
export declare function postagePayloadToCbor(payload: PostagePayload): CborValue;
