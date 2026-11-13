//! `WebSocket`s of UGOS.
//!
//! The log and terminal views of the Docker app are not request and answer:
//! the web UI opens a WebSocket and the NAS sends frames. A browser cannot set
//! headers on a WebSocket, so the session token travels in the query string as
//! `ugk`. That makes the address a credential, and no error here carries it.

use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tokio_rustls::client::TlsStream;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

use crate::client::UgosClient;
use crate::crypto::urlencode_component;
use crate::error::{Result, UgosError};

/// How long to wait for the connection and the handshake.
const OPEN_TIMEOUT: Duration = Duration::from_secs(15);

fn fail(what: &str) -> UgosError {
    UgosError::Socket(what.to_owned())
}

/// An open WebSocket to the NAS that speaks JSON text frames.
pub struct Socket {
    inner: WebSocketStream<TlsStream<TcpStream>>,
}

impl std::fmt::Debug for Socket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Socket").finish_non_exhaustive()
    }
}

impl Socket {
    /// Open `path` (under `v1/`) with `query`. The session token is added.
    ///
    /// # Errors
    ///
    /// Returns [`UgosError::Socket`] when the connection, the TLS handshake
    /// or the upgrade fails. The message names the step, never the address.
    pub async fn open(client: &UgosClient, path: &str, query: &[(&str, &str)]) -> Result<Self> {
        let (host, port, policy, token) = client.socket_target().await?;
        let mut pairs: Vec<String> = query
            .iter()
            .map(|(key, value)| format!("{key}={}", urlencode_component(value)))
            .collect();
        pairs.push(format!("ugk={}", urlencode_component(&token)));
        let url = format!("wss://{host}:{port}/ugreen/v1/{path}?{}", pairs.join("&"));
        let mut request = url
            .into_client_request()
            .map_err(|_| fail("the address could not be built"))?;
        let origin = format!("https://{host}:{port}")
            .parse()
            .map_err(|_| fail("the origin could not be built"))?;
        let _ = request.headers_mut().insert("Origin", origin);

        let config = crate::tls::client_config(&policy)?;
        let name = rustls::pki_types::ServerName::try_from(host.clone())
            .map_err(|_| fail("the host name is not valid for TLS"))?;
        let open = async {
            let tcp = TcpStream::connect((host.as_str(), port))
                .await
                .map_err(|_| fail("the NAS did not accept the connection"))?;
            let tls = TlsConnector::from(std::sync::Arc::new(config))
                .connect(name, tcp)
                .await
                .map_err(|_| fail("the TLS handshake failed (certificate not the pinned one?)"))?;
            let (inner, _) = tokio_tungstenite::client_async(request, tls)
                .await
                .map_err(|e| {
                    UgosError::Socket(format!("the NAS refused the upgrade: {}", short(&e)))
                })?;
            Ok::<_, UgosError>(Self { inner })
        };
        tokio::time::timeout(OPEN_TIMEOUT, open)
            .await
            .map_err(|_| fail("timed out while connecting"))?
    }

    /// Send one JSON frame.
    ///
    /// # Errors
    ///
    /// Returns [`UgosError::Socket`] when the connection is gone.
    pub async fn send_json(&mut self, value: &Value) -> Result<()> {
        self.inner
            .send(Message::text(value.to_string()))
            .await
            .map_err(|_| fail("the connection is closed"))
    }

    /// The next JSON frame, or `None` when the NAS closed the connection.
    /// Frames that are not JSON text are skipped.
    ///
    /// # Errors
    ///
    /// Returns [`UgosError::Socket`] when the connection breaks.
    pub async fn next_json(&mut self) -> Result<Option<Value>> {
        loop {
            match self.inner.next().await {
                None | Some(Ok(Message::Close(_))) => return Ok(None),
                Some(Err(_)) => return Err(fail("the connection broke")),
                Some(Ok(Message::Text(text))) => {
                    if let Ok(value) = serde_json::from_str(text.as_str()) {
                        return Ok(Some(value));
                    }
                }
                Some(Ok(Message::Binary(bytes))) => {
                    if let Ok(value) = serde_json::from_slice(&bytes) {
                        return Ok(Some(value));
                    }
                }
                // Pings are answered by the library on the next read.
                Some(Ok(_)) => {}
            }
        }
    }

    /// Close the connection. Errors are dropped: it is going away anyway.
    pub async fn close(&mut self) {
        let _ = self.inner.close(None).await;
    }
}

/// The part of a handshake error that is safe to show.
fn short(error: &tokio_tungstenite::tungstenite::Error) -> String {
    use tokio_tungstenite::tungstenite::Error;
    match error {
        Error::Http(response) => format!("HTTP {}", response.status()),
        other => other
            .to_string()
            .split(['\n', '('])
            .next()
            .unwrap_or("")
            .trim()
            .to_owned(),
    }
}
