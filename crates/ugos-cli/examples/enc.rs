//! Temporary debug helper: send an encrypted request.
//!
//! `enc <path>` GETs, `enc <path> <json>` POSTs.
#![allow(clippy::print_stdout)]
use ugos_client::{Credentials, UgosClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let creds = Credentials {
        username: std::env::var("UGOS_USER")?,
        password: std::env::var("UGOS_PASSWORD")?,
    };
    let client = UgosClient::connect(&std::env::var("UGOS_HOST")?, 9443, creds).await?;
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        return Err("usage: enc <path> [json-body]".into());
    };
    let value: serde_json::Value = match args.next() {
        Some(body) => {
            let body: serde_json::Value = serde_json::from_str(&body)?;
            client.post_encrypted(&path, &body).await?
        }
        None => client.get_encrypted(&path, &[]).await?,
    };
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}
