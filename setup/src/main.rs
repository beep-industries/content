use std::env;
use std::fs::{self, OpenOptions};
use std::io::{BufReader, Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::Duration;

use anyhow::anyhow;
use base64::Engine;
use clap::{Parser, ValueEnum};

const BUCKETS: [&str; 2] = ["beep", "test"];
const MAX_RETRIES: u32 = 30;

#[derive(Parser)]
#[command(name = "setup")]
#[command(about = "Setup script for Garage S3 storage")]
struct Cli {
    #[arg(value_enum)]
    action: Action,

    #[arg(value_enum, default_value = "no-env")]
    write_env: WriteEnv,
}

#[derive(Clone, ValueEnum)]
enum Action {
    Reset,
    SetupS3,
    GenKey,
    Setup,
}

#[derive(Clone, ValueEnum, PartialEq)]
enum WriteEnv {
    Env,
    NoEnv,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let write_env = cli.write_env == WriteEnv::Env;

    match check_if_need_to_setup() {
        Ok(_) => {}
        Err(e) => {
            println!("No need to setup, aborting: {}", e);
            return Ok(());
        }
    }

    if should_init_env_file() && write_env {
        init_env_file().map_err(|e| {
            println!("Failed to init env file: {}", e);
            e
        })?;
    }

    match cli.action {
        Action::Reset => {
            reset()?;
            wait_until_s3_up()?;
            setup_s3(write_env)?;
            setup_signing_key(write_env)?;
        }
        Action::SetupS3 => {
            println!("Waiting for S3 to be ready");
            wait_until_s3_up()?;
            println!("Starting setup");
            setup_s3(write_env)?;
        }
        Action::GenKey => {
            setup_signing_key(write_env)?;
        }
        Action::Setup => {
            println!("Waiting for S3 to be ready");
            wait_until_s3_up()?;
            println!("Starting setup");
            setup_s3(write_env)?;
            println!("Generating signing key");
            setup_signing_key(write_env)?;
        }
    }

    Ok(())
}

fn check_if_need_to_setup() -> anyhow::Result<()> {
    let bucket_list = exec("garage", &["bucket", "list"])?;

    for bucket in BUCKETS {
        let is_present = bucket_list.lines().any(|line| line.contains(bucket));
        if !is_present {
            return Ok(());
        }
    }
    anyhow::bail!("Buckets already exist, aborting setup")
}

fn should_init_env_file() -> bool {
    let needed_keys = vec![
        "ORIGINS",
        "PORT",
        "OTEL_EXPORTER_OTLP_ENDPOINT",
        "S3_ENDPOINT",
        "BASE_URL",
    ];
    let env_file = env::var("ENV_FILE").unwrap_or(".env".to_string());
    if !Path::new(&env_file).exists() {
        return true;
    }
    let env_file_content = fs::read_to_string(&env_file).unwrap_or_default();
    for key in needed_keys {
        if !env_file_content.contains(key) {
            return true;
        }
    }
    false
}

fn update_env_var(key: &str, value: &str) -> anyhow::Result<()> {
    let env_file = env::var("ENV_FILE").unwrap_or(".env".to_string());
    println!("Updating env var {} to {} in file {}", key, value, env_file);

    let mut file = OpenOptions::new().read(true).open(&env_file)?;

    let mut env_file_content = String::new();
    file.read_to_string(&mut env_file_content)?;

    let mut new_env_file_content = String::new();
    let mut key_exists = false;

    for line in env_file_content.lines() {
        if line.starts_with(&format!("{}=", key)) {
            new_env_file_content.push_str(&format!("{}={}\n", key, value));
            key_exists = true;
        } else {
            new_env_file_content.push_str(line);
            new_env_file_content.push('\n');
        }
    }

    // If the key doesn't exist, add it
    if !key_exists {
        new_env_file_content.push_str(&format!("{}={}\n", key, value));
    }

    // Reopen the file for writing, truncating it
    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&env_file)?;

    file.write_all(new_env_file_content.as_bytes())?;
    println!("New content: {}", new_env_file_content);

    Ok(())
}

fn exec(command: &str, args: &[&str]) -> anyhow::Result<String> {
    let output = Command::new(command)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;

    Ok(format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ))
}

fn exec_silent(command: &str, args: &[&str]) -> anyhow::Result<()> {
    Command::new(command)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .output()?;

    Ok(())
}

fn create_buckets() -> anyhow::Result<()> {
    let bucket_list = exec("garage", &["bucket", "list"])?;

    for bucket in BUCKETS {
        let is_present = bucket_list.lines().any(|line| line.contains(bucket));
        if !is_present {
            exec_silent("garage", &["bucket", "create", bucket])?;
        }
    }

    Ok(())
}

