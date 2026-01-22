use std::fmt::{Display, Formatter};

use hmac::{Hmac, Mac};
use mockall::automock;

use crate::error::ApiError;

#[automock]
pub trait Signer: Send + Sync {
    fn sign(&self, data: &[u8]) -> Result<Vec<u8>, SignerError>;
    #[allow(dead_code)]
    fn verify(&self, data: &[u8], signature: &[u8]) -> Result<bool, SignerError>;
}

pub struct HMACSigner {
    key: String,
}

type HmacSha256 = Hmac<sha2::Sha256>;

impl HMACSigner {
    pub fn new(key: String) -> Result<Self, SignerError> {
        if key.is_empty() {
            return Err(SignerError::InvalidKey("Key cannot be empty".to_string()));
        }
        // I don't want this structure to be mutable but at the same time I want to
        // detect as soon as possible if the key is invalid. So I do it at three different
        // places. At construction time and at each call to sign or verify.
        HmacSha256::new_from_slice(key.as_bytes())
            .map_err(|e| SignerError::InvalidKey(e.to_string()))?;

        Ok(Self { key })
    }
}

impl Signer for HMACSigner {
    fn sign(&self, data: &[u8]) -> Result<Vec<u8>, SignerError> {
        let mut mac = HmacSha256::new_from_slice(self.key.as_bytes())
            .map_err(|e| SignerError::InvalidKey(e.to_string()))?;

        mac.update(data);
        Ok(mac.finalize().into_bytes().to_vec())
    }

    fn verify(&self, data: &[u8], signature: &[u8]) -> Result<bool, SignerError> {
        let mut mac = HmacSha256::new_from_slice(self.key.as_bytes())
            .map_err(|e| SignerError::InvalidKey(e.to_string()))?;
        mac.update(data);
        Ok(mac.verify_slice(signature).is_ok())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SignerError {
    InvalidKey(String),
}

#[allow(clippy::from_over_into)]
impl Into<ApiError> for SignerError {
    fn into(self) -> ApiError {
        ApiError::InternalServerError(self.to_string())
    }
}

impl Display for SignerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SignerError::InvalidKey(e) => write!(f, "{}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signature_without_alteration() {
        let signer = HMACSigner::new("test".to_string()).expect("Invalid key");
        let data = b"test";
        let signature = signer.sign(data).expect("Invalid signature");
        assert!(signer.verify(data, &signature).expect("Invalid signature"));
    }

    #[test]
    fn test_signature_with_alteration() {
        let signer = HMACSigner::new("test".to_string()).expect("Invalid key");
        let data = b"test";
        let signature = signer.sign(data).expect("Invalid signature");
        let signature = signature[1..].to_vec();
        assert!(!signer.verify(data, &signature).expect("Invalid signature"));
    }

    #[test]
    fn test_empty_key() {
        let signer = HMACSigner::new("".to_string());
        assert!(signer.is_err());
    }
}
