#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use clap::Parser;
    use dotenv::dotenv;

    use crate::{app, config::Config, error::CoreError, utils};

    async fn launch() -> Result<(), CoreError> {
        dotenv().ok();
        let config = Arc::new(Config::parse());

        app(config, utils::RealTime {}).await?;
        Ok(())
    }

    #[ignore]
    #[tokio::test]
    async fn test_full_flow() {
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
        let url = response.text().await.expect("Failed to get response");
        assert!(url.starts_with("https://beep.com"));

        let url = url.replace("https://beep.com", "http://localhost:4000");

        // now we'll try to upload a file
        let buf: &[u8] = &[0xFF, 0xD8, 0xFF, 0xAA];

        let form = reqwest::multipart::Form::new()
            .part("file", reqwest::multipart::Part::bytes(buf.to_vec()));
        let response = client
            .put(&url)
            .multipart(form)
            .send()
            .await
            .expect("Failed to make request");

        assert!(response.status().is_success());

        let response = client
            .get(&url)
            .send()
            .await
            .expect("Failed to make request");

        assert!(response.status().is_success());
        assert_eq!(response.bytes().await.expect("Failed to get response"), buf);

        handle.abort();
    }
}
