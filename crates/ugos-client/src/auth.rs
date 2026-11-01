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
/// Login credentials for a UGOS NAS.
#[derive(Clone)]
pub struct Credentials {
    /// Username (e.g. "fabian").
    pub username: String,
    /// Plaintext password.
    pub password: String,
}

impl std::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Credentials")
            .field("username", &self.username)
            .field("password", &"[REDACTED]")
            .finish()
    }
}

/// Session data returned after successful login.
#[derive(Clone, Serialize, Deserialize)]
pub struct Session {
    /// The session token appended as `?token=` to every request.
    pub token: String,
    /// PEM public key the NAS returns at login, used to wrap the per-request
    /// AES key for encrypted endpoints. Absent from cached sessions written
    /// by older versions.
    #[serde(default)]
    pub public_key: String,
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session")
            .field("token", &"[REDACTED]")
            .finish()
    }
}

/// Login response payload from `/verify/login`.
#[derive(Debug, Deserialize)]
struct LoginData {
    /// Session token. Absent when two-factor authentication is on: UGOS
    /// answers `200` with `enable_otp` and withholds the token.
    #[serde(default)]
    token: String,
    /// `true` when the account has two-factor authentication enabled.
    #[serde(default)]
    enable_otp: bool,
    /// Base64-encoded PEM, used for the request encryption scheme.
    #[serde(default)]
    public_key: String,
}

/// Body of `POST /verify/login`.
///
/// `otp: true` and `is_simple: false` match the UGOS 1.20 web client. With
/// `otp: false`, an account that has two-factor authentication enabled is
/// rejected with code 9406 ("client is not compatible with the firmware"),
/// which names the firmware instead of the second factor. Accounts without
/// two-factor authentication accept `otp: true` and still return a token.
fn login_body(username: &str, encrypted_password: &str) -> serde_json::Value {
    serde_json::json!({
        "username": username,
        "password": encrypted_password,
        "keepalive": true,
        "otp": true,
        "is_simple": false,
    })
}

/// Turn a decoded login payload into a session.
///
/// A payload with `enable_otp` and no token is a successful password check
/// that still needs a second factor. That is reported on its own, rather
/// than as a missing token or as API error 9406.
fn session_from_login(data: LoginData) -> Result<Session> {
    if data.enable_otp && data.token.is_empty() {
        return Err(UgosError::TwoFactorUnsupported);
    }
    if data.token.is_empty() {
        return Err(UgosError::Encryption(
            "login response carried no token".into(),
        ));
    }

    // The key arrives base64-encoded; decode it here so callers see PEM.
    let public_key = if data.public_key.is_empty() {
        String::new()
    } else {
        B64.decode(&data.public_key)
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .unwrap_or_default()
    };

    Ok(Session {
        token: data.token,
        public_key,
    })
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
    let url = format!("{base_url}/v1/verify/check");
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

    parse_public_key(&pem_str)
}

/// Parse a PEM public key, tolerating the mislabelled variant UGOS sends.
///
/// # Errors
///
/// Returns [`UgosError::Encryption`] when the PEM cannot be parsed.
pub fn parse_public_key(pem: &str) -> Result<RsaPublicKey> {
    RsaPublicKey::from_pkcs1_pem(pem)
        .or_else(|_| RsaPublicKey::from_public_key_pem(pem))
        .or_else(|_| {
            // UGOS sometimes wraps SPKI DER in a "BEGIN RSA PUBLIC KEY" header.
            // Re-label and try as SPKI.
            let relabeled = pem
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
/// Returns [`UgosError::TwoFactorUnsupported`] when the response has
/// `enable_otp` set and no token.
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
    let url = format!("{base_url}/v1/verify/login");
    let body = login_body(&creds.username, &encrypted);

    let resp = client.post(&url).json(&body).send().await?;

    // Check the status code before decoding: a rejected login carries no
    // token, and decoding first would report "missing field `token`" instead
    // of what actually went wrong.
    let api: crate::types::common::ApiResponse<serde_json::Value> = resp.json().await?;
    let data: LoginData = serde_json::from_value(api.into_result()?)?;
    session_from_login(data)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::{LoginData, login_body, session_from_login};
    use crate::error::UgosError;
    use base64::{Engine as _, engine::general_purpose::STANDARD as B64};

    #[test]
    fn login_body_matches_the_web_client() {
        let body = login_body("admin", "ciphertext");
        assert_eq!(
            body,
            serde_json::json!({
                "username": "admin",
                "password": "ciphertext",
                "keepalive": true,
                "otp": true,
                "is_simple": false,
            })
        );
    }

    #[test]
    fn two_factor_without_a_token_is_a_clear_error() {
        let data: LoginData = serde_json::from_value(serde_json::json!({
            "enable_otp": true,
            "token_id": "pending",
        }))
        .unwrap();
        let err = session_from_login(data).unwrap_err();
        assert!(matches!(err, UgosError::TwoFactorUnsupported));
        assert_eq!(
            err.to_string(),
            "two-factor authentication is enabled; ugos-cli does not support it yet"
        );
    }

    #[test]
    fn an_empty_token_with_two_factor_is_the_same_error() {
        let data: LoginData = serde_json::from_value(serde_json::json!({
            "enable_otp": true,
            "token": "",
        }))
        .unwrap();
        let err = session_from_login(data).unwrap_err();
        assert!(matches!(err, UgosError::TwoFactorUnsupported));
    }

    #[test]
    fn a_missing_token_without_two_factor_is_not_that_error() {
        let data: LoginData = serde_json::from_value(serde_json::json!({
            "public_key": "",
        }))
        .unwrap();
        let err = session_from_login(data).unwrap_err();
        assert!(matches!(
            err,
            UgosError::Encryption(ref message) if message == "login response carried no token"
        ));
    }

    #[test]
    fn a_token_still_opens_a_session() {
        let pem = "-----BEGIN PUBLIC KEY-----\nabc\n-----END PUBLIC KEY-----";
        let data: LoginData = serde_json::from_value(serde_json::json!({
            "token": "session-token",
            "enable_otp": false,
            "public_key": B64.encode(pem),
        }))
        .unwrap();
        let session = session_from_login(data).unwrap();
        assert_eq!(session.token, "session-token");
        assert_eq!(session.public_key, pem);
    }

    #[test]
    fn a_token_is_kept_when_enable_otp_is_also_set() {
        let data: LoginData = serde_json::from_value(serde_json::json!({
            "token": "session-token",
            "enable_otp": true,
        }))
        .unwrap();
        let session = session_from_login(data).unwrap();
        assert_eq!(session.token, "session-token");
        assert!(session.public_key.is_empty());
    }
}
