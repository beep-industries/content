use mockall::automock;
use std::fmt::{Display, Formatter};
use tracing::info;

use aws_config::BehaviorVersion;
use aws_sdk_s3::{self as s3, config::Credentials};
use axum::http::Uri;

use crate::error::ApiError;

pub trait S3: Send + Sync {
    async fn put_object(
        &self,
        bucket: &str,
        key: &str,
        file: FileObject,
    ) -> Result<String, S3Error>;
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
            .force_path_style(true)
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
    /// let file = FileObject {
    ///     data: vec![1, 2, 3],
    ///     content_type: "application/octet-stream".to_string(),
    ///     file_name: "test.txt".to_string(),
    /// };
    /// let res = s3.put_object("test", "test.txt", file).await;
    /// assert!(res.is_ok());
    /// ```
    async fn put_object(
        &self,
        bucket: &str,
        key: &str,
        file: FileObject,
    ) -> Result<String, S3Error> {
        let body = file.data;
        let content_type = file.content_type;
        let body_stream = aws_sdk_s3::primitives::ByteStream::from(body);

        self.client
            .put_object()
            .bucket(bucket)
            .key(key)
            .content_type(content_type)
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

    /// Download an object from an s3
    /// This functions if successful returns `(Vec<u8>, String)`. The Vec<u8> is an
    /// array containing the file. The String defines the file content type.
    ///
    /// # Examples
    ///
    ///```
    /// let s3 = Garage::new(
    ///     "https://s3.us-west-2.amazonaws.com".parse().unwrap(),
    ///     "key_id",
    ///     "secret_key",
    /// )
    /// let res = s3.show_buckets().await;
    /// assert!(res.is_ok());
    /// ```
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

#[derive(Debug)]
pub struct FileObject {
    pub data: Vec<u8>,
    pub content_type: String,
}
