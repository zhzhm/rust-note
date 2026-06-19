//! AES-256-GCM 加密/解密工具
//!
//! 加密密钥基于本机硬件标识（machine-uid）生成，确保 settings.json
//! 中的 WebDAV 密码只能在当前机器上解密。

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use sha2::{Digest, Sha256};
use rand::RngCore;
use base64::Engine;

/// AES-GCM 推荐的 nonce 长度（12 字节）
const NONCE_LEN: usize = 12;

/// 获取本机绑定的 AES-256 密钥
///
/// 使用 machine-uid 获取硬件标识，再通过 SHA-256 派生为 256 位密钥。
/// 同一台机器每次调用返回相同的密钥；不同机器返回不同的密钥。
pub fn get_machine_key() -> Result<[u8; 32], String> {
    let uid = machine_uid::get().map_err(|e| format!("无法获取机器标识: {}", e))?;
    let mut hasher = Sha256::new();
    hasher.update(uid.as_bytes());
    let result = hasher.finalize();
    Ok(result.into())
}

/// 加密明文密码
///
/// 返回 Base64 编码的「nonce + 密文」。
pub fn encrypt_password(password: &str, key: &[u8; 32]) -> Result<String, String> {
    let cipher =
        Aes256Gcm::new_from_slice(key).map_err(|e| format!("AES 密钥初始化失败: {}", e))?;

    // 生成随机 nonce
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, password.as_bytes())
        .map_err(|e| format!("加密失败: {}", e))?;

    // 拼接 nonce + ciphertext 并 Base64 编码
    let mut combined = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    combined.extend_from_slice(&nonce_bytes);
    combined.extend_from_slice(&ciphertext);

    Ok(base64::engine::general_purpose::STANDARD.encode(&combined))
}

/// 解密密文密码
///
/// `encrypted` 是 `encrypt_password` 返回的 Base64 字符串。
pub fn decrypt_password(encrypted: &str, key: &[u8; 32]) -> Result<String, String> {
    let combined = base64::engine::general_purpose::STANDARD
        .decode(encrypted)
        .map_err(|e| format!("Base64 解码失败: {}", e))?;

    if combined.len() < NONCE_LEN {
        return Err("加密数据格式错误".to_string());
    }

    let (nonce_bytes, ciphertext) = combined.split_at(NONCE_LEN);
    let nonce = Nonce::from_slice(nonce_bytes);

    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| format!("AES 密钥初始化失败: {}", e))?;

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| "解密失败: 密钥不匹配或数据已损坏".to_string())?;

    String::from_utf8(plaintext).map_err(|e| format!("解密结果不是有效文本: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = get_machine_key().expect("should get machine key");
        let password = "my_secret_password_123!@#";

        let encrypted = encrypt_password(password, &key).expect("should encrypt");
        assert!(!encrypted.is_empty());
        assert_ne!(encrypted, password);

        let decrypted = decrypt_password(&encrypted, &key).expect("should decrypt");
        assert_eq!(decrypted, password);
    }

    #[test]
    fn test_wrong_key_fails() {
        let key1 = [1u8; 32];
        let key2 = [2u8; 32];
        let password = "test_password";

        let encrypted = encrypt_password(password, &key1).expect("should encrypt");
        let result = decrypt_password(&encrypted, &key2);
        assert!(result.is_err(), "wrong key should fail decryption");
    }

    #[test]
    fn test_different_plaintexts_different_ciphertexts() {
        let key = get_machine_key().expect("should get machine key");

        let enc1 = encrypt_password("hello", &key).expect("should encrypt");
        let enc2 = encrypt_password("hello", &key).expect("should encrypt");

        // 相同的明文每次加密结果不同（因为 nonce 随机）
        assert_ne!(enc1, enc2);
    }
}
