//! Core HTTP client for the UGOS API.
//!
//! [`UgosClient`] wraps a [`reqwest::Client`] with automatic token
//! management and transparent re-authentication on token expiry (code 1024).

use std::sync::Arc;

use serde::de::DeserializeOwned;
use tokio::sync::RwLock;

use crate::auth::{self, Credentials, Session};
use crate::crypto::{RequestKey, md5_hex, query_string, rsa_seal, sha256_hex};
use crate::error::{Result, UgosError};
use crate::types::common::ApiResponse;

/// Client for interacting with a UGOS NAS.
#[derive(Debug, Clone)]
pub struct UgosClient {
    http: reqwest::Client,
    base_url: String,
    creds: Credentials,
    session: Arc<RwLock<Session>>,
    /// Cached RSA key, fetched on first use by an encrypted request.
    public_key: Arc<RwLock<Option<rsa::RsaPublicKey>>>,
}

impl UgosClient {
    /// Create a new client and authenticate.
    ///
    /// Builds a reqwest client with cookie storage and self-signed cert
    /// support, then performs the full login flow.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP client cannot be built or login fails.
    pub async fn connect(host: &str, port: u16, creds: Credentials) -> Result<Self> {
        let http = reqwest::Client::builder()
            .cookie_store(true)
            .danger_accept_invalid_certs(true)
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| UgosError::Encryption(format!("HTTP client build: {e}")))?;

        let base_url = format!("https://{host}:{port}/ugreen");
        let session = auth::login(&http, &base_url, &creds).await?;

