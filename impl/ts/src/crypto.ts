import { sha256 as sha256hash } from "@noble/hashes/sha256";
import nacl from "tweetnacl";

export function sha256(data: Uint8Array): Uint8Array {
  return sha256hash(data);
}

export function verifyEd25519Hash(publicKey: Uint8Array, messageHash: Uint8Array, signature: Uint8Array): void {
  if (publicKey.length !== 32 || signature.length !== 64 || messageHash.length !== 32) {
    throw new Error("invalid signature");
  }
  const ok = nacl.sign.detached.verify(messageHash, signature, publicKey);
  if (!ok) {
    throw new Error("invalid signature");
  }
}

export function signEd25519Hash(secretKey: Uint8Array, messageHash: Uint8Array): Uint8Array {
  if (secretKey.length !== 32 || messageHash.length !== 32) {
    throw new Error("invalid signature inputs");
  }
  const keyPair = nacl.sign.keyPair.fromSeed(secretKey);
  return nacl.sign.detached(messageHash, keyPair.secretKey);
}
