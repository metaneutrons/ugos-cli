//! The logs of a container and of a compose project, as the Docker app reads
//! them: over a WebSocket that first sends what is already there and then
//! what is written from now on.
//!
//! The frames are JSON, `{"type": ..., "data": {"code", "text", ...}}`. Code
//! `200` carries text, `2001` is a ping the client has to answer, `2002` is
//! ignored. A frame can end in the middle of a line, so a caller that cares
//! about lines has to join the text first.

use std::time::Duration;

use serde_json::{Value, json};

use crate::client::UgosClient;
use crate::error::{Result, UgosError};
use crate::ws::Socket;

/// What the stream is of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Container,
    Project,
}

/// An open log stream.
#[derive(Debug)]
pub struct LogStream {
    socket: Socket,
    kind: Kind,
}

/// The answer to the NAS's ping, in the form the web UI sends it.
fn pong(kind: Kind) -> Value {
    match kind {
        Kind::Container => {
            json!({"type": "docker-log", "data": {"code": 2001, "msg": "", "text": ""}})
        }
        Kind::Project => {
            json!({"type": "compose-log", "data": {"code": 2001, "msg": "pong", "text": ""}})
        }
    }
}

impl LogStream {
    /// Open the log of a container.
    ///
    /// # Errors
    ///
    /// Returns [`UgosError::Socket`] when the stream cannot be opened.
    pub async fn container(client: &UgosClient, id: &str) -> Result<Self> {
        let socket = Socket::open(
            client,
            "docker/container/ShowContainerLogsFollow",
            &[("containerId", id), ("rawData", "true")],
        )
        .await?;
        Ok(Self {
            socket,
            kind: Kind::Container,
        })
    }

    /// Open the log of a compose project: every service, each line with the
    /// service's name in front.
    ///
    /// # Errors
    ///
    /// Returns [`UgosError::Socket`] when the stream cannot be opened.
    pub async fn project(client: &UgosClient, name: &str) -> Result<Self> {
        let mut socket = Socket::open(
            client,
            "docker/compose/GetProjectLogs",
            &[("projectName", name)],
        )
        .await?;
        // The NAS sends nothing until the client has said hello.
        socket
            .send_json(&json!({"type": "docker-project-log",
                "data": {"code": 200, "msg": "", "text": "", "columns": 200, "rows": 50}}))
            .await?;
        Ok(Self {
            socket,
            kind: Kind::Project,
        })
    }

    /// The next piece of log text, or `None` when the NAS ended the stream.
    ///
    /// # Errors
    ///
    /// Returns [`UgosError::Socket`] when the connection breaks, or when the
    /// NAS answers with an error code.
    pub async fn next_text(&mut self) -> Result<Option<String>> {
        loop {
            let Some(frame) = self.socket.next_json().await? else {
                return Ok(None);
            };
            let data = &frame["data"];
            match data["code"].as_i64() {
                Some(2001) => self.socket.send_json(&pong(self.kind)).await?,
                Some(2002) => {}
                Some(200) => {
                    if let Some(text) = data["text"].as_str().filter(|t| !t.is_empty()) {
                        return Ok(Some(text.to_owned()));
                    }
                }
                other => {
                    let msg = data["msg"].as_str().unwrap_or("");
                    return Err(UgosError::Socket(format!(
                        "the NAS answered with code {} {msg}",
                        other.map_or_else(|| "?".to_owned(), |c| c.to_string())
                    )));
                }
            }
        }
    }

    /// Read what is there already: everything up to the first pause of
    /// `quiet`, or `max_bytes`. The NAS sends the backlog in a burst, so a
    /// pause means it is done.
    ///
    /// # Errors
    ///
    /// Returns [`UgosError::Socket`] when the connection breaks.
    pub async fn backlog(&mut self, quiet: Duration, max_bytes: usize) -> Result<String> {
        let mut text = String::new();
        while text.len() < max_bytes {
            match tokio::time::timeout(quiet, self.next_text()).await {
                Ok(Ok(Some(piece))) => text.push_str(&piece),
                Ok(Ok(None)) | Err(_) => break,
                Ok(Err(e)) => return Err(e),
            }
        }
        Ok(text)
    }

    /// Close the stream.
    pub async fn close(&mut self) {
        self.socket.close().await;
    }
}

/// Take out the colour and cursor codes of a terminal, and turn `\r\n` into
/// `\n`. The project log is written for a terminal and full of them.
#[must_use]
pub fn plain(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\u{1b}' => {
                if chars.peek() == Some(&'[') {
                    let _ = chars.next();
                    for d in chars.by_ref() {
                        if d.is_ascii_alphabetic() {
                            break;
                        }
                    }
                }
            }
            '\r' if chars.peek() == Some(&'\n') => {}
            other => out.push(other),
        }
    }
    out
}

/// The last `lines` lines of the log of a container, read from the stream.
///
/// # Errors
///
/// Returns [`UgosError::Socket`] when the stream cannot be read.
pub async fn container_tail(client: &UgosClient, id: &str, lines: usize) -> Result<String> {
    let mut stream = LogStream::container(client, id).await?;
    let text = stream.backlog(BACKLOG_PAUSE, BACKLOG_MAX).await;
    stream.close().await;
    Ok(super::dockerview::tail(&plain(&text?), lines))
}

/// The last `lines` lines of the log of a compose project.
///
/// # Errors
///
/// Returns [`UgosError::Socket`] when the stream cannot be read.
pub async fn project_tail(client: &UgosClient, name: &str, lines: usize) -> Result<String> {
    let mut stream = LogStream::project(client, name).await?;
    let text = stream.backlog(BACKLOG_PAUSE, BACKLOG_MAX).await;
    stream.close().await;
    Ok(super::dockerview::tail(&plain(&text?), lines))
}

/// A pause this long means the backlog is complete.
pub const BACKLOG_PAUSE: Duration = Duration::from_millis(700);

/// How much backlog to take at most.
pub const BACKLOG_MAX: usize = 4 * 1024 * 1024;

#[cfg(test)]
mod tests {
    use super::plain;

    #[test]
    fn colours_and_carriage_returns_are_taken_out() {
        assert_eq!(
            plain("\u{1b}[36mworker-1  | \u{1b}[0mready\r\n"),
            "worker-1  | ready\n"
        );
        assert_eq!(plain("a\r\nb"), "a\nb");
        assert_eq!(plain("plain"), "plain");
    }

    #[test]
    fn a_lone_return_stays() {
        assert_eq!(plain("50%\r60%"), "50%\r60%");
    }
}
