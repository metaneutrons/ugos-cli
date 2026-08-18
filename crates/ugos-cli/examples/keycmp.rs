//! Temporary: print the login public key so it can be compared with the UI's.
#![allow(clippy::print_stdout)]
use ugos_client::{Credentials, UgosClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let creds = Credentials {
        username: std::env::var("UGOS_USER")?,
        password: std::env::var("UGOS_PASSWORD")?,
    };
    let client = UgosClient::connect(&std::env::var("UGOS_HOST")?, 9443, creds).await?;
    print!("{}", client.session().await.public_key);
    Ok(())
}
