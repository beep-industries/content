use std::sync::Arc;

use tracing::info;

use crate::{
    app::AppState, config::Config, error::CoreError, guards::GuardsBuilder,
    plumbing::create_service, signed_url::service::HMACUrlService, signer::HMACSigner,
    utils::RealTime,
};

mod app;
pub mod config;
pub mod error;
mod healthcheck;
mod http;
mod openapi;
mod plumbing;
mod router;
mod s3;
mod signed_url;
mod signer;
mod storage;
pub mod telemetry;
pub mod utils;

mod guards;

#[cfg(test)]
mod router_test;

#[cfg(test)]
mod integrations;

pub async fn app(config: Arc<Config>, time: RealTime) -> Result<(), CoreError> {
    let content_service = Arc::new(
        create_service(config.clone()).map_err(|e| CoreError::StorageError(e.to_string()))?,
    );

    let signer_service = Arc::new(
        HMACUrlService::new(
            HMACSigner::new(config.key_id.clone())
                .map_err(|e| CoreError::SigningKeyError(e.to_string()))?,
            time,
            config.base_url.clone(),
        )
        .map_err(|e| CoreError::SigningKeyError(e.to_string()))?,
    );
    let guards = Arc::new(
        GuardsBuilder::new()
            .add(
                "profile_picture",
                crate::guards::Guard::new(vec![
                    crate::guards::FileType::ImagePNG,
                    crate::guards::FileType::ImageJPEG,
                    crate::guards::FileType::ImageGIF,
                ]),
            )
            .build(),
    );
    let app_state: AppState =
        AppState::new(content_service, config.clone(), signer_service, guards);
    let root = router::app(app_state)
        .await
        .map_err(|e| CoreError::HttpServer(e.to_string()))?;

    info!("Starting server on {}", config.base_url);
    let _ = crate::http::serve(root, config)
        .await
        .inspect_err(|e| eprintln!("{}", e));

    Ok(())
}
