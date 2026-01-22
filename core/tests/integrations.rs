use std::sync::Arc;

use content::{config::Config, error::CoreError, utils::RealTime};
use dotenv::dotenv;

fn bootstrap_config() -> Config {
    dotenv().ok();
    Config {
        port: std::env::var("PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(4000),
        origins: std::env::var("ORIGINS")
            .unwrap_or("beep_test.com".to_string())
            .split(',')
            .map(|s| s.to_string())
            .collect(),
        s3_endpoint: std::env::var("S3_ENDPOINT").unwrap_or("http://0.0.0.0:3900/".to_string()),
        key_id: std::env::var("TEST_KEY_ID").unwrap_or("beep_admin".to_string()),
        secret_key: std::env::var("TEST_SECRET_KEY").unwrap_or("beep_admin".to_string()),
        s3_bucket: std::env::var("S3_BUCKET").unwrap_or("test".to_string()),
        base_url: std::env::var("BASE_URL").unwrap_or("https://beep.com".to_string()),
    }
}

async fn launch() -> Result<(), CoreError> {
    let config = Arc::new(bootstrap_config());
    content::app(config, RealTime {}).await?;
    Ok(())
}

async fn integration_full_flow(endpoint: &str, payload: &[u8], mime: &str) {
    let handle = tokio::spawn(launch());

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let response = reqwest::get("http://localhost:4000/health")
        .await
        .expect("Failed to make request");

    assert!(response.status().is_success());

    // next we'll sign an url
    let client = reqwest::Client::new();
    let response = client
        .post(format!("http://localhost:4000/{}", endpoint))
        .json(&serde_json::json!({
            "action": "Put",
            "expires_in_ms": 1000
        }))
        .send()
        .await
        .expect("Failed to make request");
    assert!(response.status().is_success());
    let binding = response.json::<serde_json::Value>().await.unwrap();
    let url = binding["url"].as_str().unwrap();
    assert!(url.starts_with("https://beep.com"));

    let url = url.replace("https://beep.com", "http://localhost:4000");

    // now we'll try to upload a file

    let response = client
        .put(&url)
        .header("Content-Type", mime)
        .body(payload.to_vec())
        .send()
        .await
        .expect("Failed to make request");

    let status = response.status();

    let body = response.text().await;

    assert!(status.is_success());

    let response = client
        .post(format!("http://localhost:4000/{}", endpoint))
        .json(&serde_json::json!({
            "action": "Get",
            "expires_in_ms": 1000
        }))
        .send()
        .await
        .expect("Failed to make request");

    let binding = response.json::<serde_json::Value>().await.unwrap();
    let url = binding["url"].as_str().unwrap();
    assert!(url.starts_with("https://beep.com"));

    let url = url.replace("https://beep.com", "http://localhost:4000");

    let response = client
        .get(&url)
        .send()
        .await
        .expect("Failed to make request");

    assert!(response.status().is_success());
    assert_eq!(
        response.bytes().await.expect("Failed to get response"),
        payload
    );

    handle.abort();
}

#[tokio::test]
async fn test_server_picture_jpeg() {
    integration_full_flow(
        "server_picture/index.jpg",
        &[0xFF, 0xD8, 0xFF, 0xAA],
        "image/jpeg",
    )
    .await;
}
