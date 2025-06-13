use aes_gcm::{aead::{Aead, KeyInit, AeadCore}, Aes256Gcm};
use aes_gcm::aead::generic_array::GenericArray;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use argon2::{self, password_hash::{SaltString, rand_core::OsRng}};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use thiserror::Error;
use zeroize::Zeroize;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum EncryptionError {
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),
    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),
    #[error("Invalid key length")]
    InvalidKeyLength,
    #[error("Invalid data length")]
    InvalidDataLength,
    #[error("Hash error: {0}")]
    HashError(String),
}

// Secure key derivation using Argon2
#[allow(dead_code)]
pub fn derive_key(password: &str, salt: Option<&[u8]>) -> Result<Vec<u8>, EncryptionError> {
    let salt = match salt {
        Some(s) => SaltString::encode_b64(s).map_err(|e| EncryptionError::HashError(e.to_string()))?,
        None => SaltString::generate(&mut OsRng),
    };

    let argon2 = argon2::Argon2::default();
    let password_hash = argon2::PasswordHash::generate(
        argon2,
        password.as_bytes(),
        &salt,
    ).map_err(|e| EncryptionError::HashError(e.to_string()))?;

    Ok(password_hash.hash.unwrap().as_bytes().to_vec())
}

// ChaCha20-Poly1305 encryption (considered more secure than AES-GCM in some contexts)
#[allow(dead_code)]
pub fn encrypt_chacha20(data: &[u8], key: &[u8]) -> Result<Vec<u8>, EncryptionError> {
    let key = Key::from_slice(key);
    let cipher = ChaCha20Poly1305::new(key);
    let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
    
    let encrypted = cipher.encrypt(&nonce, data)
        .map_err(|e| EncryptionError::EncryptionFailed(e.to_string()))?;
    
    let mut result = nonce.to_vec();
    result.extend_from_slice(&encrypted);
    Ok(result)
}

// ChaCha20-Poly1305 decryption
#[allow(dead_code)]
pub fn decrypt_chacha20(data: &[u8], key: &[u8]) -> Result<Vec<u8>, EncryptionError> {
    if data.len() < 12 {
        return Err(EncryptionError::InvalidDataLength);
    }

    let key = Key::from_slice(key);
    let cipher = ChaCha20Poly1305::new(key);
    
    let nonce = Nonce::from_slice(&data[..12]);
    let ciphertext = &data[12..];
    
    cipher.decrypt(nonce, ciphertext)
        .map_err(|e| EncryptionError::DecryptionFailed(e.to_string()))
}

// Enhanced AES-GCM encryption with better key handling
#[allow(dead_code)]
pub fn encrypt_aes_gcm(data: &[u8], key: &[u8]) -> Result<Vec<u8>, EncryptionError> {
    if key.len() < 32 {
        return Err(EncryptionError::InvalidKeyLength);
    }

    let key = GenericArray::clone_from_slice(&key[..32]);
    let cipher = Aes256Gcm::new(&key);
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    
    let encrypted = cipher.encrypt(&nonce, data)
        .map_err(|e| EncryptionError::EncryptionFailed(e.to_string()))?;
    
    let mut result = nonce.to_vec();
    result.extend_from_slice(&encrypted);
    Ok(result)
}

// Enhanced AES-GCM decryption with better error handling
#[allow(dead_code)]
pub fn decrypt_aes_gcm(data: &[u8], key: &[u8]) -> Result<Vec<u8>, EncryptionError> {
    if data.len() < 12 {
        return Err(EncryptionError::InvalidDataLength);
    }
    if key.len() < 32 {
        return Err(EncryptionError::InvalidKeyLength);
    }

    let key = GenericArray::clone_from_slice(&key[..32]);
    let cipher = Aes256Gcm::new(&key);
    
    let nonce = GenericArray::from_slice(&data[..12]);
    let ciphertext = &data[12..];
    
    cipher.decrypt(nonce, ciphertext)
        .map_err(|e| EncryptionError::DecryptionFailed(e.to_string()))
}

