use aes_gcm::aead::{Aead, OsRng};
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use anyhow::{anyhow, Result};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use rand::RngCore;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EncryptedBlob {
    pub nonce: String,
    pub ciphertext: String,
}

#[derive(Clone)]
pub struct SecretBox {
    key: [u8; 32],
}

impl SecretBox {
    pub fn new_for_tests(key: [u8; 32]) -> Self {
        Self { key }
    }

    pub fn encrypt(&self, plaintext: &[u8]) -> Result<EncryptedBlob> {
        let cipher = Aes256Gcm::new_from_slice(&self.key)
            .map_err(|err| anyhow!("invalid encryption key: {err}"))?;
        let mut nonce_bytes = [0_u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|err| anyhow!("encrypt failed: {err}"))?;

        Ok(EncryptedBlob {
            nonce: STANDARD.encode(nonce_bytes),
            ciphertext: STANDARD.encode(ciphertext),
        })
    }

    pub fn decrypt(&self, blob: &EncryptedBlob) -> Result<Vec<u8>> {
        let cipher = Aes256Gcm::new_from_slice(&self.key)
            .map_err(|err| anyhow!("invalid encryption key: {err}"))?;
        let nonce_bytes = STANDARD.decode(&blob.nonce)?;
        let ciphertext = STANDARD.decode(&blob.ciphertext)?;
        let nonce = Nonce::from_slice(&nonce_bytes);

        cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|err| anyhow!("decrypt failed: {err}"))
    }
}
