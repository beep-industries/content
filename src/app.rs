use std::sync::Arc;

use http::request::Parts;
use mockall::automock;

use crate::{
    config::Config,
    guards::Guards,
    plumbing::ContentService,
    s3::{FileObject, S3, S3Error},
    signed_url::{
        extractor::Claims,
        service::{AvailableActions, HMACUrlService, SignedUrlError, SignedUrlService},
    },
};

#[automock]
pub trait AppStateOperations {
    fn config(&self) -> Arc<Config>;
    async fn upload(&self, bucket: &str, key: &str, file: FileObject) -> Result<String, S3Error>;
    async fn show_buckets(&self) -> Result<Vec<String>, S3Error>;
    fn sign_url(
        &self,
        prefix: String,
        action: AvailableActions,
        expires_in_ms: u64,
    ) -> Result<String, SignedUrlError>;
    async fn get_object(&self, bucket: &str, key: &str) -> Result<(Vec<u8>, String), S3Error>;
    fn verify_parts(&self, parts: Parts) -> Result<Claims, SignedUrlError>;
    fn guards(&self) -> Arc<Guards>;
}

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub service: Arc<ContentService>,
    pub signer: Arc<HMACUrlService>,
    pub guards: Arc<Guards>,
}

impl AppState {
    pub fn new(
        service: Arc<ContentService>,
        args: Arc<Config>,
        signer: Arc<HMACUrlService>,
        guards: Arc<Guards>,
    ) -> Self {
        Self {
            service,
            config: args,
            signer,
            guards,
        }
    }
}

impl AppStateOperations for AppState {
    fn config(&self) -> Arc<Config> {
        self.config.clone()
    }

    async fn upload(&self, bucket: &str, key: &str, file: FileObject) -> Result<String, S3Error> {
        self.service.s3.put_object(bucket, key, file).await
    }

    async fn show_buckets(&self) -> Result<Vec<String>, S3Error> {
        self.service.s3.show_buckets().await
    }

    fn sign_url(
        &self,
        prefix: String,
        action: AvailableActions,
        expires_in_ms: u64,
    ) -> Result<String, SignedUrlError> {
        self.signer.sign_url(prefix, action, expires_in_ms)
    }

    fn verify_parts(&self, parts: Parts) -> Result<Claims, SignedUrlError> {
        self.signer.verify_parts(parts)
    }

    async fn get_object(&self, bucket: &str, key: &str) -> Result<(Vec<u8>, String), S3Error> {
        self.service.s3.get_object(bucket, key).await
    }

    fn guards(&self) -> Arc<Guards> {
        self.guards.clone()
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use std::sync::Arc;

    use crate::app::MockAppStateOperations;

    #[derive(Clone)]
    pub struct TestAppState(Arc<MockAppStateOperations>);

    impl TestAppState {
        pub fn new(mock: MockAppStateOperations) -> Self {
            Self(Arc::new(mock))
        }
    }

    impl AppStateOperations for TestAppState {
        fn config(&self) -> Arc<Config> {
            self.0.config()
        }

        async fn upload(
            &self,
            bucket: &str,
            key: &str,
            file: FileObject,
        ) -> Result<String, S3Error> {
            self.0.upload(bucket, key, file).await
        }

        async fn show_buckets(&self) -> Result<Vec<String>, S3Error> {
            self.0.show_buckets().await
        }

        fn sign_url(
            &self,
            prefix: String,
            action: AvailableActions,
            expires_in_ms: u64,
        ) -> Result<String, SignedUrlError> {
            self.0.sign_url(prefix, action, expires_in_ms)
        }

        fn verify_parts(&self, parts: Parts) -> Result<Claims, SignedUrlError> {
            self.0.verify_parts(parts)
        }

        async fn get_object(&self, bucket: &str, key: &str) -> Result<(Vec<u8>, String), S3Error> {
            self.0.get_object(bucket, key).await
        }

        fn guards(&self) -> Arc<Guards> {
            self.0.guards()
        }
    }
}
