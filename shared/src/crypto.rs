//! 加密工具模块
//! 使用 ChaCha20-Poly1305 加密通信

use chacha20poly1305::{
    aead::{Aead, KeyInit, OsRng},
    ChaCha20Poly1305, Nonce,
};
use rand::RngCore;

/// 加密密钥长度 (256 bits = 32 bytes)
pub const KEY_SIZE: usize = 32;
/// Nonce 长度 (96 bits = 12 bytes)
pub const NONCE_SIZE: usize = 12;

/// ChaCha20-Poly1305 加密器
pub struct Crypto {
    cipher: ChaCha20Poly1305,
}

/// 加密错误
#[derive(Debug)]
pub struct CryptoError;

impl std::fmt::Display for CryptoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Crypto error")
    }
}

impl std::error::Error for CryptoError {}

impl Crypto {
    /// 从密钥创建加密器
    pub fn new(key: &[u8; KEY_SIZE]) -> Self {
        let cipher = ChaCha20Poly1305::new_from_slice(key).expect("Invalid key length");
        Self { cipher }
    }
    
    /// 从 hex 字符串创建加密器
    pub fn from_hex(hex_key: &str) -> Result<Self, CryptoError> {
        let key = hex_to_bytes(hex_key).ok_or(CryptoError)?;
        if key.len() != KEY_SIZE {
            return Err(CryptoError);
        }
        let mut key_array = [0u8; KEY_SIZE];
        key_array.copy_from_slice(&key);
        Ok(Self::new(&key_array))
    }
    
    /// 生成随机密钥
    pub fn generate_key() -> [u8; KEY_SIZE] {
        let mut key = [0u8; KEY_SIZE];
        OsRng.fill_bytes(&mut key);
        key
    }
    
    /// 生成随机密钥并返回 hex 字符串
    pub fn generate_key_hex() -> String {
        bytes_to_hex(&Self::generate_key())
    }
    
    /// 加密数据
    /// 返回: nonce (12 bytes) + ciphertext
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        let ciphertext = self.cipher.encrypt(nonce, plaintext).map_err(|_| CryptoError)?;
        
        // 将 nonce 附加到密文前面
        let mut result = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
        result.extend_from_slice(&nonce_bytes);
        result.extend(ciphertext);
        
        Ok(result)
    }
    
    /// 解密数据
    /// 输入格式: nonce (12 bytes) + ciphertext
    pub fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        if data.len() < NONCE_SIZE {
            return Err(CryptoError);
        }
        
        let (nonce_bytes, ciphertext) = data.split_at(NONCE_SIZE);
        let nonce = Nonce::from_slice(nonce_bytes);
        
        self.cipher.decrypt(nonce, ciphertext).map_err(|_| CryptoError)
    }
}

/// 字节转 hex 字符串
pub fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// hex 字符串转字节
pub fn hex_to_bytes(hex: &str) -> Option<Vec<u8>> {
    if hex.len() % 2 != 0 {
        return None;
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_encrypt_decrypt() {
        let key = Crypto::generate_key();
        let crypto = Crypto::new(&key);
        
        let plaintext = b"Hello, World!";
        let encrypted = crypto.encrypt(plaintext).unwrap();
        let decrypted = crypto.decrypt(&encrypted).unwrap();
        
        assert_eq!(plaintext.as_slice(), decrypted.as_slice());
    }
    
    #[test]
    fn test_hex_key() {
        let hex_key = Crypto::generate_key_hex();
        assert_eq!(hex_key.len(), KEY_SIZE * 2);
        
        let crypto = Crypto::from_hex(&hex_key).unwrap();
        let plaintext = b"Test message";
        let encrypted = crypto.encrypt(plaintext).unwrap();
        let decrypted = crypto.decrypt(&encrypted).unwrap();
        
        assert_eq!(plaintext.as_slice(), decrypted.as_slice());
    }
    
    #[test]
    fn test_wrong_key_fails() {
        let key1 = Crypto::generate_key();
        let key2 = Crypto::generate_key();
        
        let crypto1 = Crypto::new(&key1);
        let crypto2 = Crypto::new(&key2);
        
        let plaintext = b"Secret message";
        let encrypted = crypto1.encrypt(plaintext).unwrap();
        
        // 使用错误密钥解密应该失败
        assert!(crypto2.decrypt(&encrypted).is_err());
    }
    
    #[test]
    fn test_encrypt_empty_data() {
        let key = Crypto::generate_key();
        let crypto = Crypto::new(&key);
        
        let plaintext = b"";
        let encrypted = crypto.encrypt(plaintext).unwrap();
        let decrypted = crypto.decrypt(&encrypted).unwrap();
        
        assert_eq!(plaintext.as_slice(), decrypted.as_slice());
    }
    
    #[test]
    fn test_encrypt_large_data() {
        let key = Crypto::generate_key();
        let crypto = Crypto::new(&key);
        
        let plaintext = vec![0xABu8; 1024 * 64]; // 64KB
        let encrypted = crypto.encrypt(&plaintext).unwrap();
        let decrypted = crypto.decrypt(&encrypted).unwrap();
        
        assert_eq!(plaintext, decrypted);
    }
    
    #[test]
    fn test_decrypt_too_short() {
        let key = Crypto::generate_key();
        let crypto = Crypto::new(&key);
        
        // 数据太短（小于 NONCE_SIZE）
        assert!(crypto.decrypt(&[0u8; 5]).is_err());
        assert!(crypto.decrypt(&[]).is_err());
    }
    
    #[test]
    fn test_decrypt_corrupted() {
        let key = Crypto::generate_key();
        let crypto = Crypto::new(&key);
        
        let plaintext = b"Test";
        let mut encrypted = crypto.encrypt(plaintext).unwrap();
        // 修改密文数据
        if let Some(last) = encrypted.last_mut() {
            *last ^= 0xFF;
        }
        assert!(crypto.decrypt(&encrypted).is_err());
    }
    
    #[test]
    fn test_hex_conversion_roundtrip() {
        let original = vec![0x00, 0x11, 0xAA, 0xFF, 0x5C];
        let hex = bytes_to_hex(&original);
        let recovered = hex_to_bytes(&hex).unwrap();
        assert_eq!(original, recovered);
    }
    
    #[test]
    fn test_hex_invalid_input() {
        assert!(hex_to_bytes("GG").is_none());  // 非法 hex 字符
        assert!(hex_to_bytes("abc").is_none()); // 奇数长度
        assert_eq!(hex_to_bytes("").unwrap(), Vec::<u8>::new()); // 空字符串
    }
    
    #[test]
    fn test_from_hex_wrong_length() {
        // 密钥长度不是 32 字节
        let short_hex = "00112233";
        assert!(Crypto::from_hex(short_hex).is_err());
    }
    
    #[test]
    fn test_each_encryption_is_unique() {
        let key = Crypto::generate_key();
        let crypto = Crypto::new(&key);
        let plaintext = b"Same message";
        
        let enc1 = crypto.encrypt(plaintext).unwrap();
        let enc2 = crypto.encrypt(plaintext).unwrap();
        
        // 每次加密结果不同（因为 nonce 随机）
        assert_ne!(enc1, enc2);
        // 但解密结果相同
        assert_eq!(crypto.decrypt(&enc1).unwrap(), crypto.decrypt(&enc2).unwrap());
    }
}
