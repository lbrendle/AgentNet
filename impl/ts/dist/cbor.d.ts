export type CborValue = bigint | number | string | Uint8Array | CborValue[] | CborMap | boolean | null;
export interface CborMap {
    entries: [CborValue, CborValue][];
}
export declare class CborError extends Error {
    constructor(message: string);
}
export declare function decodeCanonical(data: Uint8Array): CborValue;
export declare function encodeCanonical(value: CborValue): Uint8Array;