fn create_keys(write_env: bool) -> anyhow::Result<()> {
    let key_list = exec("garage", &["key", "list"])?;

    for bucket in BUCKETS {
        let key_name = format!("{}_admin", bucket);
        let is_present = key_list.lines().any(|line| line.contains(&key_name));

        if !is_present {
            let key_infos = exec("garage", &["key", "create", &key_name])?;

            let key_id = extract_value(&key_infos, "Key ID");
            let secret_key = extract_value(&key_infos, "Secret key");

            exec_silent(
                "garage",
                &[
                    "bucket", "allow", "--read", "--write", "--owner", bucket, "--key", &key_name,
                ],
            )?;

            if bucket == "beep" {
                if write_env {
                    update_env_var("KEY_ID", &key_id)?;
                    update_env_var("SECRET_KEY", &secret_key)?;
                }
                println!("KEY_ID={}", key_id);
                println!("SECRET_KEY={}", secret_key);
            }

            if bucket == "test" {
                if write_env {
                    update_env_var("TEST_KEY_ID", &key_id)?;
                    update_env_var("TEST_SECRET_KEY", &secret_key)?;
                }
                println!("TEST_KEY_ID={}", key_id);
                println!("TEST_SECRET_KEY={}", secret_key);
            }
        }
    }

    Ok(())
}

fn extract_value(text: &str, key: &str) -> String {
    for line in text.lines() {
        if line.contains(key)
            && let Some(value) = line.split(':').nth(1)
        {
            return value.trim().replace([' ', '\r', '\n'], "");
        }
    }
    String::new()
}

fn setup_s3(write_env: bool) -> anyhow::Result<()> {
    let status = exec("garage", &["status"])?;
    println!("{}", status);
    let node_id_output = exec("garage", &["node", "id"])?;
    let node_id = node_id_output
        .split('@')
        .next()
        .unwrap_or("")
        .trim()
        .replace(['\r', '\n'], "");
    println!("node_id: {}", node_id);

    let layout_output = exec("garage", &["layout", "show"])?;
    println!("layout_output: {}", layout_output);
    let is_initial_layout = layout_output
        .lines()
        .any(|line| line.contains("Current cluster layout version: 0"));

    if is_initial_layout {
        let res = exec(
            "garage",
            &["layout", "assign", "-z", "dc1", "-c", "1G", &node_id],
        )?;
        println!("layout assign: {}", res);
        let res = exec("garage", &["layout", "apply", "--version", "1"])?;
        println!("layout apply: {}", res);
    }

    create_buckets()?;
    create_keys(write_env)?;

    Ok(())
}

fn wait_until_s3_up() -> anyhow::Result<()> {
    for _ in 0..MAX_RETRIES {
        let status = Command::new("garage")
            .args(["status"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;

        println!("status: {:?}", status);
        if status.success() {
            let bucket_endpoint = env::var("BUCKET_ENDPOINT").expect("BUCKET_ENDPOINT not set");
            let res = reqwest::blocking::get(&bucket_endpoint);
            println!("res: {:?}", res);
            if res.is_err() {
                println!("Waiting for S3 to be ready");
                sleep(Duration::from_secs(1));
                continue;
            } else {
                return Ok(());
            }
        }

        sleep(Duration::from_secs(1));
    }

    anyhow::bail!("Timeout waiting for S3 to be ready")
}

fn setup_signing_key(write_env: bool) -> anyhow::Result<()> {
    let mut random_bytes = [0u8; 20];
    let file = fs::File::open("/dev/urandom")?;
    let mut reader = BufReader::new(file);
    reader.read_exact(&mut random_bytes)?;

    let signing_key = base64::engine::general_purpose::STANDARD.encode(random_bytes);

    println!("SIGNING_KEY={}", signing_key);

    if write_env {
        update_env_var("SIGNING_KEY", &signing_key)?;
    }

    Ok(())
}

fn init_env_file() -> anyhow::Result<()> {
    let env_file = env::var("ENV_FILE").unwrap_or(".env".to_string());
    let base_file = include_str!("../../.env.example");
    println!("Writing env file: {}", env_file);
    println!("Base file: {}", base_file);
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(env_file)
        .map_err(|e| anyhow!("Failed to open env file: {}", e))?;
    file.write_all(base_file.as_bytes())
        .map_err(|e| anyhow!("Failed to write env file: {}", e))?;
    Ok(())
}

fn reset() -> anyhow::Result<()> {
    Command::new("docker")
        .args(["compose", "--ansi", "never", "down", "-v"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;

    Command::new("docker")
        .args(["compose", "--ansi", "never", "up", "-d"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;

    Ok(())
}