// Secure string encryption with automatic key derivation
#[allow(dead_code)]
pub fn encrypt_string(data: &str, password: &str) -> Result<String, EncryptionError> {
    let key = derive_key(password, None)?;
    let encrypted = encrypt_chacha20(data.as_bytes(), &key)?;
    Ok(STANDARD.encode(&encrypted))
}

// Secure string decryption with automatic key derivation
#[allow(dead_code)]
pub fn decrypt_string(encrypted: &str, password: &str) -> Result<String, EncryptionError> {
    let key = derive_key(password, None)?;
    let decoded = STANDARD.decode(encrypted)
        .map_err(|e| EncryptionError::DecryptionFailed(e.to_string()))?;
    let decrypted = decrypt_chacha20(&decoded, &key)?;
    String::from_utf8(decrypted)
        .map_err(|e| EncryptionError::DecryptionFailed(e.to_string()))
}

// Secure file encryption
#[allow(dead_code)]
pub fn encrypt_file(data: &[u8], password: &str) -> Result<Vec<u8>, EncryptionError> {
    let key = derive_key(password, None)?;
    encrypt_aes_gcm(data, &key)
}

// Secure file decryption
#[allow(dead_code)]
pub fn decrypt_file(data: &[u8], password: &str) -> Result<Vec<u8>, EncryptionError> {
    let key = derive_key(password, None)?;
    decrypt_aes_gcm(data, &key)
}

// Secure memory wiping for sensitive data
#[allow(dead_code)]
pub fn secure_wipe<T: Zeroize>(mut data: T) {
    data.zeroize();
}

// XOR-based encoding with base64
pub fn z26(s: String) -> String {
    let key = b"xai_stealth_key_";
    let mut result = Vec::with_capacity(s.len());
    
    for (i, byte) in s.bytes().enumerate() {
        result.push(byte ^ key[i % key.len()]);
    }
    
    STANDARD.encode(&result)
}

// XOR-based decoding from base64
pub fn aa27(s: String) -> String {
    if let Ok(decoded) = STANDARD.decode(&s) {
        let key = b"xai_stealth_key_";
        let mut result = Vec::with_capacity(decoded.len());
        
        for (i, byte) in decoded.iter().enumerate() {
            result.push(byte ^ key[i % key.len()]);
        }
        
        String::from_utf8(result).unwrap_or_default()
    } else {
        String::new()
    }
}

// AES-GCM encryption for binary data
pub fn ad30(data: &[u8], key: &str) -> Result<Vec<u8>, anyhow::Error> {
    let key_bytes = key.as_bytes();
    let key_bytes = &key_bytes[..16.min(key_bytes.len())];
    let padded_key = if key_bytes.len() < 32 {
        let mut padded = Vec::from(key_bytes);
        padded.resize(32, 0);
        padded
    } else {
        Vec::from(&key_bytes[..32])
    };
    
    let key = GenericArray::clone_from_slice(&padded_key);
    let cipher = Aes256Gcm::new(&key);
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    
    let encrypted = cipher.encrypt(&nonce, data)
        .map_err(|_| anyhow::Error::msg("encryption failed"))?;
    let mut result = nonce.to_vec();
    result.extend_from_slice(&encrypted);
    
    Ok(result)
}

// AES-GCM decryption for binary data
pub fn ae31(data: &[u8], key: &str) -> Result<Vec<u8>, anyhow::Error> {
    if data.len() < 12 {
        return Err(anyhow::Error::msg("data too short"));
    }
    
    let key_bytes = key.as_bytes();
    let key_bytes = &key_bytes[..16.min(key_bytes.len())];
    let padded_key = if key_bytes.len() < 32 {
        let mut padded = Vec::from(key_bytes);
        padded.resize(32, 0);
        padded
    } else {
        Vec::from(&key_bytes[..32])
    };
    
    let key = GenericArray::clone_from_slice(&padded_key);
    let cipher = Aes256Gcm::new(&key);
    
    let nonce_bytes = &data[..12];
    let ciphertext = &data[12..];
    
    let nonce = GenericArray::from_slice(nonce_bytes);
    let decrypted = cipher.decrypt(nonce, ciphertext)
        .map_err(|_| anyhow::Error::msg("decryption failed"))?;
    
    Ok(decrypted)
}
