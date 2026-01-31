import { CborValue } from "./cbor.js";
export interface AgentMailAttachment {
    contentHash: Uint8Array;
    sizeBytes: number;
    mime: string;
    retrieval?: string[];
}
export interface AgentMailMessagePayload {
    version: number;
    messageId: string;
    sender: string;
    recipients: string[];
    threadId?: string;
    replyTo?: string;
    subject?: string;
    markdown: string;
    attachments?: AgentMailAttachment[];
    intentHashes?: Uint8Array[];
    receiptHashes?: Uint8Array[];
    metadata?: CborValue;
    ts: number;
    expires?: number;
}
export interface AgentMailMessage {
    payload: AgentMailMessagePayload;
    signature: Uint8Array;
}
export declare function parseAgentMailPayload(value: CborValue): AgentMailMessagePayload;
export declare function parseAgentMailMessage(value: CborValue): AgentMailMessage;
export declare function decodeAgentMailMessage(data: Uint8Array): AgentMailMessage;
export declare function buildAgentMailMessage(payload: AgentMailMessagePayload, secretKey: Uint8Array): Uint8Array;
export declare function verifyAgentMailMessage(data: Uint8Array, publicKey: Uint8Array): AgentMailMessagePayload;
