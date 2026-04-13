//! Core HTTP client for the UGOS API.
//!
//! [`UgosClient`] wraps a [`reqwest::Client`] with automatic token
//! management and transparent re-authentication on token expiry (code 1024).

use std::sync::Arc;

use serde::de::DeserializeOwned;
use tokio::sync::RwLock;

use crate::auth::{self, Credentials, Session};
use crate::error::{Result, UgosError};
use crate::types::common::ApiResponse;

/// Client for interacting with a UGOS NAS.
#[derive(Debug, Clone)]
pub struct UgosClient {
    http: reqwest::Client,
    base_url: String,
    creds: Credentials,
    session: Arc<RwLock<Session>>,
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

        let base_url = format!("https://{host}:{port}/ugreen/v1");
        let session = auth::login(&http, &base_url, &creds).await?;

        Ok(Self {
            http,
            base_url,
            creds,
            session: Arc::new(RwLock::new(session)),
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

        let base_url = format!("https://{host}:{port}/ugreen/v1");

        Ok(Self {
            http,
            base_url,
            creds,
            session: Arc::new(RwLock::new(session)),
        })
    }

    /// The current session token.
    pub async fn session(&self) -> Session {
        self.session.read().await.clone()
    }

    /// Append `?token=` (or `&token=`) to a URL.
    fn append_token(url: &str, token: &str) -> String {
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
        let url = Self::append_token(&format!("{}/{path}", self.base_url), &token);

        let resp: ApiResponse<T> = self
            .http
            .get(&url)
            .query(params)
            .send()
            .await?
            .json()
            .await?;
        resp.into_result()
    }

    /// Internal POST without retry.
    async fn do_post<T: DeserializeOwned + Send, B: serde::Serialize + Sync>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let token = self.session.read().await.token.clone();
        let url = Self::append_token(&format!("{}/{path}", self.base_url), &token);

        let resp: ApiResponse<T> = self.http.post(&url).json(body).send().await?.json().await?;
        resp.into_result()
    }

    /// Re-authenticate and update the stored session.
    async fn re_auth(&self) -> Result<()> {
        tracing::info!("token expired, re-authenticating");
        let new_session = auth::login(&self.http, &self.base_url, &self.creds).await?;
        *self.session.write().await = new_session;
        Ok(())
    }
}
