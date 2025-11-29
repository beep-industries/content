use std::sync::Arc;

use mockall::automock;

use crate::{
    config::Config,
    plumbing::ContentService,
    s3::{S3, S3Error},
};

#[automock]
pub trait AppStateOperations {
    fn config(&self) -> Arc<Config>;
    async fn upload(&self, bucket: &str, key: &str, body: Vec<u8>) -> Result<String, S3Error>;
    async fn show_buckets(&self) -> Result<Vec<String>, S3Error>;
}

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub service: Arc<ContentService>,
}

impl AppState {
    pub fn new(service: Arc<ContentService>, args: Arc<Config>) -> Self {
        Self {
            service,
            config: args,
        }
    }
}

impl AppStateOperations for AppState {
    fn config(&self) -> Arc<Config> {
        self.config.clone()
    }

    async fn upload(&self, bucket: &str, key: &str, body: Vec<u8>) -> Result<String, S3Error> {
        self.service.s3.put_object(bucket, key, body).await
    }

    async fn show_buckets(&self) -> Result<Vec<String>, S3Error> {
        self.service.s3.show_buckets().await
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

        async fn upload(&self, bucket: &str, key: &str, body: Vec<u8>) -> Result<String, S3Error> {
            self.0.upload(bucket, key, body).await
        }

        async fn show_buckets(&self) -> Result<Vec<String>, S3Error> {
            self.0.show_buckets().await
        }
    }
}
