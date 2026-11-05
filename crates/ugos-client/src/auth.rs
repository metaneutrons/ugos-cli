//! Authentication flow for the UGOS API.
//!
//! Implements the auth handshake:
//! 1. Fetch RSA public key via `POST /verify/check`
//! 2. Encrypt password with PKCS1v1.5
//! 3. Login via `POST /verify/login` → session token, or a pending second factor
//! 4. When two-factor authentication is on, `POST /verify/code/login` with an
//!    authenticator code (type 1)

use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use rsa::{Pkcs1v15Encrypt, RsaPublicKey, pkcs1::DecodeRsaPublicKey, pkcs8::DecodePublicKey};
use serde::{Deserialize, Serialize};

use crate::error::{Result, UgosError};

/// Login credentials for a UGOS NAS.
///
/// `otp` is one authenticator code, sent as-is. It is spent on the first
/// login, so the re-authentication that follows session expiry (about 25
/// minutes) cannot use it again.
///
/// `otp_secret` is a TOTP secret: base32, or an `otpauth://totp/` URI whose
/// `digits` and `period` are honoured (HMAC-SHA1, 6 digits and 30 seconds
/// when they are absent). A fresh code is generated on every login, which is
/// what a long-running process needs. When both are set, the secret is used.
///
/// Only authenticator apps (UGOS `type` 1) are supported.
#[derive(Clone)]
pub struct Credentials {
    /// Username (e.g. "fabian").
    pub username: String,
    /// Plaintext password.
    pub password: String,
    /// One-time authenticator code. `None` when the account has no second factor.
    pub otp: Option<String>,
    /// TOTP secret (base32 or an `otpauth://totp/` URI). A code is generated per login.
    pub otp_secret: Option<String>,
}

impl std::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Credentials")
            .field("username", &self.username)
            .field("password", &"[REDACTED]")
            .field("otp", &self.otp.as_ref().map(|_| "[REDACTED]"))
            .field(
                "otp_secret",
                &self.otp_secret.as_ref().map(|_| "[REDACTED]"),
            )
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
    /// Handle for the pending second factor. Present when `enable_otp` is set
    /// and the token is withheld.
    #[serde(default)]
    token_id: String,
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
/// that still needs a second factor. That is [`UgosError::OtpRequired`],
/// rather than a missing token or API error 9406.
fn session_from_login(data: LoginData) -> Result<Session> {
    if data.enable_otp && data.token.is_empty() {
        return Err(UgosError::OtpRequired);
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
/// Returns [`UgosError::OtpRequired`] when the account has a second factor
/// and [`Credentials`] carries neither a code nor a secret.
/// Returns [`UgosError::InvalidOtpSecret`] when the secret cannot be parsed.
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
    if data.enable_otp && data.token.is_empty() {
        return complete_second_factor(client, base_url, creds, &data).await;
    }
    session_from_login(data)
}

/// Code to send for this login.
///
/// A secret wins over a one-shot code: the secret can be turned into a new
/// code every time, and the one-shot code cannot.
fn one_time_code(creds: &Credentials) -> Result<Option<String>> {
    if let Some(secret) = nonempty(creds.otp_secret.as_deref()) {
        let totp = crate::totp::parse_secret(secret).map_err(UgosError::InvalidOtpSecret)?;
        return Ok(Some(crate::totp::code_at(&totp, crate::totp::unix_now()?)));
    }
    Ok(nonempty(creds.otp.as_deref()).map(ToOwned::to_owned))
}

fn nonempty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|text| !text.is_empty())
}

