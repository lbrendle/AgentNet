use crate::Error;
use ed25519_dalek::{Signature, VerifyingKey};
use sha2::{Digest, Sha256};

pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&result);
    out
}

pub fn verify_ed25519_hash(
    public_key: &[u8],
    message_hash: &[u8],
    signature: &[u8],
) -> Result<(), Error> {
    if public_key.len() != 32 || signature.len() != 64 || message_hash.len() != 32 {
        return Err(Error::InvalidSignature);
    }
    let pk_bytes: [u8; 32] = public_key.try_into().map_err(|_| Error::InvalidSignature)?;
    let sig_bytes: [u8; 64] = signature.try_into().map_err(|_| Error::InvalidSignature)?;
    let vk = VerifyingKey::from_bytes(&pk_bytes).map_err(|_| Error::InvalidSignature)?;
    let sig = Signature::from_bytes(&sig_bytes);
    vk.verify_strict(message_hash, &sig)
        .map_err(|_| Error::InvalidSignature)
}
