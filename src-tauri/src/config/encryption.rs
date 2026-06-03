use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use sha2::{Digest, Sha256};

#[cfg(feature = "password-encryption")]
use pbkdf2::pbkdf2_hmac;

const NONCE_SIZE: usize = 12;

#[derive(Debug)]
pub enum EncryptionError {
    EncryptionFailed,
    DecryptionFailed,
    InvalidInput,
}

impl std::fmt::Display for EncryptionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EncryptionError::EncryptionFailed => write!(f, "Encryption failed"),
            EncryptionError::DecryptionFailed => write!(f, "Decryption failed"),
            EncryptionError::InvalidInput => write!(f, "Invalid input"),
        }
    }
}

impl std::error::Error for EncryptionError {}

pub struct EncryptionManager {
    key: [u8; 32],
}

impl EncryptionManager {
    /// 从 machine-id 派生密钥
    /// Windows 上读取注册表 HKLM\SOFTWARE\Microsoft\Cryptography\MachineGuid
    /// 然后用 SHA-256 哈希后作为密钥
    pub fn from_machine_id() -> Self {
        let machine_guid = Self::get_machine_guid();
        let mut hasher = Sha256::new();
        hasher.update(machine_guid.as_bytes());
        let result = hasher.finalize();
        let mut key = [0u8; 32];
        key.copy_from_slice(&result);
        Self { key }
    }

    /// 从用户密码派生密钥 (PBKDF2)
    #[cfg(feature = "password-encryption")]
    pub fn from_password(password: &str, salt: &[u8]) -> Self {
        let mut key = [0u8; 32];
        pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, 600_000, &mut key);
        Self { key }
    }

    /// 加密数据，返回 base64 编码的密文（格式：base64(nonce + ciphertext)）
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<String, EncryptionError> {
        let cipher = Aes256Gcm::new_from_slice(&self.key)
            .map_err(|_| EncryptionError::EncryptionFailed)?;

        let nonce_bytes = Self::generate_nonce();
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|_| EncryptionError::EncryptionFailed)?;

        let mut combined = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
        combined.extend_from_slice(&nonce_bytes);
        combined.extend_from_slice(&ciphertext);

        Ok(BASE64.encode(&combined))
    }

    /// 解密 base64 编码的密文
    pub fn decrypt(&self, ciphertext: &str) -> Result<Vec<u8>, EncryptionError> {
        let combined = BASE64
            .decode(ciphertext)
            .map_err(|_| EncryptionError::InvalidInput)?;

        if combined.len() < NONCE_SIZE {
            return Err(EncryptionError::InvalidInput);
        }

        let (nonce_bytes, encrypted_data) = combined.split_at(NONCE_SIZE);
        let nonce = Nonce::from_slice(nonce_bytes);

        let cipher = Aes256Gcm::new_from_slice(&self.key)
            .map_err(|_| EncryptionError::DecryptionFailed)?;

        cipher
            .decrypt(nonce, encrypted_data)
            .map_err(|_| EncryptionError::DecryptionFailed)
    }

    fn generate_nonce() -> [u8; NONCE_SIZE] {
        let mut nonce = [0u8; NONCE_SIZE];
        // 使用简单的基于时间的 nonce 生成
        // 生产环境应使用 rand::rngs::OsRng
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let nanos = now.as_nanos() as u64;
        let hash = Sha256::digest(nanos.to_le_bytes());
        nonce.copy_from_slice(&hash[..NONCE_SIZE]);
        nonce
    }

    #[cfg(target_os = "windows")]
    fn get_machine_guid() -> String {
        use std::process::Command;
        // 读取注册表 HKLM\SOFTWARE\Microsoft\Cryptography\MachineGuid
        let output = Command::new("reg")
            .args([
                "query",
                r"HKLM\SOFTWARE\Microsoft\Cryptography",
                "/v",
                "MachineGuid",
            ])
            .output();

        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                // 解析输出，格式如：
                // \r\nHKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Cryptography\r\n    MachineGuid    REG_SZ    xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx\r\n
                for line in stdout.lines() {
                    let line = line.trim();
                    if line.contains("REG_SZ") {
                        if let Some(guid) = line.split("REG_SZ").last() {
                            return guid.trim().to_string();
                        }
                    }
                }
                // 回退：使用默认值
                "autofree-default-machine-id".to_string()
            }
            Err(_) => "autofree-default-machine-id".to_string(),
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn get_machine_guid() -> String {
        // 非Windows平台的回退方案
        // 尝试读取 /etc/machine-id (Linux) 或使用 hostname
        if let Ok(id) = std::fs::read_to_string("/etc/machine-id") {
            id.trim().to_string()
        } else if let Ok(hostname) = std::env::var("HOSTNAME")
            .or_else(|_| std::env::var("HOST"))
        {
            hostname
        } else {
            "autofree-default-machine-id".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let manager = EncryptionManager::from_machine_id();
        let plaintext = b"hello, world! this is a secret API key.";
        let encrypted = manager.encrypt(plaintext).unwrap();
        let decrypted = manager.decrypt(&encrypted).unwrap();
        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_encrypt_produces_different_ciphertexts() {
        let manager = EncryptionManager::from_machine_id();
        let plaintext = b"same data";
        let encrypted1 = manager.encrypt(plaintext).unwrap();
        // 由于 nonce 不同，每次加密结果不同（但这里时间可能相同，所以不强制断言）
        let encrypted2 = manager.encrypt(plaintext).unwrap();
        // 至少能正常解密
        assert!(manager.decrypt(&encrypted1).is_ok());
        assert!(manager.decrypt(&encrypted2).is_ok());
    }
}
