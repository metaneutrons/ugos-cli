//! Terminals in containers, as the Docker app opens them.
//!
//! A terminal is a process (`/bin/bash`) that UGOS starts in the container
//! and keeps in a list until it is removed, whatever happens to the
//! connection. `AddTerminal` starts one and answers with its id, a WebSocket
//! (`Ws`) carries the keystrokes and the output, and `DelTerminal` ends it.
//! Opening one is running code in the container: everything here is for the
//! user to decide on.

use std::time::Duration;

use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::client::UgosClient;
use crate::error::{Result, UgosError};
use crate::ops::dockerstream::plain;
use crate::types::common::ResultWrapper;
use crate::ws::Socket;

/// UGOS refuses at most this many terminals in one container (the web UI's
/// own limit).
pub const MAX_TERMINALS: usize = 10;

/// Code UGOS answers `AddTerminal` with when the command does not exist in
/// the container.
const INVALID_COMMAND: i32 = 2100;

fn refused(message: impl Into<String>) -> UgosError {
    UgosError::Refused(message.into())
}

/// A terminal that exists in a container.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Terminal {
    /// Its id.
    #[serde(default)]
    pub uuid: String,
    /// The command it runs.
    #[serde(default)]
    pub cmd: String,
    /// `1` while someone is connected to it, `0` otherwise.
    #[serde(default)]
    pub state: i64,
    /// The user that opened it.
    #[serde(default)]
    pub uid: i64,
    /// The browser page that opened it.
    #[serde(default, rename = "pageID")]
    pub page_id: String,
}

/// The terminals open in a container.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn list(client: &UgosClient, container: &str) -> Result<Vec<Terminal>> {
    let answer: ResultWrapper<Option<Vec<Terminal>>> = client
        .get_with_params(
            "docker/container/GetTerminalList",
            &[("containerId", container), ("processCheck", "true")],
        )
        .await?;
    Ok(answer.result.unwrap_or_default())
}

/// The commands UGOS suggests for a terminal.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn suggested(client: &UgosClient) -> Result<Vec<String>> {
    let answer: ResultWrapper<Option<Vec<String>>> = client
        .get("docker/container/RecommendedTerminalCommands")
        .await?;
    Ok(answer.result.unwrap_or_default())
}

/// A page id that is unlikely to collide with the one of a browser.
fn page_id() -> Result<String> {
    let mut bytes = [0u8; 4];
    SystemRandom::new()
        .fill(&mut bytes)
        .map_err(|_| refused("no random number available"))?;
    Ok((u32::from_le_bytes(bytes) % 900_000_000 + 100_000_000).to_string())
}

/// Start a terminal that runs `command`. Returns its id and the page id it
/// has to be connected with.
///
/// # Errors
///
/// Refuses a command the container does not have, and a container that
/// already holds [`MAX_TERMINALS`] terminals.
pub async fn open(client: &UgosClient, container: &str, command: &str) -> Result<(String, String)> {
    if command.trim().is_empty() {
        return Err(refused("the command of a terminal is empty"));
    }
    if list(client, container).await?.len() >= MAX_TERMINALS {
        return Err(refused(format!(
            "the container holds {MAX_TERMINALS} terminals already: remove one first"
        )));
    }
    let page = page_id()?;
    let body = json!({"id": container, "exec": command, "pageID": page});
    let answer: Result<ResultWrapper<String>> =
        client.post("docker/container/AddTerminal", &body).await;
    match answer {
        Ok(a) if !a.result.is_empty() => Ok((a.result, page)),
        Ok(_) => Err(refused("UGOS said yes but sent no terminal id")),
        Err(UgosError::Api { code, .. }) if code == INVALID_COMMAND => Err(refused(format!(
            "'{command}' is not a command the container has (UGOS answer {INVALID_COMMAND})"
        ))),
        Err(e) => Err(e),
    }
}

