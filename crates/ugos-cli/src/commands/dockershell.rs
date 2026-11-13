//! `docker terminal-ls`, `terminal-rm`, `exec` and `shell`: terminals in
//! containers.

use std::io::{IsTerminal, Write};
use std::time::Duration;

use anyhow::{Result, bail};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ugos_client::UgosClient;
use ugos_client::ops::{dockerterm, dockerview};

use super::share::with_force_hint;
use crate::cli::{DockerAction, OutputFormat};
use crate::output;
use crate::output::docker::TerminalRow;

/// How much output `exec` takes before it gives up.
const EXEC_MAX_BYTES: usize = 16 * 1024 * 1024;

/// The shell to start: the one asked for, else what UGOS suggests.
async fn shell_of(client: &UgosClient, asked: Option<&String>) -> Result<String> {
    if let Some(shell) = asked {
        return Ok(shell.clone());
    }
    Ok(dockerterm::suggested(client)
        .await?
        .into_iter()
        .next()
        .unwrap_or_else(|| "/bin/bash".to_owned()))
}

/// Run one of the terminal commands.
pub async fn run(
    client: &UgosClient,
    action: &DockerAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    match action {
        DockerAction::TerminalLs { id } => {
            let full = dockerview::resolve(client, id).await?;
            let rows: Vec<TerminalRow> = dockerterm::list(client, &full)
                .await?
                .iter()
                .map(Into::into)
                .collect();
            output::print_list(w, &rows, fmt)
        }
        DockerAction::TerminalRm { id, uuid, force } => {
            let full = dockerview::resolve(client, id).await?;
            dockerterm::remove(client, &full, uuid, *force)
                .await
                .map_err(with_force_hint)?;
            output::print_success(w, &format!("Ended terminal {uuid}"), fmt)
        }
        DockerAction::Exec { .. } => exec(client, action, fmt, w).await,
        DockerAction::Shell { id, shell } => {
            let full = dockerview::resolve(client, id).await?;
            let shell = shell_of(client, shell.as_ref()).await?;
            interactive(client, &full, &shell, w).await
        }
        _ => bail!("not a terminal command"),
    }
}

async fn exec(
    client: &UgosClient,
    action: &DockerAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    let DockerAction::Exec {
        id,
        shell,
        timeout,
        command,
    } = action
    else {
        bail!("not an exec command");
    };
    let full = dockerview::resolve(client, id).await?;
    let shell = shell_of(client, shell.as_ref()).await?;
    let result = dockerterm::run(
        client,
        &full,
        &shell,
        &command.join(" "),
        Duration::from_secs(*timeout),
        EXEC_MAX_BYTES,
    )
    .await?;
    exec_out(w, &result, fmt)
}

fn exec_out(w: &mut impl Write, result: &dockerterm::ExecResult, fmt: OutputFormat) -> Result<()> {
    if fmt == OutputFormat::Json {
        return output::print_json(w, result);
    }
    w.write_all(result.output.as_bytes())?;
    if result.exit_code != 0 {
        bail!("the command exited with status {}", result.exit_code);
    }
    Ok(())
}

/// What a key sends to the terminal, as a program in it expects to read it.
fn key_text(key: &KeyEvent) -> Option<String> {
    if key.kind == KeyEventKind::Release {
        return None;
    }
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(KeyModifiers::ALT);
    let text = match key.code {
        KeyCode::Char(c) if ctrl && c.is_ascii_alphabetic() => {
            char::from(c.to_ascii_lowercase() as u8 - b'a' + 1).to_string()
        }
        KeyCode::Char(c) if alt => format!("\u{1b}{c}"),
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Enter => "\r".to_owned(),
        KeyCode::Tab => "\t".to_owned(),
        KeyCode::BackTab => "\u{1b}[Z".to_owned(),
        KeyCode::Backspace => "\u{7f}".to_owned(),
        KeyCode::Esc => "\u{1b}".to_owned(),
        KeyCode::Left => "\u{1b}[D".to_owned(),
        KeyCode::Right => "\u{1b}[C".to_owned(),
        KeyCode::Up => "\u{1b}[A".to_owned(),
        KeyCode::Down => "\u{1b}[B".to_owned(),
        KeyCode::Home => "\u{1b}[H".to_owned(),
        KeyCode::End => "\u{1b}[F".to_owned(),
        KeyCode::PageUp => "\u{1b}[5~".to_owned(),
        KeyCode::PageDown => "\u{1b}[6~".to_owned(),
        KeyCode::Delete => "\u{1b}[3~".to_owned(),
        KeyCode::Insert => "\u{1b}[2~".to_owned(),
        _ => return None,
    };
    Some(text)
}

