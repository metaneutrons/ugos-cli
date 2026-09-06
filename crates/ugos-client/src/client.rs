//! Core HTTP client for the UGOS API.
//!
//! [`UgosClient`] wraps a [`reqwest::Client`] with automatic token
//! management and transparent re-authentication on token expiry (code 1024).

use std::future::Future;
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
    /// Serialises re-authentication so one expiry causes one login.
    reauth: Arc<tokio::sync::Mutex<()>>,
}

impl UgosClient {
    /// Create a new client and authenticate.
    ///
    /// Builds a reqwest client that enforces `tls`, then performs the full
    /// login flow. See [`crate::tls::TlsPolicy`] for what the policies mean.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP client cannot be built or login fails.
    pub async fn connect(
        host: &str,
        port: u16,
        creds: Credentials,
        tls: &crate::tls::TlsPolicy,
    ) -> Result<Self> {
        let http = crate::tls::http_client(tls)?;

        let base_url = format!("https://{host}:{port}/ugreen");
        let session = auth::login(&http, &base_url, &creds).await?;

        Ok(Self {
            http,
            base_url,
            creds,
            session: Arc::new(RwLock::new(session)),
            public_key: Arc::new(RwLock::new(None)),
            reauth: Arc::new(tokio::sync::Mutex::new(())),
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
        tls: &crate::tls::TlsPolicy,
    ) -> Result<Self> {
        let http = crate::tls::http_client(tls)?;

        let base_url = format!("https://{host}:{port}/ugreen");

        Ok(Self {
            http,
            base_url,
            creds,
            session: Arc::new(RwLock::new(session)),
            public_key: Arc::new(RwLock::new(None)),
            reauth: Arc::new(tokio::sync::Mutex::new(())),
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
    ///
    /// The token is percent-encoded. It is an opaque server-issued string,
    /// and a `&` in it would split the query, a `#` would drop everything
    /// after it, a `+` would arrive as a space.
    pub(crate) fn append_token(url: &str, token: &str) -> String {
        let token = crate::crypto::urlencode_component(token);
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
        self.retrying(|| self.do_get::<T>(path, params)).await
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
        self.retrying(|| self.do_post::<T, B>(path, body)).await
    }

    /// Perform a PUT request with a JSON body.
    ///
    /// The snapshot app is the only part of UGOS with a REST-shaped API;
    /// everything else posts to verb-named endpoints.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on API or network failure.
    pub async fn put<T: DeserializeOwned + Send, B: serde::Serialize + Sync>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        self.retrying(|| self.do_request::<T, B>(reqwest::Method::PUT, path, body))
            .await
    }

    /// Perform a DELETE request that carries a JSON body.
    ///
    /// Unusual, but the snapshot app deletes by posting a list of ids in the
    /// body rather than naming one in the path.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on API or network failure.
    pub async fn delete_with_body<T: DeserializeOwned + Send, B: serde::Serialize + Sync>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        self.retrying(|| self.do_request::<T, B>(reqwest::Method::DELETE, path, body))
            .await
    }

    /// Internal request with a JSON body for methods other than POST.
    async fn do_request<T: DeserializeOwned + Send, B: serde::Serialize + Sync>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let token = self.session.read().await.token.clone();
        let url = Self::append_token(&self.url_for(path), &token);

        let resp: ApiResponse<serde_json::Value> = self
            .http
            .request(method, &url)
            .json(body)
            .send()
            .await?
            .json()
            .await?;
        Self::decode(resp)
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
        // A multipart::Form cannot be cloned, so no retrying() here: on
        // expiry the client logs in once more and the caller sends the form
        // again.
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
        self.retrying(|| self.do_get_bytes(path, params)).await
    }

    /// One attempt at a byte GET.
    ///
    /// UGOS answers a failed download with HTTP 200 and the ordinary
    /// `{code, msg}` envelope. Only the status was checked here, so an
    /// expired session ended up written into the target file and reported as
    /// a successful download. A JSON content type is now read back and
    /// decoded; anything that is not an error envelope is handed on
    /// unchanged, because a downloaded file may legitimately be JSON.
    async fn do_get_bytes(&self, path: &str, params: &[(&str, &str)]) -> Result<reqwest::Response> {
        let token = self.session.read().await.token.clone();
        let url = Self::append_token(&self.url_for(path), &token);
        let resp = self
            .http
            .get(&url)
            .query(params)
            .send()
            .await?
            .error_for_status()?;

        let is_json = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.starts_with("application/json"));
        if !is_json {
            return Ok(resp);
        }

        let bytes = resp.bytes().await?;
        if let Ok(envelope) = serde_json::from_slice::<ApiResponse<serde_json::Value>>(&bytes)
            && envelope.code != 200
        {
            // Denselben Weg wie jede andere Antwort: decode() bildet den Code
            // auf die passende Variante ab, 1024 also auf LoginExpired, das
            // retrying() dann als Wiederholungsgrund erkennt.
            return match Self::decode::<serde_json::Value>(envelope) {
                Err(e) => Err(e),
                Ok(_) => Err(UgosError::Api {
                    code: 0,
                    msg: "unexpected envelope in a byte response".into(),
                }),
            };
        }
        // No error envelope: the bytes are the content. They were read for the
        // check and are handed back as a new response.
        Ok(reqwest::Response::from(
            http::Response::builder()
                .status(reqwest::StatusCode::OK)
                .body(bytes)
                .map_err(|e| UgosError::Encryption(format!("rebuilding response: {e}")))?,
        ))
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

    /// Run an operation, and once more if the token expired in between.
    ///
    /// Used to be spelled out four times and was missing from six further
    /// methods. Anyone adding a new kind of request now gets the behaviour by
    /// passing through here.
    async fn retrying<T, F, Fut>(&self, mut op: F) -> Result<T>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        match op().await {
            Err(UgosError::LoginExpired) => {
                self.re_auth().await?;
                op().await
            }
            other => other,
        }
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
        // An expiry hits every request in flight at once. Without this lock
        // each of them logs in separately, and every login can invalidate the
        // previous one's token. Whoever takes the lock logs in; whoever waits
        // checks afterwards whether that is still necessary.
        let _guard = self.reauth.lock().await;
        let token_before = self.session.read().await.token.clone();

        tracing::info!("token expired, re-authenticating");
        let new_session = auth::login(&self.http, &self.base_url, &self.creds).await?;
        {
            let current = self.session.read().await;
            if current.token != token_before {
                // Someone else renewed while this call was waiting.
                return Ok(());
            }
        }
        *self.session.write().await = new_session;
        // The RSA key belongs to the session. If it stayed, every later
        // encrypted request would wrap its AES key with the key of the previous
        // login.
        *self.public_key.write().await = None;
        Ok(())
    }
}
