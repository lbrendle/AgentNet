import { CborValue } from "./cbor.js";
export interface TxEnvelopePayload {
    txType: number;
    sender: string;
    nonce: number;
    fee: number;
    payload: CborValue;
}
export interface TxEnvelope {
    payload: TxEnvelopePayload;
    signature: Uint8Array;
}
export declare function parseTxEnvelopePayload(value: CborValue): TxEnvelopePayload;
export declare function parseTxEnvelope(value: CborValue): TxEnvelope;
export declare function decodeTxEnvelope(data: Uint8Array): TxEnvelope;
export declare function buildTxEnvelope(payload: TxEnvelopePayload, secretKey: Uint8Array): Uint8Array;
export declare function verifyTxEnvelope(data: Uint8Array, publicKey: Uint8Array): TxEnvelopePayload;
export declare function txEnvelopePayloadToCbor(payload: TxEnvelopePayload): CborValue;
