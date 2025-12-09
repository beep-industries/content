use std::{
    fmt::{Display, Formatter},
    path::Path,
};

use base64::{Engine as _, engine::general_purpose::URL_SAFE};
use http::{Uri, uri::Scheme};
use mockall::automock;
use serde::{Deserialize, Serialize};

use crate::{
    error::{ApiError, CoreError},
    signer::{HMACSigner, Signer},
    utils::{RealTime, Time},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AvailableActions {
    Put,
    Get,
    Delete,
}

impl Display for AvailableActions {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AvailableActions::Put => write!(f, "Put"),
            AvailableActions::Get => write!(f, "Get"),
            AvailableActions::Delete => write!(f, "Delete"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedURLParams {
    action: AvailableActions,
    expires: i64,
    signature: String,
}

pub type HMACUrlService = SignedUrlServiceImpl<HMACSigner, RealTime>;

#[derive(Debug)]
pub enum SignedUrlError {
    #[allow(dead_code)]
    MissingQueryParams(String),
    #[allow(dead_code)]
    InvalidEncoding,
    InvalidBaseUrl(String),
    #[allow(dead_code)]
    InternalError(String),
    #[allow(dead_code)]
    Expired,
    #[allow(dead_code)]
    InvalidSignature,
    #[allow(dead_code)]
    Unauthorized,
}

#[allow(clippy::from_over_into)]
impl Into<ApiError> for SignedUrlError {
    fn into(self) -> ApiError {
        match self {
            SignedUrlError::MissingQueryParams(e) => ApiError::BadRequest(e),
            SignedUrlError::InvalidEncoding => ApiError::BadRequest("Invalid encoding".to_string()),
            SignedUrlError::InvalidBaseUrl(e) => ApiError::BadRequest(e),
            SignedUrlError::InternalError(e) => ApiError::InternalServerError(e),
            SignedUrlError::Expired => ApiError::Unauthorized("Expired".to_string()),
            SignedUrlError::InvalidSignature => {
                ApiError::Unauthorized("Invalid signature".to_string())
            }
            SignedUrlError::Unauthorized => ApiError::Unauthorized("Unauthorized".to_string()),
        }
    }
}

impl Display for SignedUrlError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SignedUrlError::MissingQueryParams(e) => write!(f, "{}", e),
            SignedUrlError::InvalidEncoding => write!(f, "Invalid encoding"),
            SignedUrlError::InvalidBaseUrl(e) => write!(f, "{}", e),
            SignedUrlError::InternalError(e) => write!(f, "{}", e),
            SignedUrlError::Expired => write!(f, "Expired"),
            SignedUrlError::InvalidSignature => write!(f, "Invalid signature"),
            SignedUrlError::Unauthorized => write!(f, "Unauthorized"),
        }
    }
}

impl From<SignedUrlError> for CoreError {
    fn from(e: SignedUrlError) -> Self {
        CoreError::SigningKeyError(e.to_string())
    }
}

#[automock]
pub trait SignedUrlService<S>
where
    S: Signer,
{
    fn sign_url(
        &self,
        prefix: String,
        action: AvailableActions,
        expires_in_ms: i64,
    ) -> Result<String, SignedUrlError>;
    #[allow(dead_code)]
    fn verify_url(&self, url: &str) -> Result<(), SignedUrlError>;
}

pub struct SignedUrlServiceImpl<S, T>
where
    S: Signer,
    T: Time,
{
    signer: S,
    time: T,
    base_url: Uri,
}

