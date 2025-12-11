use mockall::automock;
use std::fmt::{Display, Formatter};
use tracing::info;

use aws_config::BehaviorVersion;
use aws_sdk_s3::{self as s3, config::Credentials};
use axum::http::Uri;

use crate::error::ApiError;

pub trait S3: Send + Sync {
    async fn put_object(&self, bucket: &str, key: &str, body: Vec<u8>) -> Result<String, S3Error>;
    async fn show_buckets(&self) -> Result<Vec<String>, S3Error>;
    async fn get_object(&self, bucket: &str, key: &str) -> Result<(Vec<u8>, String), S3Error>;
}

pub struct Garage {
    client: s3::Client,
    url: Uri,
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

        Self { client, url }
    }
}

#[automock]
impl S3 for Garage {
    /// Uploads a byte array to S3
    ///
    /// # Examples
    ///
    /// ```
    /// let s3 = Garage::new(
    ///     "https://s3.us-west-2.amazonaws.com".parse().unwrap(),
    ///     "key_id",
    ///     "secret_key",
    /// );
    /// let res = s3.put_object("test", "test.txt", vec![1, 2, 3]).await;
    /// assert!(res.is_ok());
    /// ```
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

        let object_url = self.url.to_string() + "/" + bucket + "/" + key;

        Ok(object_url)
    }

    /// List all buckets on S3
    ///
    /// # Examples
    ///
    /// ```
    /// let s3 = Garage::new(
    ///     "https://s3.us-west-2.amazonaws.com".parse().unwrap(),
    ///     "key_id",
    ///     "secret_key",
    /// );
    /// let res = s3.show_buckets().await;
    /// assert!(res.is_ok());
    /// ```
    async fn show_buckets(&self) -> Result<Vec<String>, S3Error> {
        let mut buckets = self.client.list_buckets().into_paginator().send();

        let mut bucket_res = vec![];

        while let Some(output) = buckets.next().await {
            let output = output.map_err(|e| {
                let service_error = e.into_service_error();
                S3Error::UploadFailure(service_error.to_string())
            })?;
            let Some(buckets) = output.buckets else {
                return Err(S3Error::NoBucketFound);
            };
            for bucket in buckets {
                let Some(bucket_name) = bucket.name else {
                    return Err(S3Error::BucketNameError("No bucket name".to_string()));
                };
                bucket_res.push(bucket_name);
            }
        }
        Ok(bucket_res)
    }

    async fn get_object(&self, bucket: &str, key: &str) -> Result<(Vec<u8>, String), S3Error> {
        let object = self
            .client
            .get_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| {
                let service_error = e.into_service_error();
                S3Error::UploadFailure(service_error.to_string())
            })?;

        info!("Downloading object from S3 {:?}", object);

        let mime_type = object
            .content_type
            .clone()
            .unwrap_or("application/octet-stream".to_string());

        let body = object.body.collect().await.map_err(|e| {
            let service_error = e.to_string();
            S3Error::UploadFailure(service_error)
        })?;

        Ok((body.to_vec(), mime_type.to_string()))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum S3Error {
    UploadFailure(String),
    NoBucketFound,
    BucketNameError(String),
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
            S3Error::NoBucketFound => write!(f, "No bucket found"),
            S3Error::BucketNameError(e) => write!(f, "{}", e),
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
            config.s3_endpoint.parse().expect("Invalid S3 endpoint"),
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
            // this is fine because we are not testing the config
            config.s3_endpoint.parse().expect("Invalid S3 endpoint"),
            &config.key_id,
            &config.secret_key,
        );
        let res = s3.put_object("test", "test.txt", vec![1, 2, 3]).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    #[ignore]
    async fn test_get_object() {
        let config = bootstrap_integration_tests();
        let s3 = Garage::new(
            // this is fine because we are not testing the config
            config.s3_endpoint.parse().expect("Invalid S3 endpoint"),
            &config.key_id,
            &config.secret_key,
        );
        let _ = s3.put_object("test2", "test.txt", vec![1, 2, 3]).await;
        let res = s3.get_object("test2", "test.txt").await;
        assert!(res.is_ok());
    }
}
