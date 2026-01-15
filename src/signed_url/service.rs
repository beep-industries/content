use std::path::Path;

use axum::response::IntoResponse;
use base64::{Engine as _, engine::general_purpose::URL_SAFE};
use http::{StatusCode, Uri, uri::Scheme};
use mockall::automock;
use serde::{Deserialize, Serialize};
use strum_macros::Display;
use thiserror::Error;
use utoipa::ToSchema;

use crate::{
    error::CoreError,
    signed_url::extractor::Claims,
    signer::{HMACSigner, Signer},
    utils::{RealTime, Time},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema, Copy, Default, Display)]
pub enum AvailableActions {
    #[default]
    Put,
    Get,
    Delete,
}

impl From<AvailableActions> for http::Method {
    fn from(action: AvailableActions) -> Self {
        match action {
            AvailableActions::Put => http::Method::PUT,
            AvailableActions::Get => http::Method::GET,
            AvailableActions::Delete => http::Method::DELETE,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedURLParams {
    pub action: AvailableActions,
    pub expires: u64,
    pub signature: String,
}

pub type HMACUrlService = SignedUrlServiceImpl<HMACSigner, RealTime>;

#[derive(Debug, Error, strum_macros::Display)]
pub enum SignedUrlError {
    MissingQueryParams(String),
    InvalidEncoding,
    InvalidBaseUrl(String),
    InternalError(String),
    Expired,
    InvalidSignature,
}

impl IntoResponse for SignedUrlError {
    fn into_response(self) -> axum::response::Response {
        let status = match self {
            SignedUrlError::MissingQueryParams(_) => StatusCode::BAD_REQUEST,
            SignedUrlError::InvalidEncoding => StatusCode::BAD_REQUEST,
            SignedUrlError::InvalidBaseUrl(_) => StatusCode::BAD_REQUEST,
            SignedUrlError::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            SignedUrlError::Expired => StatusCode::UNAUTHORIZED,
            SignedUrlError::InvalidSignature => StatusCode::UNAUTHORIZED,
        };
        (status, self.to_string()).into_response()
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
        expires_in_ms: u64,
    ) -> Result<String, SignedUrlError>;
    #[allow(dead_code)]
    fn verify_url(&self, url: &str) -> Result<Claims, SignedUrlError>;
    fn verify_parts(&self, parts: http::request::Parts) -> Result<Claims, SignedUrlError>;
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
        duration: u64,
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
        expires_in_ms: u64,
    ) -> Result<String, SignedUrlError> {
        let duration = self.time.now() + expires_in_ms;
        let url = self.build_signable_url(prefix, action, duration)?;

        let signature = self
            .signer
            .sign(url.as_bytes())
            .map_err(|e| SignedUrlError::InternalError(e.to_string()))?;
        let url = format!("{}&signature={}", url, URL_SAFE.encode(signature));

        Ok(url)
    }

    fn verify_url(&self, url: &str) -> Result<Claims, SignedUrlError> {
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
        let now = self.time.now();
        if parsed_params.expires < now {
            return Err(SignedUrlError::Expired);
        }
        let action = parsed_params.action;
        let prefix = prefix.trim_start_matches('/');
        let path = prefix.split_once('/').unwrap_or((prefix, ""));
        if path.1.is_empty() {
            return Err(SignedUrlError::InvalidBaseUrl(
                "Path is invalid".to_string(),
            ));
        }
        Ok(Claims {
            action,
            path: (path.0.to_string(), path.1.to_string()),
        })
    }

    fn verify_parts(&self, parts: http::request::Parts) -> Result<Claims, SignedUrlError> {
        let uri = parts.uri.to_string();
        let claims = self.verify_url(&uri)?;
        let action_as_method: http::Method = claims.action.into();
        if parts.method != action_as_method {
            return Err(SignedUrlError::InvalidSignature);
        }
        Ok(claims)
    }
}

#[cfg(test)]
mod tests {
    use axum::extract::FromRequest;
    use http::Request;

    use super::*;
    use crate::{signer::HMACSigner, utils::tests::get_time};

    pub fn sign_url(prefix: String, action: AvailableActions, expires: u64) -> String {
        let now: u64 = chrono::Utc::now()
            .timestamp()
            .try_into()
            .expect("Time shouldnt be negative");
        let duration = now + expires;
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
        let url = sign_url("/test/test".to_string(), AvailableActions::Put, 100);
        let params = service.verify_url(&url);
        assert!(params.is_ok());
    }

    #[tokio::test]
    async fn test_verify_parts() {
        let signer = HMACSigner::new("test".to_string()).expect("Invalid key");
        let time = get_time();
        let service = SignedUrlServiceImpl::new(signer, time, "https://beep.com".to_string())
            .expect("Invalid signer");
        let url = sign_url("/bucket/test".to_string(), AvailableActions::Put, 100);
        let request = Request::builder()
            .uri(url)
            .method(http::Method::PUT)
            .body(axum::body::Body::empty())
            .unwrap();

        let parts = http::request::Parts::from_request(request, &())
            .await
            .expect("Invalid request");
        let params = service.verify_parts(parts);
        assert!(params.is_ok());
        let params = params.unwrap();
        assert_eq!(params.path, ("bucket".to_string(), "test".to_string()));
        assert_eq!(params.action, AvailableActions::Put);
    }

    #[tokio::test]
    async fn test_verify_parts_invalid_method() {
        let signer = HMACSigner::new("test".to_string()).expect("Invalid key");
        let time = get_time();
        let service = SignedUrlServiceImpl::new(signer, time, "https://beep.com".to_string())
            .expect("Invalid signer");
        let url = sign_url("test".to_string(), AvailableActions::Put, 100);
        let request = Request::builder()
            .uri(url)
            .method(http::Method::GET)
            .body(axum::body::Body::empty())
            .unwrap();

        let parts = http::request::Parts::from_request(request, &())
            .await
            .expect("Invalid request");
        let params = service.verify_parts(parts);
        assert!(params.is_err());
    }
}
