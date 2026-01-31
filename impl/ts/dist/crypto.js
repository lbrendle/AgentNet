import { sha256 as sha256hash } from "@noble/hashes/sha256";
import nacl from "tweetnacl";
export function sha256(data) {
    return sha256hash(data);
}
export function verifyEd25519Hash(publicKey, messageHash, signature) {
    if (publicKey.length !== 32 || signature.length !== 64 || messageHash.length !== 32) {
        throw new Error("invalid signature");
    }
    const ok = nacl.sign.detached.verify(messageHash, signature, publicKey);
    if (!ok) {
        throw new Error("invalid signature");
    }
}
export function signEd25519Hash(secretKey, messageHash) {
    if (secretKey.length !== 32 || messageHash.length !== 32) {
        throw new Error("invalid signature inputs");
    }
    const keyPair = nacl.sign.keyPair.fromSeed(secretKey);
    return nacl.sign.detached(messageHash, keyPair.secretKey);
}