/// End a terminal. One that someone is connected to is a question for the
/// user.
///
/// # Errors
///
/// Refuses an unknown terminal, and one in use unless `force`.
pub async fn remove(client: &UgosClient, container: &str, uuid: &str, force: bool) -> Result<()> {
    let all = list(client, container).await?;
    let Some(found) = all.iter().find(|t| t.uuid == uuid) else {
        return Err(refused(format!(
            "there is no terminal '{uuid}' in this container"
        )));
    };
    if found.state == 1 && !force {
        return Err(UgosError::NeedsForce(
            "someone is connected to this terminal, ending it cuts them off".into(),
        ));
    }
    let _: Value = client
        .post(
            "docker/container/DelTerminal",
            &json!({"containerId": container, "uuid": uuid}),
        )
        .await?;
    if list(client, container)
        .await?
        .iter()
        .any(|t| t.uuid == uuid)
    {
        return Err(refused("UGOS said yes but the terminal is still there"));
    }
    Ok(())
}

/// A connection to a terminal.
#[derive(Debug)]
pub struct Connection {
    socket: Socket,
}

impl Connection {
    /// Connect to a terminal that `open` started.
    ///
    /// # Errors
    ///
    /// Returns [`UgosError::Socket`] when the connection fails.
    pub async fn connect(
        client: &UgosClient,
        container: &str,
        uuid: &str,
        page: &str,
    ) -> Result<Self> {
        let socket = Socket::open(
            client,
            "docker/container/Ws",
            &[("containerId", container), ("uuid", uuid), ("pageID", page)],
        )
        .await?;
        let mut me = Self { socket };
        me.resize(200, 50).await?;
        Ok(me)
    }

    async fn frame(&mut self, code: i64, text: &str, size: Option<(u16, u16)>) -> Result<()> {
        let mut data = json!({"code": code, "msg": "", "text": text});
        if let Some((cols, rows)) = size {
            data["columns"] = json!(cols);
            data["rows"] = json!(rows);
        }
        self.socket
            .send_json(&json!({"type": "docker-terminal", "data": data}))
            .await
    }

    /// Tell the terminal how large the window is.
    ///
    /// # Errors
    ///
    /// Returns [`UgosError::Socket`] when the connection is gone.
    pub async fn resize(&mut self, cols: u16, rows: u16) -> Result<()> {
        self.frame(200, "", Some((cols, rows))).await
    }

    /// Type text into the terminal.
    ///
    /// # Errors
    ///
    /// Returns [`UgosError::Socket`] when the connection is gone.
    pub async fn input(&mut self, text: &str) -> Result<()> {
        self.frame(200, text, None).await
    }

    /// The heartbeat the web UI sends every minute.
    ///
    /// # Errors
    ///
    /// Returns [`UgosError::Socket`] when the connection is gone.
    pub async fn heartbeat(&mut self) -> Result<()> {
        self.frame(2223, "", None).await
    }

    /// The next output, or `None` when the terminal ended. Pings are
    /// answered here.
    ///
    /// # Errors
    ///
    /// Returns [`UgosError::Socket`] when the connection breaks.
    pub async fn next_output(&mut self) -> Result<Option<String>> {
        loop {
            let Some(frame) = self.socket.next_json().await? else {
                return Ok(None);
            };
            let data = &frame["data"];
            match data["code"].as_i64() {
                Some(2001) => {
                    self.frame(2001, "", None).await?;
                }
                Some(200) => {
                    if let Some(text) = data["text"].as_str().filter(|t| !t.is_empty()) {
                        return Ok(Some(text.to_owned()));
                    }
                }
                // The terminal was removed, or taken over elsewhere.
                Some(2047 | 2048) => return Ok(None),
                _ => {}
            }
        }
    }

    /// Close the connection.
    pub async fn close(&mut self) {
        self.socket.close().await;
    }
}

/// What a command printed and how it ended.
#[derive(Debug, Clone, Serialize)]
pub struct ExecResult {
    /// Output and error output together, as a terminal merges them.
    pub output: String,
    /// The exit status of the command.
    pub exit_code: i32,
}

/// The marker that ends the output, and the exit status after it.
fn marker() -> Result<String> {
    let mut bytes = [0u8; 8];
    SystemRandom::new()
        .fill(&mut bytes)
        .map_err(|_| refused("no random number available"))?;
    Ok(format!("__UGOS_{}__", hex::encode(bytes)))
}

/// Cut the output at the marker and read the exit status after it.
///
/// The marker is printed as `<marker><status>` on its own. The command line
/// that asks for it is typed with echo off, so the marker appears only once.
#[must_use]
pub fn split_marker(text: &str, marker: &str) -> Option<(String, i32)> {
    let at = text.find(marker)?;
    let rest = &text[at + marker.len()..];
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    // Without the end of the number the status could be cut short.
    if digits.is_empty() || rest.len() == digits.len() {
        return None;
    }
    let code = digits.parse().ok()?;
    Some((text[..at].to_owned(), code))
}

