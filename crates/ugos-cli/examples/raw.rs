//! Temporary debug helper: dump a raw endpoint response.
//!
//! `raw <path>` does a GET, `raw <path> <json-body>` does a POST.
//!
//! Kept as a reverse-engineering aid: the CLI never shows an unmodelled
//! response field, this does.
#![allow(clippy::print_stdout)]
use ugos_client::{Credentials, UgosClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let creds = Credentials {
        username: std::env::var("UGOS_USER")?,
        password: std::env::var("UGOS_PASSWORD")?,
    };
    let client = UgosClient::connect(
        &std::env::var("UGOS_HOST")?,
        9443,
        creds,
        &ugos_client::TlsPolicy::Insecure,
    )
    .await?;
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .unwrap_or_else(|| "kvm/manager/ShowLocalVirtualList".to_owned());
    let value: serde_json::Value = match args.next() {
        Some(body) => {
            let body: serde_json::Value = serde_json::from_str(&body)?;
            client.post(&path, &body).await?
        }
        None => client.get(&path).await?,
    };
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}
