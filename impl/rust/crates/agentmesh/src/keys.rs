use anyhow::{Context, Result};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use rand::RngCore;
use std::fs;
use std::path::Path;

pub struct KeyMaterial {
    pub signing_key: SigningKey,
    pub verifying_key: VerifyingKey,
}

pub fn load_keypair(path: &Path) -> Result<KeyMaterial> {
    let data = fs::read_to_string(path).with_context(|| format!("read key {}", path.display()))?;
    let decoded = BASE64.decode(data.trim()).context("decode base64 key")?;
    if decoded.len() != 32 {
        anyhow::bail!("ed25519 secret must be 32 bytes");
    }
    let mut sk_bytes = [0u8; 32];
    sk_bytes.copy_from_slice(&decoded);
    let signing_key = SigningKey::from_bytes(&sk_bytes);
    let verifying_key = signing_key.verifying_key();
    Ok(KeyMaterial { signing_key, verifying_key })
}

pub fn generate_keypair() -> KeyMaterial {
    let mut rng = OsRng;
    let mut secret = [0u8; 32];
    rng.fill_bytes(&mut secret);
    let signing_key = SigningKey::from_bytes(&secret);
    let verifying_key = signing_key.verifying_key();
    KeyMaterial { signing_key, verifying_key }
}

pub fn write_secret_key(path: &Path, signing_key: &SigningKey) -> Result<()> {
    let secret = signing_key.to_bytes();
    let encoded = BASE64.encode(secret);
    fs::write(path, format!("{}\n", encoded)).with_context(|| format!("write key {}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
            .with_context(|| format!("chmod key {}", path.display()))?;
    }
    Ok(())
}
