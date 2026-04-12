//! Authentication flow for the UGOS API.
//!
//! Implements the three-step auth handshake:
//! 1. Fetch RSA public key via `POST /verify/check`
//! 2. Encrypt password with PKCS1v1.5
//! 3. Login via `POST /verify/login` → obtain session token + cookies

use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use rsa::{Pkcs1v15Encrypt, RsaPublicKey, pkcs1::DecodeRsaPublicKey, pkcs8::DecodePublicKey};
use serde::{Deserialize, Serialize};

use crate::error::{Result, UgosError};

/// Credentials needed to authenticate with a UGOS NAS.
#[derive(Debug, Clone)]
pub struct Credentials {
    /// Username (e.g. "fabian").
    pub username: String,
    /// Plaintext password.
    pub password: String,
}

/// Session data returned after successful login.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// The session token appended as `?token=` to every request.
    pub token: String,
}

/// Login response payload from `/verify/login`.
#[derive(Debug, Deserialize)]
struct LoginData {
    token: String,
}

/// Fetch the RSA public key from the NAS.
///
/// # Errors
///
/// Returns [`UgosError::Encryption`] if the key header is missing or malformed.
/// Returns [`UgosError::Http`] on network failure.
pub async fn fetch_rsa_key(
    client: &reqwest::Client,
    base_url: &str,
    username: &str,
) -> Result<RsaPublicKey> {
    let url = format!("{base_url}/verify/check");
    let body = serde_json::json!({"username": username});

    let resp = client.post(&url).json(&body).send().await?;

    let header = resp
        .headers()
        .get("x-rsa-token")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| UgosError::Encryption("missing x-rsa-token header".into()))?;

    let pem_bytes = B64
        .decode(header)
        .map_err(|e| UgosError::Encryption(format!("base64 decode: {e}")))?;
    let pem_str = String::from_utf8(pem_bytes)
        .map_err(|e| UgosError::Encryption(format!("PEM not UTF-8: {e}")))?;

    RsaPublicKey::from_pkcs1_pem(&pem_str)
        .or_else(|_| RsaPublicKey::from_public_key_pem(&pem_str))
        .or_else(|_| {
            // UGOS sometimes wraps SPKI DER in a "BEGIN RSA PUBLIC KEY" header.
            // Re-label and try as SPKI.
            let relabeled = pem_str
                .replace("BEGIN RSA PUBLIC KEY", "BEGIN PUBLIC KEY")
                .replace("END RSA PUBLIC KEY", "END PUBLIC KEY");
            RsaPublicKey::from_public_key_pem(&relabeled)
        })
        .map_err(|e| UgosError::Encryption(format!("RSA key parse: {e}")))
}

/// Encrypt a password with the given RSA public key using PKCS1v1.5 padding.
///
/// # Errors
///
/// Returns [`UgosError::Encryption`] if RSA encryption fails.
pub fn encrypt_password(key: &RsaPublicKey, password: &str) -> Result<String> {
    let mut rng = rsa::rand_core::OsRng;
    let ciphertext = key
        .encrypt(&mut rng, Pkcs1v15Encrypt, password.as_bytes())
        .map_err(|e| UgosError::Encryption(format!("RSA encrypt: {e}")))?;
    Ok(B64.encode(ciphertext))
}

/// Perform the full login flow and return a [`Session`].
///
/// The `client` must have `cookie_store(true)` so cookies are captured
/// automatically from the `Set-Cookie` response headers.
///
/// # Errors
///
/// Returns [`UgosError::AuthFailed`] on bad credentials.
/// Returns [`UgosError::Encryption`] on RSA failures.
/// Returns [`UgosError::Http`] on network failures.
pub async fn login(
    client: &reqwest::Client,
    base_url: &str,
    creds: &Credentials,
) -> Result<Session> {
    // Step 1: fetch RSA public key
    let pubkey = fetch_rsa_key(client, base_url, &creds.username).await?;

    // Step 2: encrypt password
    let encrypted = encrypt_password(&pubkey, &creds.password)?;

    // Step 3: POST /verify/login
    let url = format!("{base_url}/verify/login");
    let body = serde_json::json!({
        "username": creds.username,
        "password": encrypted,
        "keepalive": true,
        "otp": false,
    });

    let resp = client.post(&url).json(&body).send().await?;
    let api: crate::types::common::ApiResponse<LoginData> = resp.json().await?;
    let data = api.into_result()?;

    Ok(Session { token: data.token })
}
