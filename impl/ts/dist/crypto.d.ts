export declare function sha256(data: Uint8Array): Uint8Array;
export declare function verifyEd25519Hash(publicKey: Uint8Array, messageHash: Uint8Array, signature: Uint8Array): void;
export declare function signEd25519Hash(secretKey: Uint8Array, messageHash: Uint8Array): Uint8Array;
