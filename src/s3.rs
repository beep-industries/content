use mockall::automock;
use std::fmt::{Display, Formatter};

use aws_config::BehaviorVersion;
use aws_sdk_s3::{self as s3, config::Credentials};
use axum::http::Uri;

use crate::error::ApiError;

pub trait S3: Send + Sync {
    async fn put_object(&self, bucket: &str, key: &str, body: Vec<u8>) -> Result<String, S3Error>;
    #[allow(dead_code)]
    async fn show_buckets(&self) -> Result<Vec<String>, S3Error>;
}

pub struct Garage {
    client: s3::Client,
}

impl Garage {
    pub fn new(url: Uri, key_id: &str, secret_key: &str) -> Self {
        let credentials = Credentials::new(key_id, secret_key, None, None, "beep");

        let s3_config = s3::config::Builder::new()
            .credentials_provider(credentials)
            .endpoint_url(url.to_string())
            .behavior_version(BehaviorVersion::latest())
            .region(s3::config::Region::new("garage"))
            .build();

        let client = s3::Client::from_conf(s3_config);

        Self { client }
    }
}

#[automock]
impl S3 for Garage {
    async fn put_object(&self, bucket: &str, key: &str, body: Vec<u8>) -> Result<String, S3Error> {
        let body_stream = aws_sdk_s3::primitives::ByteStream::from(body);

        self.client
            .put_object()
            .bucket(bucket)
            .key(key)
            .body(body_stream)
            .send()
            .await
            .map_err(|e| {
                let service_error = e.into_service_error();
                S3Error::UploadFailure(service_error.to_string())
            })?;

        Ok(format!(
            "https://{}.s3.garage.aws.dxflrs.com/{}/{}",
            bucket, bucket, key
        ))
    }

    async fn show_buckets(&self) -> Result<Vec<String>, S3Error> {
        let mut buckets = self.client.list_buckets().into_paginator().send();

        let mut bucket_res = vec![];

        while let Some(output) = buckets.next().await {
            let output = output.map_err(|e| {
                let service_error = e.into_service_error();
                S3Error::UploadFailure(service_error.to_string())
            })?;
            for bucket in output.buckets.unwrap() {
                bucket_res.push(bucket.name.unwrap());
            }
        }
        Ok(bucket_res)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum S3Error {
    UploadFailure(String),
}

#[allow(clippy::from_over_into)]
impl Into<ApiError> for S3Error {
    fn into(self) -> ApiError {
        ApiError::InternalServerError(self.to_string())
    }
}

impl Display for S3Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            S3Error::UploadFailure(e) => write!(f, "{}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::config::tests::bootstrap_integration_tests;

    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_show_buckets() {
        let config = bootstrap_integration_tests();
        let s3 = Garage::new(
            config.s3_endpoint.parse().unwrap(),
            &config.key_id,
            &config.secret_key,
        );
        insta::assert_debug_snapshot!(s3.show_buckets().await);
    }

    #[tokio::test]
    #[ignore]
    async fn test_put_object() {
        let config = bootstrap_integration_tests();
        let s3 = Garage::new(
            config.s3_endpoint.parse().unwrap(),
            &config.key_id,
            &config.secret_key,
        );
        let res = s3.put_object("test", "test.txt", vec![1, 2, 3]).await;
        if let Err(e) = res {
            panic!("{}", e);
        }
        assert!(res.is_ok());
    }
}
