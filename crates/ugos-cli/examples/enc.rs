//! Temporary debug helper: send an encrypted request.
//!
//! `enc <path> [key=value ...]` GETs with those parameters inside the encrypted
//! query, `enc <path> <json>` POSTs.
#![allow(clippy::print_stdout)]
use ugos_client::{Credentials, UgosClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let creds = Credentials {
        username: std::env::var("UGOS_USER")?,
        password: std::env::var("UGOS_PASSWORD")?,
        otp: env_nonempty("UGOS_OTP"),
        otp_secret: env_nonempty("UGOS_OTP_SECRET"),
    };
    let client = UgosClient::connect(
        &std::env::var("UGOS_HOST")?,
        9443,
        creds,
        &ugos_client::TlsPolicy::Insecure,
    )
    .await?;
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        return Err("usage: enc <path> [json-body]".into());
    };
    let rest: Vec<String> = args.collect();
    let value: serde_json::Value = if rest.first().is_some_and(|arg| arg.starts_with('{')) {
        let body: serde_json::Value = serde_json::from_str(&rest[0])?;
        client.post_encrypted(&path, &body).await?
    } else {
        let params: Vec<(&str, &str)> = rest
            .iter()
            .map(|pair| pair.split_once('=').unwrap_or((pair.as_str(), "")))
            .collect();
        client.get_encrypted(&path, &params).await?
    };
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}

fn env_nonempty(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}
