use clap::Parser;

#[derive(Parser, Default, Clone, Debug)]
#[clap(name = "beep-content", version, about = "Content server for Beep")]
pub struct Config {
    #[clap(env, long, default_value = "3000", help = "Port to listen on")]
    pub port: u16,

    #[clap(env, long, default_value = "beep.com", help = "Allowed origins")]
    pub origins: Vec<String>,

    #[clap(
        env,
        long,
        default_value = "http://0.0.0.0:3900/",
        help = "S3 endpoint"
    )]
    pub s3_endpoint: String,

    #[clap(env, long, default_value = "beep", help = "S3 bucket")]
    pub s3_bucket: String,

    #[clap(env, long, default_value = "beep_admin", help = "S3 key")]
    pub key_id: String,

    #[clap(env, long, default_value = "beep_admin", help = "S3 secret key")]
    pub secret_key: String,

    #[clap(env, long, default_value = "https://beep.com", help = "Base URL")]
    pub base_url: String,
}

#[cfg(test)]
pub mod tests {
    use crate::config::Config;
    use dotenv::dotenv;

    pub fn bootstrap_integration_tests() -> Config {
        // load environment variables
        dotenv().ok();
        Config {
            origins: std::env::var("ORIGINS")
                .unwrap_or("beep_test.com".to_string())
                .split(',')
                .map(|s| s.to_string())
                .collect(),
            s3_endpoint: std::env::var("S3_ENDPOINT")
                .unwrap_or("http://0.0.0.0:3900/beep_test".to_string()),
            key_id: std::env::var("TEST_KEY_ID").unwrap_or("beep_test_admin".to_string()),
            secret_key: std::env::var("TEST_SECRET_KEY").unwrap_or("beep_test_admin".to_string()),
            s3_bucket: std::env::var("TEST_S3_BUCKET").unwrap_or("test".to_string()),
            ..Default::default()
        }
    }
}
