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
    #[serde(default)]
    token: String,
    /// Set when the account has 2FA enabled; the token is then withheld
    /// until `/verify/code/login` succeeds.
    #[serde(default)]
    enable_otp: bool,
    /// Handle for the pending 2FA step.
    #[serde(default)]
    token_id: String,
    /// Base64-encoded PEM, used for the request encryption scheme.
    #[serde(default)]
    public_key: String,
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

    let resp = client
        .post(&url)
        .header(CLIENT_ID_HEADER, client_id())
        .json(&body)
        .send()
        .await?;

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

/// Header the web client uses to identify the device; UGOS keys trusted
/// devices (2FA "trust this device") on it.
pub const CLIENT_ID_HEADER: &str = "UG-Client-Id";

/// Whether to ask UGOS to trust this device after a 2FA login.
/// On by default; `UGOS_TRUST=0|false|no` disables it.
fn trust_enabled() -> bool {
    !matches!(
        std::env::var("UGOS_TRUST").map(|v| v.trim().to_ascii_lowercase()),
        Ok(v) if v == "0" || v == "false" || v == "no"
    )
}

/// Stable per-installation device id sent as `UG-Client-Id`.
///
/// Taken from `UGOS_CLIENT_ID` when set, otherwise read from (or created
/// at) `<config dir>/ugos-cli/client-id`. A random id is used for this
/// process if the file cannot be written.
pub fn client_id() -> String {
    static ID: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    ID.get_or_init(|| {
        if let Ok(v) = std::env::var("UGOS_CLIENT_ID") {
            let v = v.trim().to_owned();
            if !v.is_empty() {
                return v;
            }
        }
        let path = dirs::config_dir().map(|d| d.join("ugos-cli").join("client-id"));
        if let Some(p) = &path
            && let Ok(v) = std::fs::read_to_string(p)
        {
            let v = v.trim().to_owned();
            if !v.is_empty() {
                return v;
            }
        }
        let id = random_uuid();
        if let Some(p) = &path {
            if let Some(dir) = p.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            if let Err(e) = std::fs::write(p, &id) {
                tracing::warn!("could not persist client id: {e}");
            }
        }
        id
    })
    .clone()
}

/// Random RFC 4122 version-4 UUID string.
fn random_uuid() -> String {
    use rsa::rand_core::RngCore as _;
    let mut b = [0u8; 16];
    rsa::rand_core::OsRng.fill_bytes(&mut b);
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let h = b.iter().fold(String::with_capacity(32), |mut acc, x| {
        use std::fmt::Write as _;
        let _ = write!(acc, "{x:02x}");
        acc
    });
    format!(
        "{}-{}-{}-{}-{}",
        &h[0..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..32]
    )
}

#[cfg(test)]
mod client_id_tests {
    #[test]
    fn uuid_shape() {
        let u = super::random_uuid();
        assert_eq!(u.len(), 36);
        assert_eq!(&u[14..15], "4");
    }
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
    let url = format!("{base_url}/v1/verify/login");
    let body = serde_json::json!({
        "username": creds.username,
        "password": encrypted,
        "keepalive": true,
        // `otp: true` tells UGOS the client can handle a second factor. The
        // web desktop always sends it. UGOS 1.20 rejects `otp: false` for
        // accounts with 2FA enabled with code 9406 ("client is not compatible
        // with the firmware"), which hides the real reason.
        "otp": true,
        "is_simple": false,
    });

    let resp = client
        .post(&url)
        .header(CLIENT_ID_HEADER, client_id())
        .json(&body)
        .send()
        .await?;

    // Check the status code before decoding: a rejected login carries no
    // token, and decoding first would report "missing field `token`" instead
    // of what actually went wrong.
    let api: crate::types::common::ApiResponse<serde_json::Value> = resp.json().await?;
    let mut data: LoginData = serde_json::from_value(api.into_result()?)?;

    if data.token.is_empty() && data.enable_otp {
        data = verify_otp(client, base_url, &data.token_id).await?;
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
            .and_then(|b| String::from_utf8(b).ok())
            .unwrap_or_default()
    };

    Ok(Session {
        token: data.token,
        public_key,
    })
}

/// Complete a 2FA login with the authenticator code from `UGOS_OTP`.
async fn verify_otp(client: &reqwest::Client, base_url: &str, token_id: &str) -> Result<LoginData> {
    let code = std::env::var("UGOS_OTP")
        .ok()
        .map(|c| c.trim().to_owned())
        .filter(|c| !c.is_empty())
        .ok_or(UgosError::OtpRequired)?;
    let url = format!("{base_url}/v1/verify/code/login");
    let body = serde_json::json!({
        "code": code,
        "type": 1,
        "token_id": token_id,
        "trust_info": {"client_type": "web", "system": "ugos-cli", "dev_name": "ugos-cli"},
        // With `trust: true` UGOS remembers this `UG-Client-Id`, and later
        // logins from it skip the second factor, like "trust this device"
        // in the web UI. Set `UGOS_TRUST=0` to opt out.
        "trust": trust_enabled(),
    });
    let resp = client
        .post(&url)
        .header(CLIENT_ID_HEADER, client_id())
        .json(&body)
        .send()
        .await?;
    let api: crate::types::common::ApiResponse<serde_json::Value> = resp.json().await?;
    Ok(serde_json::from_value(api.into_result()?)?)
}
