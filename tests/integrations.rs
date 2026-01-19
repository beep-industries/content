use std::sync::Arc;

use content::{config::Config, error::CoreError, utils::RealTime};
use dotenv::dotenv;
use reqwest::multipart::Part;

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
        key_id: std::env::var("KEY_ID").unwrap_or("beep_admin".to_string()),
        secret_key: std::env::var("SECRET_KEY").unwrap_or("beep_admin".to_string()),
        s3_bucket: std::env::var("S3_BUCKET").unwrap_or("test".to_string()),
        base_url: std::env::var("BASE_URL").unwrap_or("https://beep.com".to_string()),
    }
}

async fn launch() -> Result<(), CoreError> {
    let config = Arc::new(bootstrap_config());
    println!("Config {:?}", config);
    content::app(config, RealTime {}).await?;
    Ok(())
}

#[ignore]
#[tokio::test]
async fn integration_full_flow() {
    let handle = tokio::spawn(launch());

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let response = reqwest::get("http://localhost:4000/health")
        .await
        .expect("Failed to make request");

    assert!(response.status().is_success());

    // next we'll sign an url
    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:4000/profile_picture/test.jpg")
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
    let buf: &[u8] = &[0xFF, 0xD8, 0xFF, 0xAA];

    let part = Part::bytes(buf.to_vec())
        .file_name("test.jpg")
        .mime_str("image/jpeg")
        .expect("Failed to create part");

    let form = reqwest::multipart::Form::new().part("file", part);
    let response = client
        .put(&url)
        .multipart(form)
        .send()
        .await
        .expect("Failed to make request");

    let status = response.status();

    let body = response.text().await;
    println!("{:?}", body);

    assert!(status.is_success());

    let response = client
        .get(&url)
        .send()
        .await
        .expect("Failed to make request");

    assert!(response.status().is_success());
    assert_eq!(response.bytes().await.expect("Failed to get response"), buf);

    handle.abort();
}