        Ok(Self {
            http,
            base_url,
            creds,
            session: Arc::new(RwLock::new(session)),
            public_key: Arc::new(RwLock::new(None)),
        })
    }

    /// Create a client from an existing session (e.g. loaded from cache).
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP client cannot be built.
    pub fn from_session(
        host: &str,
        port: u16,
        creds: Credentials,
        session: Session,
    ) -> Result<Self> {
        let http = reqwest::Client::builder()
            .cookie_store(true)
            .danger_accept_invalid_certs(true)
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| UgosError::Encryption(format!("HTTP client build: {e}")))?;

        let base_url = format!("https://{host}:{port}/ugreen");

        Ok(Self {
            http,
            base_url,
            creds,
            session: Arc::new(RwLock::new(session)),
            public_key: Arc::new(RwLock::new(None)),
        })
    }

    /// The current session token.
    pub async fn session(&self) -> Session {
        self.session.read().await.clone()
    }

    /// Build a full URL for an API path.
    ///
    /// A path may name its API version (`v2/filemgr/...`); anything else gets
    /// `v1/`, which is what the KVM and Docker apps use.
    fn url_for(&self, path: &str) -> String {
        if path.starts_with("v1/") || path.starts_with("v2/") {
            format!("{}/{path}", self.base_url)
        } else {
            format!("{}/v1/{path}", self.base_url)
        }
    }

    /// Append `?token=` (or `&token=`) to a URL.
    pub(crate) fn append_token(url: &str, token: &str) -> String {
        if url.contains('?') {
            format!("{url}&token={token}")
        } else {
            format!("{url}?token={token}")
        }
    }

    /// Perform a GET request, deserialize the [`ApiResponse`], and check the status code.
    /// Automatically retries once on token expiry (code 1024).
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on API or network failure.
    pub async fn get<T: DeserializeOwned + Send>(&self, path: &str) -> Result<T> {
        self.get_with_params(path, &[]).await
    }

    /// Perform a GET request with query parameters.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on API or network failure.
    pub async fn get_with_params<T: DeserializeOwned + Send>(
        &self,
        path: &str,
        params: &[(&str, &str)],
    ) -> Result<T> {
        let result = self.do_get::<T>(path, params).await;

        if matches!(&result, Err(UgosError::LoginExpired)) {
            self.re_auth().await?;
            return self.do_get(path, params).await;
        }

        result
    }

    /// Perform a POST request with a JSON body.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on API or network failure.
    pub async fn post<T: DeserializeOwned + Send, B: serde::Serialize + Sync>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let result = self.do_post::<T, B>(path, body).await;

        if matches!(&result, Err(UgosError::LoginExpired)) {
            self.re_auth().await?;
            return self.do_post(path, body).await;
        }

        result
    }

    /// Internal GET without retry.
    async fn do_get<T: DeserializeOwned + Send>(
        &self,
        path: &str,
        params: &[(&str, &str)],
    ) -> Result<T> {
        let token = self.session.read().await.token.clone();
        let url = Self::append_token(&self.url_for(path), &token);

        let resp: ApiResponse<serde_json::Value> = self
            .http
            .get(&url)
            .query(params)
            .send()
            .await?
            .json()
            .await?;
        Self::decode(resp)
    }

    /// Internal POST without retry.
    async fn do_post<T: DeserializeOwned + Send, B: serde::Serialize + Sync>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let token = self.session.read().await.token.clone();
        let url = Self::append_token(&self.url_for(path), &token);

        let resp: ApiResponse<serde_json::Value> =
            self.http.post(&url).json(body).send().await?.json().await?;
        Self::decode(resp)
    }

    /// Perform a POST request with a `multipart/form-data` body.
    ///
    /// Used for chunked uploads, which is the only place UGOS expects a
    /// multipart body rather than JSON.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on API or network failure.
    pub async fn post_multipart<T: DeserializeOwned + Send>(
        &self,
        path: &str,
        form: reqwest::multipart::Form,
    ) -> Result<T> {
        let token = self.session.read().await.token.clone();
        let url = Self::append_token(&self.url_for(path), &token);

        let resp: ApiResponse<serde_json::Value> = self
            .http
            .post(&url)
            .multipart(form)
            .send()
            .await?
            .json()
            .await?;
        Self::decode(resp)
    }

    /// GET raw bytes, for file downloads that answer with the file itself.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on network failure or a non-2xx
    /// status.
    pub async fn get_bytes(
        &self,
        path: &str,
        params: &[(&str, &str)],
    ) -> Result<reqwest::Response> {
        let token = self.session.read().await.token.clone();
        let url = Self::append_token(&self.url_for(path), &token);
        Ok(self
            .http
            .get(&url)
            .query(params)
            .send()
            .await?
            .error_for_status()?)
    }

    /// POST a raw body, which is how the file manager takes file contents.
    ///
    /// The transfer metadata rides in a `ug-param` header rather than in the
    /// body, and the token is sealed the way encrypted endpoints expect it.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on API, crypto or network failure.
    pub async fn post_bytes<T: DeserializeOwned + Send>(
        &self,
        path: &str,
        body: Vec<u8>,
        file_name: &str,
        ug_param: &serde_json::Value,
    ) -> Result<T> {
        let public_key = self.public_key().await?;
        let token = self.session.read().await.token.clone();

        let resp: ApiResponse<serde_json::Value> = self
            .http
            .post(self.url_for(path))
            .header(
                "Content-Disposition",
                format!("attachment; filename=\"{file_name}\""),
            )
            .header("Content-Type", "application/octet-stream")
            .header("ug-param", ug_param.to_string())
            .header("X-Ugreen-Security-Key", md5_hex(&token))
            .header("X-Ugreen-Token", rsa_seal(&public_key, &token)?)
            .body(body)
            .send()
            .await?
            .json()
            .await?;
        Self::decode(resp)
    }

    /// The RSA key used to wrap per-request AES keys.
    ///
    /// It comes from the login response, not from the `verify/check`
    /// handshake — those are different keys. A session restored from an older
    /// cache has none, in which case a fresh login provides it.
    async fn public_key(&self) -> Result<rsa::RsaPublicKey> {
        let cached = self.public_key.read().await.clone();
        if let Some(key) = cached {
            return Ok(key);
        }

        let mut pem = self.session.read().await.public_key.clone();
        if pem.is_empty() {
            let session = auth::login(&self.http, &self.base_url, &self.creds).await?;
            pem = session.public_key.clone();
            *self.session.write().await = session;
        }
        if pem.is_empty() {
            return Err(UgosError::Encryption(
                "NAS did not provide an encryption key at login".into(),
            ));
        }

        let key = auth::parse_public_key(&pem)?;
        *self.public_key.write().await = Some(key.clone());
        Ok(key)
    }

    /// Perform an encrypted GET, for endpoints that reject plain requests.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on API, crypto or network failure.
    pub async fn get_encrypted<T: DeserializeOwned + Send>(
        &self,
        path: &str,
        params: &[(&str, &str)],
    ) -> Result<T> {
        let public_key = self.public_key().await?;
        let key = RequestKey::generate(&public_key)?;
        let token = self.session.read().await.token.clone();

        let raw: serde_json::Value = self
            .http
            .get(self.url_for(path))
            // The token rides inside the encrypted query, and RSA-sealed in
            // the header alongside an MD5 of it.
            .query(&[(
                "encrypt_query",
                key.encrypt(&Self::with_token(params, &token))?,
            )])
            .header("X-Ugreen-Security-Code", &key.security_code)
            .header("X-Ugreen-Security-Key", md5_hex(&token))
            .header("X-Ugreen-Token", rsa_seal(&public_key, &token)?)
            .send()
            .await?
            .json()
            .await?;
        Self::decode_encrypted(raw, &key)
    }

    /// Perform an encrypted DELETE, which the Download Center uses.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on API, crypto or network failure.
    pub async fn delete_encrypted<T: DeserializeOwned + Send>(
        &self,
        path: &str,
        params: &[(&str, &str)],
    ) -> Result<T> {
        let public_key = self.public_key().await?;
        let key = RequestKey::generate(&public_key)?;
        let token = self.session.read().await.token.clone();

        let raw: serde_json::Value = self
            .http
            .delete(self.url_for(path))
            .query(&[(
                "encrypt_query",
                key.encrypt(&Self::with_token(params, &token))?,
            )])
            .header("X-Ugreen-Security-Code", &key.security_code)
            .header("X-Ugreen-Security-Key", md5_hex(&token))
            .header("X-Ugreen-Token", rsa_seal(&public_key, &token)?)
            .send()
            .await?
            .json()
            .await?;
        Self::decode_encrypted(raw, &key)
    }

    /// Perform an encrypted POST, for endpoints that reject plain requests.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on API, crypto or network failure.
    pub async fn post_encrypted<T: DeserializeOwned + Send, B: serde::Serialize + Sync>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let public_key = self.public_key().await?;
        let key = RequestKey::generate(&public_key)?;
        let token = self.session.read().await.token.clone();
        let json = serde_json::to_string(body)?;

        let payload = serde_json::json!({
            "encrypt_req_body": key.encrypt(&json)?,
            "req_body_sha256": sha256_hex(&json),
        });

        let raw: serde_json::Value = self
            .http
            .post(self.url_for(path))
            .query(&[(
                "encrypt_query",
                key.encrypt(&Self::with_token(&[], &token))?,
            )])
            .header("X-Ugreen-Security-Code", &key.security_code)
            .header("X-Ugreen-Security-Key", md5_hex(&token))
            .header("X-Ugreen-Token", rsa_seal(&public_key, &token)?)
            .json(&payload)
            .send()
            .await?
            .json()
            .await?;
        Self::decode_encrypted(raw, &key)
    }

    /// Render params with the session token appended.
    fn with_token(params: &[(&str, &str)], token: &str) -> String {
        let mut all: Vec<(&str, &str)> = params.to_vec();
        all.push(("token", token));
        query_string(&all)
    }

    /// Unwrap an encrypted response.
    ///
    /// An encrypted answer arrives as a bare `{"encrypt_resp_body": "..."}`
    /// without the usual `code`/`msg`/`data` envelope; the envelope is inside.
    /// A plain answer keeps the envelope, so both shapes are handled.
    fn decode_encrypted<T: DeserializeOwned>(
        raw: serde_json::Value,
        key: &RequestKey,
    ) -> Result<T> {
        let envelope: ApiResponse<serde_json::Value> =
            if let Some(sealed) = raw.get("encrypt_resp_body").and_then(|v| v.as_str()) {
                serde_json::from_str(&key.decrypt(sealed)?)?
            } else {
                serde_json::from_value(raw)?
            };
        Ok(serde_json::from_value(envelope.into_result()?)?)
    }

    /// Check the status code first, then deserialize the payload.
    ///
    /// The order matters: on an error code UGOS sends a `data` payload that
    /// does not match the success shape, so decoding first would report a
    /// deserialization failure and swallow the actual error message.
    fn decode<T: DeserializeOwned>(resp: ApiResponse<serde_json::Value>) -> Result<T> {
        Ok(serde_json::from_value(resp.into_result()?)?)
    }

    /// Re-authenticate and update the stored session.
    async fn re_auth(&self) -> Result<()> {
        tracing::info!("token expired, re-authenticating");
        let new_session = auth::login(&self.http, &self.base_url, &self.creds).await?;
        *self.session.write().await = new_session;
        Ok(())
    }
}