/// Finish a login that came back with `enable_otp` and no token.
///
/// The encryption key often arrives on the password response and is absent
/// from `/verify/code/login`. Dropping it would leave the session without a
/// key, and the next encrypted call would log in again to fetch one. The
/// first key is kept unless the second response has its own.
async fn complete_second_factor(
    client: &reqwest::Client,
    base_url: &str,
    creds: &Credentials,
    first: &LoginData,
) -> Result<Session> {
    let code = one_time_code(creds)?.ok_or(UgosError::OtpRequired)?;
    let second = submit_otp_code(client, base_url, &first.token_id, &code).await?;
    let public_key = if second.public_key.is_empty() {
        first.public_key.clone()
    } else {
        second.public_key
    };
    // `enable_otp: false` so an empty token is reported as a missing token,
    // not as another request for a second factor we just submitted.
    session_from_login(LoginData {
        token: second.token,
        enable_otp: false,
        token_id: String::new(),
        public_key,
    })
}

/// `POST /v1/verify/code/login`.
///
/// The body follows the web client's `twoFAVerify` call: authenticator app
/// (`type` 1), and `trust: false` so this process is not registered as a
/// trusted device. It has not been confirmed against a live capture.
async fn submit_otp_code(
    client: &reqwest::Client,
    base_url: &str,
    token_id: &str,
    code: &str,
) -> Result<LoginData> {
    let url = format!("{base_url}/v1/verify/code/login");
    let body = serde_json::json!({
        "code": code,
        "type": 1,
        "token_id": token_id,
        "trust": false,
        "trust_info": {
            "client_type": "web",
            "system": "ugos-cli",
            "dev_name": "ugos-cli",
        },
    });
    let resp = client.post(&url).json(&body).send().await?;
    let api: crate::types::common::ApiResponse<serde_json::Value> = resp.json().await?;
    Ok(serde_json::from_value(api.into_result()?)?)
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
        assert!(matches!(err, UgosError::OtpRequired));
        assert_eq!(
            err.to_string(),
            "two-factor authentication is enabled for this account; pass the current authenticator code or a TOTP secret"
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
        assert!(matches!(err, UgosError::OtpRequired));
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
        assert_eq!(session.public_key, "");
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod login_flow {
    use super::login;
    use crate::Credentials;
    use crate::error::UgosError;
    use crate::test_support::{Mock, Script, SecondKey, lock};
    use crate::totp::{self, RFC6238_SHA1_SECRET_BASE32};

    const FIRST_PEM: &str = "-----BEGIN PUBLIC KEY-----\nFIRST\n-----END PUBLIC KEY-----\n";
    const SECOND_PEM: &str = "-----BEGIN PUBLIC KEY-----\nSECOND\n-----END PUBLIC KEY-----\n";

    fn creds(otp: Option<&str>, secret: Option<&str>) -> Credentials {
        Credentials {
            username: "admin".into(),
            password: "secret".into(),
            otp: otp.map(str::to_owned),
            otp_secret: secret.map(str::to_owned),
        }
    }

    fn script(requires_otp: bool, second: SecondKey) -> Script {
        Script {
            requires_otp,
            first_public_key: FIRST_PEM.into(),
            second_public_key: second,
            token_id: "pending".into(),
            expire_once: false,
            advance_clock_to: None,
        }
    }

    fn code_was_expected(secret: &str, unix: u64, code: &str) -> bool {
        let totp = totp::parse_secret(secret).unwrap();
        let current = totp::code_at(&totp, unix);
        let previous = unix
            .checked_sub(totp.period())
            .is_some_and(|earlier| totp::code_at(&totp, earlier) == code);
        code == current || previous
    }

    #[tokio::test]
    async fn a_login_without_two_factor_returns_the_token_and_the_key() {
        let _guard = lock().await;
        let mock = Mock::spawn(script(false, SecondKey::Omit));
        let session = login(&http(), mock.base_url(), &creds(None, None))
            .await
            .unwrap();
        assert_eq!(session.token, "token-0");
        assert_eq!(session.public_key, FIRST_PEM);
        assert!(mock.ending_with("/verify/code/login").is_empty());
        let posted = &mock.ending_with("/verify/login")[0];
        assert_eq!(posted.body["otp"], true);
        assert_eq!(posted.body["is_simple"], false);
        assert_eq!(posted.body["username"], "admin");
    }

    #[tokio::test]
    async fn two_factor_without_a_code_or_secret_stops_before_the_second_call() {
        let _guard = lock().await;
        let mock = Mock::spawn(script(true, SecondKey::Omit));
        let err = login(&http(), mock.base_url(), &creds(None, None))
            .await
            .unwrap_err();
        assert!(matches!(err, UgosError::OtpRequired));
        assert_eq!(
            err.to_string(),
            "two-factor authentication is enabled for this account; pass the current authenticator code or a TOTP secret"
        );
        assert!(mock.ending_with("/verify/code/login").is_empty());
    }

    #[tokio::test]
    async fn a_one_shot_code_is_sent_with_type_1_and_without_trusting_the_device() {
        let _guard = lock().await;
        let mock = Mock::spawn(script(true, SecondKey::Omit));
        let session = login(&http(), mock.base_url(), &creds(Some("123456"), None))
            .await
            .unwrap();
        assert_eq!(session.token, "token-0");
        let hits = mock.ending_with("/verify/code/login");
        assert_eq!(hits.len(), 1);
        let body = &hits[0].body;
        assert_eq!(body["code"], "123456");
        assert_eq!(body["type"], 1);
        assert_eq!(body["token_id"], "pending");
        assert_eq!(body["trust"], false);
        assert_eq!(body["trust_info"]["dev_name"], "ugos-cli");
        assert_eq!(body.as_object().unwrap().len(), 5);
        assert!(
            !hits[0]
                .headers
                .to_ascii_lowercase()
                .contains("ug-client-id")
        );
    }

    #[tokio::test]
    async fn a_secret_generates_the_code_and_wins_over_a_one_shot_code() {
        let _guard = lock().await;
        let mock = Mock::spawn(script(true, SecondKey::Omit));
        let session = login(
            &http(),
            mock.base_url(),
            &creds(Some("111111"), Some(RFC6238_SHA1_SECRET_BASE32)),
        )
        .await
        .unwrap();
        assert_eq!(session.token, "token-0");
        let hit = &mock.ending_with("/verify/code/login")[0];
        let code = hit.body["code"].as_str().unwrap();
        assert_ne!(code, "111111");
        assert!(code_was_expected(
            RFC6238_SHA1_SECRET_BASE32,
            hit.unix,
            code
        ));
    }

    #[tokio::test]
    async fn an_invalid_secret_is_rejected_before_the_second_call() {
        let _guard = lock().await;
        let mock = Mock::spawn(script(true, SecondKey::Omit));
        let err = login(&http(), mock.base_url(), &creds(None, Some("!!!!!!!!")))
            .await
            .unwrap_err();
        assert!(matches!(err, UgosError::InvalidOtpSecret(_)));
        assert!(mock.ending_with("/verify/code/login").is_empty());
    }

    #[tokio::test]
    async fn a_missing_public_key_on_the_second_response_keeps_the_first() {
        let _guard = lock().await;
        let mock = Mock::spawn(script(true, SecondKey::Omit));
        let session = login(&http(), mock.base_url(), &creds(Some("123456"), None))
            .await
            .unwrap();
        assert_eq!(session.public_key, FIRST_PEM);
    }

    #[tokio::test]
    async fn an_empty_public_key_on_the_second_response_keeps_the_first() {
        let _guard = lock().await;
        let mock = Mock::spawn(script(true, SecondKey::Empty));
        let session = login(&http(), mock.base_url(), &creds(Some("123456"), None))
            .await
            .unwrap();
        assert_eq!(session.public_key, FIRST_PEM);
    }

    #[tokio::test]
    async fn a_public_key_on_the_second_response_replaces_the_first() {
        let _guard = lock().await;
        let mock = Mock::spawn(script(true, SecondKey::Pem(SECOND_PEM.into())));
        let session = login(&http(), mock.base_url(), &creds(Some("123456"), None))
            .await
            .unwrap();
        assert_eq!(session.public_key, SECOND_PEM);
    }

    fn http() -> reqwest::Client {
        crate::tls::http_client(&crate::tls::TlsPolicy::Insecure).unwrap()
    }
}
