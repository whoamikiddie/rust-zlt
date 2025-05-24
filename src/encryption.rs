use aes_gcm::{aead::{Aead, KeyInit, OsRng, AeadCore}, Aes256Gcm};
use aes_gcm::aead::generic_array::GenericArray;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use rand::Rng;

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

// Generate random key
pub fn y25() -> String {
    let mut rng = rand::thread_rng();
    let mut buffer = [0u8; 16];
    rng.fill(&mut buffer);
    STANDARD.encode(buffer)
}

// AES-GCM encryption and base64 encoding
pub fn ab28(data: &str, key: &str) -> String {
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
    
    match cipher.encrypt(&nonce, data.as_bytes()) {
        Ok(encrypted) => {
            let mut result = nonce.to_vec();
            result.extend_from_slice(&encrypted);
            STANDARD.encode(result)
        },
        Err(_) => String::new()
    }
}

// AES-GCM decryption from base64
pub fn ac29(data: &str, key: &str) -> String {
    if let Ok(decoded) = STANDARD.decode(data) {
        let key_bytes = key.as_bytes();
        let key_bytes = &key_bytes[..16.min(key_bytes.len())];
        let padded_key = if key_bytes.len() < 16 {
            let mut padded = Vec::from(key_bytes);
            padded.resize(16, 0);
            padded
        } else {
            Vec::from(key_bytes)
        };
        
        let key = GenericArray::clone_from_slice(&padded_key);
        let cipher = Aes256Gcm::new(&key);
        
        if decoded.len() > 12 {
            let nonce_bytes = &decoded[..12];
            let ciphertext = &decoded[12..];
            
            let nonce = GenericArray::from_slice(nonce_bytes);
            
            match cipher.decrypt(nonce, ciphertext) {
                Ok(decrypted) => {
                    if let Ok(result) = String::from_utf8(decrypted) {
                        return result;
                    }
                },
                Err(_) => {}
            }
        }
    }
    
    String::new()
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
