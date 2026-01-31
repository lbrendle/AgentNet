import { CborValue } from "./cbor.js";
export interface EscrowLockPayload {
    escrowId: string;
    payer: string;
    payee: string;
    amount: number;
    currency: string;
    releaseCondition: CborValue;
    disputeWindowSec: number;
    expiry: number;
}
export interface EscrowReleasePayload {
    escrowId: string;
    evidenceReceiptHash: Uint8Array;
    ts: number;
}
export interface EscrowDisputePayload {
    escrowId: string;
    reason: string;
    evidenceAnchorOrReceipt: Uint8Array;
    ts: number;
}
export interface EscrowResolvePayload {
    escrowId: string;
    outcome: number;
    splitAmountToPayee?: number;
    ts: number;
}
export declare function parseEscrowLockPayload(value: CborValue): EscrowLockPayload;
export declare function parseEscrowReleasePayload(value: CborValue): EscrowReleasePayload;
export declare function parseEscrowDisputePayload(value: CborValue): EscrowDisputePayload;
export declare function parseEscrowResolvePayload(value: CborValue): EscrowResolvePayload;
export declare function escrowLockPayloadToCbor(payload: EscrowLockPayload): CborValue;
export declare function escrowReleasePayloadToCbor(payload: EscrowReleasePayload): CborValue;
export declare function escrowDisputePayloadToCbor(payload: EscrowDisputePayload): CborValue;
export declare function escrowResolvePayloadToCbor(payload: EscrowResolvePayload): CborValue;
