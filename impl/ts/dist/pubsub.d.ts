import { CborValue } from "./cbor.js";
export interface EconomicProof {
    kind: number;
    data: Uint8Array;
}
export interface PubSubEnvelopePayload {
    version: number;
    topic: string;
    sender: string;
    ts: number;
    seq: number;
    payloadType: number;
    payload: CborValue;
    economicProof?: EconomicProof;
}
export interface PubSubEnvelope {
    payload: PubSubEnvelopePayload;
    signature: Uint8Array;
}
export declare function economicProofOnChain(txHash: Uint8Array): EconomicProof;
export declare function economicProofVoucher(voucher: Uint8Array): EconomicProof;
export declare function parsePubSubPayload(value: CborValue): PubSubEnvelopePayload;
export declare function parsePubSubEnvelope(value: CborValue): PubSubEnvelope;
export declare function decodePubSubEnvelope(data: Uint8Array): PubSubEnvelope;
export declare function buildPubSubEnvelope(payload: PubSubEnvelopePayload, secretKey: Uint8Array): Uint8Array;
export declare function verifyPubSubEnvelope(data: Uint8Array, publicKey: Uint8Array): PubSubEnvelopePayload;