/// Puts the terminal into raw mode and puts it back, also on an error.
struct RawMode;

impl RawMode {
    fn on() -> Result<Self> {
        crossterm::terminal::enable_raw_mode()?;
        Ok(Self)
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        let _ = crossterm::terminal::disable_raw_mode();
    }
}

async fn interactive(
    client: &UgosClient,
    container: &str,
    shell: &str,
    w: &mut impl Write,
) -> Result<()> {
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        bail!("docker shell needs a terminal on both ends, use `docker exec` to run a command");
    }
    let (uuid, page) = dockerterm::open(client, container, shell).await?;
    let outcome = session(client, container, &uuid, &page, w).await;
    // Always remove what this command started.
    let _ = dockerterm::remove(client, container, &uuid, true).await;
    outcome
}

/// Key and window events of the terminal, read on a thread of their own
/// because reading them blocks.
fn read_events() -> tokio::sync::mpsc::UnboundedReceiver<Event> {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let _reader = std::thread::spawn(move || {
        while let Ok(ev) = event::read() {
            if tx.send(ev).is_err() {
                break;
            }
        }
    });
    rx
}

/// Pass a key or a new window size on to the terminal.
async fn send_event(conn: &mut dockerterm::Connection, event: &Event) -> Result<()> {
    match event {
        Event::Key(key) => {
            if let Some(text) = key_text(key) {
                conn.input(&text).await?;
            }
        }
        Event::Resize(cols, rows) => conn.resize(*cols, *rows).await?,
        _ => {}
    }
    Ok(())
}

async fn session(
    client: &UgosClient,
    container: &str,
    uuid: &str,
    page: &str,
    w: &mut impl Write,
) -> Result<()> {
    let mut conn = dockerterm::Connection::connect(client, container, uuid, page).await?;
    let (cols, rows) = crossterm::terminal::size().unwrap_or((120, 40));
    conn.resize(cols, rows).await?;
    let mut keys = read_events();
    let _raw = RawMode::on()?;
    let mut beat = tokio::time::interval(Duration::from_secs(50));
    let _ = beat.tick().await;
    // UGOS does not close the connection when the shell ends. The web UI
    // finds out from the list of terminals, and so does this.
    let mut watch = tokio::time::interval(Duration::from_secs(2));
    let _ = watch.tick().await;
    loop {
        tokio::select! {
            ev = keys.recv() => match ev {
                Some(event) => send_event(&mut conn, &event).await?,
                None => break,
            },
            out = conn.next_output() => match out? {
                Some(text) => {
                    w.write_all(text.as_bytes())?;
                    w.flush()?;
                }
                None => break,
            },
            _ = beat.tick() => conn.heartbeat().await?,
            _ = watch.tick() => {
                if !dockerterm::list(client, container).await?.iter().any(|t| t.uuid == uuid) {
                    break;
                }
            }
        }
    }
    conn.close().await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    #[test]
    fn keys_become_what_a_terminal_reads() {
        let none = KeyModifiers::NONE;
        assert_eq!(
            key_text(&key(KeyCode::Char('a'), none)).as_deref(),
            Some("a")
        );
        assert_eq!(key_text(&key(KeyCode::Enter, none)).as_deref(), Some("\r"));
        assert_eq!(
            key_text(&key(KeyCode::Up, none)).as_deref(),
            Some("\u{1b}[A")
        );
        assert_eq!(
            key_text(&key(KeyCode::Backspace, none)).as_deref(),
            Some("\u{7f}")
        );
    }

    #[test]
    fn control_and_alt_change_the_letter() {
        assert_eq!(
            key_text(&key(KeyCode::Char('c'), KeyModifiers::CONTROL)).as_deref(),
            Some("\u{3}")
        );
        assert_eq!(
            key_text(&key(KeyCode::Char('d'), KeyModifiers::CONTROL)).as_deref(),
            Some("\u{4}")
        );
        assert_eq!(
            key_text(&key(KeyCode::Char('f'), KeyModifiers::ALT)).as_deref(),
            Some("\u{1b}f")
        );
    }

    #[test]
    fn a_released_key_and_an_unknown_key_send_nothing() {
        let mut release = key(KeyCode::Char('a'), KeyModifiers::NONE);
        release.kind = KeyEventKind::Release;
        assert_eq!(key_text(&release), None);
        assert_eq!(key_text(&key(KeyCode::CapsLock, KeyModifiers::NONE)), None);
    }
}
