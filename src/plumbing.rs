use std::sync::Arc;

use crate::{config, s3};

#[derive(Clone)]
pub struct Service<S>
where
    S: s3::S3,
{
    pub s3: Arc<S>,
}

pub type ContentService = Service<s3::Garage>;

pub fn create_service(config: Arc<config::Config>) -> ContentService {
    let s3 = s3::Garage::new(
        config.s3_endpoint.parse().unwrap(),
        &config.key_id,
        &config.secret_key,
    );
    Service { s3: Arc::new(s3) }
}

#[cfg(test)]
pub mod tests {
    use std::sync::Arc;

    use crate::{plumbing::Service, s3::MockGarage};

    pub type MockContentService = Service<MockGarage>;
    #[allow(dead_code)]
    pub fn create_service() -> MockContentService {
        let s3 = MockGarage::new();
        Service { s3: Arc::new(s3) }
    }
}
