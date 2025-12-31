use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, AeadCore, KeyInit, OsRng},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),
    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),
    #[error("Invalid key length: expected 32 bytes")]
    InvalidKeyLength,
    #[error("Base64 decode error: {0}")]
    Base64Error(#[from] base64::DecodeError),
}

/// Encripta un connection string usando AES-256-GCM
pub fn encrypt(plaintext: &str, key: &[u8]) -> Result<Vec<u8>, CryptoError> {
    if key.len() != 32 {
        return Err(CryptoError::InvalidKeyLength);
    }

    let cipher =
        Aes256Gcm::new_from_slice(key).map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;

    // Generate random nonce
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;

    // Prepend nonce to ciphertext
    let mut result = nonce.to_vec();
    result.extend_from_slice(&ciphertext);

    Ok(result)
}

/// Desencripta un connection string
pub fn decrypt(ciphertext: &[u8], key: &[u8]) -> Result<String, CryptoError> {
    if key.len() != 32 {
        return Err(CryptoError::InvalidKeyLength);
    }

    if ciphertext.len() < 12 {
        return Err(CryptoError::DecryptionFailed(
            "Ciphertext too short".to_string(),
        ));
    }

    let cipher =
        Aes256Gcm::new_from_slice(key).map_err(|e| CryptoError::DecryptionFailed(e.to_string()))?;

    // Extract nonce (first 12 bytes)
    let (nonce_bytes, encrypted_data) = ciphertext.split_at(12);

    // Convertir a array de 12 bytes
    let nonce_array: [u8; 12] = nonce_bytes
        .try_into()
        .map_err(|_| CryptoError::DecryptionFailed("Invalid nonce size".to_string()))?;
    let nonce = Nonce::from(nonce_array);

    let plaintext = cipher
        .decrypt(&nonce, encrypted_data)
        .map_err(|e| CryptoError::DecryptionFailed(e.to_string()))?;

    String::from_utf8(plaintext)
        .map_err(|e| CryptoError::DecryptionFailed(format!("Invalid UTF-8: {}", e)))
}

/// Encripta y codifica en base64
pub fn encrypt_base64(plaintext: &str, key: &[u8]) -> Result<String, CryptoError> {
    let encrypted = encrypt(plaintext, key)?;
    Ok(STANDARD.encode(encrypted))
}

/// Decodifica de base64 y desencripta
pub fn decrypt_base64(base64_ciphertext: &str, key: &[u8]) -> Result<String, CryptoError> {
    let ciphertext = STANDARD.decode(base64_ciphertext)?;
    decrypt(&ciphertext, key)
}

/// Genera una clave de 32 bytes desde una password usando PBKDF2
pub fn derive_key_from_password(password: &str, salt: &[u8]) -> [u8; 32] {
    use ring::pbkdf2;
    use std::num::NonZeroU32;

    let iterations = NonZeroU32::new(100_000).unwrap();
    let mut key = [0u8; 32];

    pbkdf2::derive(
        pbkdf2::PBKDF2_HMAC_SHA256,
        iterations,
        salt,
        password.as_bytes(),
        &mut key,
    );

    key
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let key = [0u8; 32];
        let plaintext = "postgresql://user:pass@host:5432/db";

        let ciphertext = encrypt(plaintext, &key).unwrap();
        let decrypted = decrypt(&ciphertext, &key).unwrap();

        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_encrypt_decrypt_base64() {
        let key = derive_key_from_password("my-secret-key", b"salt12345678");
        let plaintext = "postgresql://user:pass@host:5432/db";

        let encrypted = encrypt_base64(plaintext, &key).unwrap();
        let decrypted = decrypt_base64(&encrypted, &key).unwrap();

        assert_eq!(plaintext, decrypted);
    }
}
