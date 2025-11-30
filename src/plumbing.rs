use std::sync::Arc;

use crate::{config, error::CoreError, s3};

#[derive(Clone)]
pub struct Service<S>
where
    S: s3::S3,
{
    pub s3: Arc<S>,
}

pub type ContentService = Service<s3::Garage>;

pub fn create_service(config: Arc<config::Config>) -> Result<ContentService, CoreError> {
    let s3 = s3::Garage::new(
        config
            .s3_endpoint
            .parse()
            .map_err(|_| CoreError::S3EndpointError("Invalid S3 endpoint".to_string()))?,
        &config.key_id,
        &config.secret_key,
    );
    Ok(Service { s3: Arc::new(s3) })
}