/// How long the shell may stay silent before its prompt counts as there.
const PROMPT_PAUSE: Duration = Duration::from_millis(600);

async fn until_quiet(conn: &mut Connection, quiet: Duration) -> Result<String> {
    let mut text = String::new();
    loop {
        match tokio::time::timeout(quiet, conn.next_output()).await {
            Ok(Ok(Some(piece))) => text.push_str(&piece),
            Ok(Ok(None)) | Err(_) => return Ok(text),
            Ok(Err(e)) => return Err(e),
        }
    }
}

async fn run_in(
    conn: &mut Connection,
    command: &str,
    limit: Duration,
    max_bytes: usize,
) -> Result<ExecResult> {
    let end = marker()?;
    // The shell's own prompt and its banner come first.
    let _ = until_quiet(conn, PROMPT_PAUSE).await?;
    // Echo off, so that what comes back is the output and not the typing.
    conn.input("stty -echo 2>/dev/null\r").await?;
    let _ = until_quiet(conn, PROMPT_PAUSE).await?;
    // One line, so that the prompt of the shell comes after the marker and
    // not between the command and the marker.
    conn.input(&format!("{command}; echo \"{end}$?\"\r"))
        .await?;
    let mut text = String::new();
    let mut beat = tokio::time::interval(Duration::from_secs(50));
    let _ = beat.tick().await;
    let deadline = tokio::time::Instant::now() + limit;
    loop {
        tokio::select! {
            piece = conn.next_output() => match piece? {
                Some(piece) => {
                    text.push_str(&piece);
                    let clean = plain(&text);
                    if let Some((out, code)) = split_marker(&clean, &end) {
                        let out = out.strip_prefix('\r').unwrap_or(&out);
                        let out = out.strip_prefix('\n').unwrap_or(out).to_owned();
                        return Ok(ExecResult { output: out, exit_code: code });
                    }
                    if text.len() > max_bytes {
                        return Err(refused("the output is larger than the limit"));
                    }
                }
                None => return Err(refused("the terminal ended before the command did")),
            },
            _ = beat.tick() => conn.heartbeat().await?,
            () = tokio::time::sleep_until(deadline) => {
                return Err(refused(format!(
                    "the command did not finish in {} seconds",
                    limit.as_secs()
                )));
            }
        }
    }
}

/// Run one command in a container and return what it printed.
///
/// A terminal is opened, used and removed again, also when the command fails
/// or runs out of time. The command is read by the shell of the terminal, so
/// quoting is the caller's.
///
/// # Errors
///
/// Refuses what [`open`] refuses, a command that does not end within `limit`,
/// and more output than `max_bytes`.
pub async fn run(
    client: &UgosClient,
    container: &str,
    shell: &str,
    command: &str,
    limit: Duration,
    max_bytes: usize,
) -> Result<ExecResult> {
    let (uuid, page) = open(client, container, shell).await?;
    let outcome = async {
        let mut conn = Connection::connect(client, container, &uuid, &page).await?;
        let result = run_in(&mut conn, command, limit, max_bytes).await;
        conn.close().await;
        result
    }
    .await;
    // Remove it in any case. A failure here must not hide the outcome.
    let _ = remove(client, container, &uuid, true).await;
    outcome
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::split_marker;

    #[test]
    fn the_output_ends_at_the_marker_and_the_status_follows_it() {
        let text = "total 4\nfile\n__UGOS_ab__0\nroot@c:/# ";
        assert_eq!(
            split_marker(text, "__UGOS_ab__"),
            Some(("total 4\nfile\n".to_owned(), 0))
        );
        let text = "boom\n__UGOS_ab__127\nroot@c:/# ";
        assert_eq!(split_marker(text, "__UGOS_ab__").unwrap().1, 127);
    }

    #[test]
    fn a_status_that_may_not_be_complete_is_not_taken() {
        assert_eq!(split_marker("out\n__UGOS_ab__", "__UGOS_ab__"), None);
        assert_eq!(split_marker("out\n__UGOS_ab__12", "__UGOS_ab__"), None);
        assert_eq!(split_marker("no marker", "__UGOS_ab__"), None);
    }
}
