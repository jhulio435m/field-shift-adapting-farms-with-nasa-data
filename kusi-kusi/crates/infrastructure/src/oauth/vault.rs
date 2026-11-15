use chacha20poly1305::{
    XChaCha20Poly1305, XNonce,
    aead::{Aead, Generate, KeyInit, Payload},
};
use kusi_application::oauth::OAuthError;
use std::collections::HashMap;

pub struct TokenVault {
    active: String,
    keys: HashMap<String, Vec<u8>>,
}
impl TokenVault {
    pub fn new(active: String, keys: HashMap<String, Vec<u8>>) -> Result<Self, OAuthError> {
        if !keys.contains_key(&active)
            || keys.iter().any(|(id, k)| {
                id.is_empty()
                    || id.len() > 32
                    || !id
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
                    || k.len() != 32
            })
        {
            return Err(OAuthError::Storage);
        }
        Ok(Self { active, keys })
    }
    pub fn encrypt(&self, aad: &str, plaintext: &[u8]) -> Result<String, OAuthError> {
        let key = self.keys.get(&self.active).ok_or(OAuthError::Storage)?;
        let cipher = XChaCha20Poly1305::new_from_slice(key).map_err(|_| OAuthError::Storage)?;
        let nonce = XNonce::generate();
        let encrypted = cipher
            .encrypt(
                &nonce,
                Payload {
                    msg: plaintext,
                    aad: aad.as_bytes(),
                },
            )
            .map_err(|_| OAuthError::Storage)?;
        Ok(format!(
            "{}:{}:{}",
            self.active,
            hex::encode(nonce),
            hex::encode(encrypted)
        ))
    }
    pub fn decrypt(&self, aad: &str, envelope: &str) -> Result<Vec<u8>, OAuthError> {
        let parts: Vec<_> = envelope.split(':').collect();
        if parts.len() != 3 {
            return Err(OAuthError::Storage);
        }
        let key = self.keys.get(parts[0]).ok_or(OAuthError::Storage)?;
        let bytes: [u8; 24] = hex::decode(parts[1])
            .map_err(|_| OAuthError::Storage)?
            .try_into()
            .map_err(|_| OAuthError::Storage)?;
        let nonce: XNonce = bytes.into();
        let ciphertext = hex::decode(parts[2]).map_err(|_| OAuthError::Storage)?;
        XChaCha20Poly1305::new_from_slice(key)
            .map_err(|_| OAuthError::Storage)?
            .decrypt(
                &nonce,
                Payload {
                    msg: &ciphertext,
                    aad: aad.as_bytes(),
                },
            )
            .map_err(|_| OAuthError::Storage)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn encryption_is_randomized_authenticated_and_tenant_bound() {
        let vault = TokenVault::new("v1".into(), HashMap::from([("v1".into(), vec![7; 32])]))
            .expect("fixture vault");
        let one = vault
            .encrypt("tenant-a:account:tokens", b"fixture-token")
            .expect("encrypt");
        let two = vault
            .encrypt("tenant-a:account:tokens", b"fixture-token")
            .expect("encrypt");
        assert_ne!(one, two);
        assert!(!one.contains("fixture-token"));
        assert_eq!(
            vault
                .decrypt("tenant-a:account:tokens", &one)
                .expect("decrypt"),
            b"fixture-token"
        );
        assert!(vault.decrypt("tenant-b:account:tokens", &one).is_err());
        let mut tampered = one.clone();
        tampered.push('0');
        assert!(vault.decrypt("tenant-a:account:tokens", &tampered).is_err());
    }
}
