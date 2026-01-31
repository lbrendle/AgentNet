import { CborValue } from "./cbor.js";
export interface SkillArtifact {
    kind: number;
    digest: Uint8Array;
    size: number;
    uris: string[];
}
export interface SkillManifestPayload {
    skillId: string;
    author: string;
    name: string;
    version: string;
    summary: string;
    license: string;
    capabilities: string[];
    permissions: string[];
    sandboxClass: number;
    endpoints?: string[];
    artifacts?: SkillArtifact[];
    requirements?: string[];
    pricing?: CborValue;
    attestations?: CborValue;
    metadata?: CborValue;
    ts: number;
}
export interface SkillManifest {
    payload: SkillManifestPayload;
    signature: Uint8Array;
}
export interface SkillPublishPayload {
    manifest: Uint8Array;
    ts: number;
}
export interface SkillUpdatePayload {
    skillId: string;
    prevManifestHash: Uint8Array;
    manifest: Uint8Array;
    ts: number;
}
export interface SkillRevokePayload {
    skillId: string;
    manifestHash: Uint8Array;
    reason: string;
    ts: number;
}
export declare function parseSkillManifestPayload(value: CborValue): SkillManifestPayload;
export declare function parseSkillManifest(value: CborValue): SkillManifest;
export declare function decodeSkillManifest(data: Uint8Array): SkillManifest;
export declare function buildSkillManifest(payload: SkillManifestPayload, secretKey: Uint8Array): Uint8Array;
export declare function verifySkillManifest(data: Uint8Array, publicKey: Uint8Array): SkillManifestPayload;
export declare function parseSkillPublishPayload(value: CborValue): SkillPublishPayload;
export declare function parseSkillUpdatePayload(value: CborValue): SkillUpdatePayload;
export declare function parseSkillRevokePayload(value: CborValue): SkillRevokePayload;
export declare function skillPublishPayloadToCbor(payload: SkillPublishPayload): CborValue;
export declare function skillUpdatePayloadToCbor(payload: SkillUpdatePayload): CborValue;
export declare function skillRevokePayloadToCbor(payload: SkillRevokePayload): CborValue;
