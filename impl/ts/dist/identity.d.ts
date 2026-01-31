import { CborValue } from "./cbor.js";
export interface IdentityRegisterPayload {
    agentId: string;
    pkEd25519: Uint8Array;
    pkX25519: Uint8Array;
    created: number;
}
export interface IdentityRotatePayload {
    agentId: string;
    pkEd25519: Uint8Array;
    pkX25519: Uint8Array;
    ts: number;
}
export interface CredentialRevokePayload {
    issuer: string;
    credentialIdHash: Uint8Array;
    ts: number;
}
export declare function parseIdentityRegisterPayload(value: CborValue): IdentityRegisterPayload;
export declare function parseIdentityRotatePayload(value: CborValue): IdentityRotatePayload;
export declare function parseCredentialRevokePayload(value: CborValue): CredentialRevokePayload;
export declare function identityRegisterPayloadToCbor(payload: IdentityRegisterPayload): CborValue;
export declare function identityRotatePayloadToCbor(payload: IdentityRotatePayload): CborValue;
export declare function credentialRevokePayloadToCbor(payload: CredentialRevokePayload): CborValue;
