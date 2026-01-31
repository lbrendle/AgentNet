import { CborValue } from "./cbor.js";
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
export declare function parseWorkOfferPayload(value: CborValue): WorkOfferPayload;
export declare function parseWorkOffer(value: CborValue): WorkOffer;
export declare function decodeWorkOffer(data: Uint8Array): WorkOffer;
export declare function buildWorkOffer(payload: WorkOfferPayload, secretKey: Uint8Array): Uint8Array;
export declare function verifyWorkOffer(data: Uint8Array, publicKey: Uint8Array): WorkOfferPayload;
export declare function parseWorkAgreementPayload(value: CborValue): WorkAgreementPayload;
export declare function parseWorkAgreement(value: CborValue): WorkAgreement;
export declare function decodeWorkAgreement(data: Uint8Array): WorkAgreement;
export declare function buildWorkAgreement(payload: WorkAgreementPayload, secretKey: Uint8Array): Uint8Array;
export declare function verifyWorkAgreement(data: Uint8Array, publicKey: Uint8Array): WorkAgreementPayload;
export declare function parseWorkOfferPublishPayload(value: CborValue): WorkOfferPublishPayload;
export declare function parseWorkAgreementPublishPayload(value: CborValue): WorkAgreementPublishPayload;
export declare function parseWorkAgreementUpdatePayload(value: CborValue): WorkAgreementUpdatePayload;
export declare function parseWorkAgreementClosePayload(value: CborValue): WorkAgreementClosePayload;
export declare function workOfferPublishPayloadToCbor(payload: WorkOfferPublishPayload): CborValue;
export declare function workAgreementPublishPayloadToCbor(payload: WorkAgreementPublishPayload): CborValue;
export declare function workAgreementUpdatePayloadToCbor(payload: WorkAgreementUpdatePayload): CborValue;
export declare function workAgreementClosePayloadToCbor(payload: WorkAgreementClosePayload): CborValue;
