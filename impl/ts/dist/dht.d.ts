import { CborValue } from "./cbor.js";
export interface Contact {
    nodeIds: string[];
    addrs: string[];
}
export interface AgentRecordPayload {
    agentId: string;
    agentPubkeys: Uint8Array[];
    contact: Contact;
    capabilities: string[];
    expires: number;
}
export interface AgentRecord {
    payload: AgentRecordPayload;
    signature: Uint8Array;
}
export interface ServiceRecordPayload {
    providerId: string;
    serviceType: number;
    addrs: string[];
    requiredCredentials?: string[];
    pricing?: CborValue;
    expires: number;
}
export interface ServiceRecord {
    payload: ServiceRecordPayload;
    signature: Uint8Array;
}
export interface CommunityRecordPayload {
    communityId: string;
    controller: string;
    joinPolicy: number;
    requiredCredentials?: string[];
    economics: CborValue;
    governance: CborValue;
    expires: number;
}
export interface CommunityRecord {
    payload: CommunityRecordPayload;
    signature: Uint8Array;
}
export declare function parseContact(value: CborValue): Contact;
export declare function parseAgentRecordPayload(value: CborValue): AgentRecordPayload;
export declare function parseAgentRecord(value: CborValue): AgentRecord;
export declare function parseServiceRecordPayload(value: CborValue): ServiceRecordPayload;
export declare function parseServiceRecord(value: CborValue): ServiceRecord;
export declare function parseCommunityRecordPayload(value: CborValue): CommunityRecordPayload;
export declare function parseCommunityRecord(value: CborValue): CommunityRecord;
export declare function buildAgentRecord(payload: AgentRecordPayload, secretKey: Uint8Array): Uint8Array;
export declare function buildServiceRecord(payload: ServiceRecordPayload, secretKey: Uint8Array): Uint8Array;
export declare function buildCommunityRecord(payload: CommunityRecordPayload, secretKey: Uint8Array): Uint8Array;
export declare function verifyAgentRecord(data: Uint8Array, publicKey: Uint8Array): AgentRecordPayload;
export declare function verifyServiceRecord(data: Uint8Array, publicKey: Uint8Array): ServiceRecordPayload;
export declare function verifyCommunityRecord(data: Uint8Array, publicKey: Uint8Array): CommunityRecordPayload;
