use crate::Error;
use ed25519_dalek::{Signature, Signer, SigningKey};

pub fn sign_ed25519_hash(secret_key: &[u8], message_hash: &[u8]) -> Result<Vec<u8>, Error> {
    if secret_key.len() != 32 || message_hash.len() != 32 {
        return Err(Error::InvalidSignature);
    }
    let sk_bytes: [u8; 32] = secret_key.try_into().map_err(|_| Error::InvalidSignature)?;
    let signing_key = SigningKey::from_bytes(&sk_bytes);
    let sig: Signature = signing_key.sign(message_hash);
    Ok(sig.to_bytes().to_vec())
}