impl<S, T> SignedUrlServiceImpl<S, T>
where
    S: Signer,
    T: Time,
{
    pub fn new(signer: S, time: T, base_url: String) -> Result<Self, SignedUrlError> {
        let base_url = base_url
            .parse::<Uri>()
            .map_err(|e| SignedUrlError::InvalidBaseUrl(e.to_string()))?;
        Ok(Self {
            signer,
            time,
            base_url,
        })
    }

    fn build_signable_url(
        &self,
        prefix: String,
        action: AvailableActions,
        duration: i64,
    ) -> Result<String, SignedUrlError> {
        let path = self.base_url.path();
        let path = Path::new(path).join(prefix);
        let Some(path) = path.to_str() else {
            return Err(SignedUrlError::InvalidBaseUrl(
                "Path is invalid".to_string(),
            ));
        };

        let query = format!("?action={}&expires={}", action, duration);
        let scheme = self.base_url.scheme().unwrap_or(&Scheme::HTTPS);
        let scheme = scheme.as_str();
        let Some(authority) = self.base_url.authority() else {
            return Err(SignedUrlError::InvalidBaseUrl(
                "Authority is missing".to_string(),
            )); // bahhahahah
            // la
            // honte
        };

        let url = Uri::builder()
            .scheme(scheme)
            .authority(authority.as_str())
            .path_and_query(format!("{}{}", path, query))
            .build()
            .map_err(|e| SignedUrlError::InvalidBaseUrl(e.to_string()))?;
        Ok(url.to_string())
    }
}

impl<S, T> SignedUrlService<S> for SignedUrlServiceImpl<S, T>
where
    S: Signer,
    T: Time,
{
    fn sign_url(
        &self,
        prefix: String,
        action: AvailableActions,
        expires_in_ms: i64,
    ) -> Result<String, SignedUrlError> {
        let duration = self.time.now() + expires_in_ms;
        let url = self.build_signable_url(prefix, action, duration)?;
        println!("sign : {}", url);

        let signature = self
            .signer
            .sign(url.as_bytes())
            .map_err(|e| SignedUrlError::InternalError(e.to_string()))?;
        let url = format!("{}&signature={}", url, URL_SAFE.encode(signature));

        Ok(url)
    }

    fn verify_url(&self, url: &str) -> Result<(), SignedUrlError> {
        let parsed_uri = url
            .parse::<Uri>()
            .map_err(|e| SignedUrlError::InvalidBaseUrl(e.to_string()))?;
        let Some(query) = parsed_uri.query() else {
            return Err(SignedUrlError::MissingQueryParams(
                "Missing query params".to_string(),
            ));
        };
        let prefix = parsed_uri.path();
        let parsed_params: SignedURLParams = serde_qs::from_str(query)
            .map_err(|e| SignedUrlError::MissingQueryParams(e.to_string()))?;
        let Ok(signature) = URL_SAFE.decode(parsed_params.signature) else {
            return Err(SignedUrlError::InvalidEncoding);
        };
        let url = self.build_signable_url(
            prefix.to_string(),
            parsed_params.action,
            parsed_params.expires,
        )?;
        if !self
            .signer
            .verify(url.as_bytes(), &signature)
            .map_err(|e| SignedUrlError::InternalError(e.to_string()))?
        {
            return Err(SignedUrlError::InvalidSignature);
        };
        if parsed_params.expires < chrono::Utc::now().timestamp() {
            return Err(SignedUrlError::Expired);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{signer::HMACSigner, utils::tests::get_time};

    pub fn sign_url(prefix: String, action: AvailableActions, expires: i64) -> String {
        let duration = chrono::Utc::now().timestamp() + expires;
        let signer = HMACSigner::new("test".to_string()).expect("Invalid key");
        let time = get_time();
        let service = SignedUrlServiceImpl::new(signer, time, "https://beep.com".to_string())
            .expect("Invalid signer");
        service
            .sign_url(prefix, action, duration)
            .expect("Invalid signature")
    }

    #[test]
    fn test_sign_url() {
        let signer = HMACSigner::new("test".to_string()).expect("Invalid key");
        let time = get_time();
        let service = SignedUrlServiceImpl::new(signer, time, "https://beep.com".to_string())
            .expect("Invalid signer");
        let url = service
            .sign_url("test".to_string(), AvailableActions::Put, 100)
            .expect("Invalid signature");
        insta::assert_snapshot!(url);
    }

    #[test]
    fn test_verify_url() {
        let signer = HMACSigner::new("test".to_string()).expect("Invalid key");
        let time = get_time();
        let service = SignedUrlServiceImpl::new(signer, time, "https://beep.com".to_string())
            .expect("Invalid signer");
        let url = sign_url("test".to_string(), AvailableActions::Put, 100);
        println!("{}", url);

        let params = service.verify_url(&url);
        println!("{:?}", params);
        assert!(params.is_ok());
    }
}
