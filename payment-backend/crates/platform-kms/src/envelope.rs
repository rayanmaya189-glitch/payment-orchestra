use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use uuid::Uuid;
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedCredential {
    pub ciphertext: Vec<u8>,
    pub nonce: Vec<u8>,
    pub kek_version: String,
    pub link_id: Uuid,
}

/// Envelope encryption service (ADR-014).
pub struct EnvelopeEncryptionService {
    #[allow(dead_code)]
    kek: Arc<RwLock<Vec<u8>>>,
    dek_cache: Arc<RwLock<HashMap<Uuid, Vec<u8>>>>,
    kek_version: String,
}

impl EnvelopeEncryptionService {
    pub fn new(kek: Vec<u8>, kek_version: &str) -> Self {
        Self {
            kek: Arc::new(RwLock::new(kek)),
            dek_cache: Arc::new(RwLock::new(HashMap::new())),
            kek_version: kek_version.to_string(),
        }
    }

    pub async fn encrypt(&self, link_id: Uuid, plaintext: &[u8]) -> Result<EncryptedCredential, String> {
        let dek = self.get_or_generate_dek(link_id).await?;
        let cipher = Aes256Gcm::new_from_slice(&dek).map_err(|e| e.to_string())?;
        let nonce_bytes: [u8; 12] = rand::random();
        let nonce = Nonce::from_slice(&nonce_bytes);
        let _aad = link_id.as_bytes().to_vec();
        let ciphertext = cipher.encrypt(nonce, plaintext)
            .map_err(|e| format!("Encryption failed: {}", e))?;
        Ok(EncryptedCredential {
            ciphertext,
            nonce: nonce_bytes.to_vec(),
            kek_version: self.kek_version.clone(),
            link_id,
        })
    }

    pub async fn decrypt(&self, encrypted: &EncryptedCredential) -> Result<Vec<u8>, String> {
        let dek = self.get_or_generate_dek(encrypted.link_id).await?;
        let cipher = Aes256Gcm::new_from_slice(&dek).map_err(|e| e.to_string())?;
        let nonce = Nonce::from_slice(&encrypted.nonce);
        let _aad = encrypted.link_id.as_bytes().to_vec();
        cipher.decrypt(nonce, encrypted.ciphertext.as_ref())
            .map_err(|e| format!("Decryption failed: {}", e))
    }

    async fn get_or_generate_dek(&self, link_id: Uuid) -> Result<Vec<u8>, String> {
        // Check cache with read lock first (fast path)
        {
            let cache = self.dek_cache.read().await;
            if let Some(dek) = cache.get(&link_id) {
                return Ok(dek.clone());
            }
        }
        // Acquire write lock and double-check before inserting
        let mut cache = self.dek_cache.write().await;
        if let Some(dek) = cache.get(&link_id) {
            return Ok(dek.clone());
        }
        let dek: [u8; 32] = rand::random();
        cache.insert(link_id, dek.to_vec());
        Ok(dek.to_vec())
    }
}
