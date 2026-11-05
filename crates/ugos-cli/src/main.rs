//! UGOS CLI — command-line interface for UGREEN NAS management.

mod cli;
mod commands;

mod output;
mod session;
mod skill;

use std::io::{BufWriter, Write};

use anyhow::{Context, Result, bail};
use clap::parser::ValueSource;
use clap::{CommandFactory as _, FromArgMatches as _};
use ugos_client::tls::known_hosts;
use ugos_client::{Credentials, Session, TlsPolicy, UgosClient, tls};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let matches = cli::Cli::command().get_matches();
    let cli = cli::Cli::from_arg_matches(&matches).unwrap_or_else(|e| e.exit());

    if run_offline(&cli)? {
        return Ok(());
    }

    let host = cli
        .host
        .as_deref()
        .context("--host or UGOS_HOST required")?;
    let user = cli
        .user
        .clone()
        .or_else(|| env_var("UGOS_USERNAME"))
        .context("--user, UGOS_USER or UGOS_USERNAME required")?;
    check_password_sources(&matches, &cli)?;
    if matches.value_source("password") == Some(ValueSource::CommandLine) {
        notice(PASSWORD_FLAG_WARNING);
    }
    let password = if cli.password_stdin {
        if cli.reads_stdin_twice() {
            bail!("--password-stdin and --registry-password-stdin both read stdin");
        }
        password_from_reader(&mut std::io::stdin().lock())?
    } else {
        cli.password
            .clone()
            .context("--password, --password-stdin or UGOS_PASSWORD required")?
    };

    let creds = Credentials {
        username: user.clone(),
        password,
    };

    let tls = resolve_tls(host, cli.port, cli.tls_insecure, cli.tls_trust_new).await?;
    let client = build_client(host, cli.port, &creds, cli.no_cache, &tls).await?;

    let mut stdout = BufWriter::new(std::io::stdout().lock());
    let result = commands::run(&client, &cli.command, cli.output, &mut stdout).await;
    let flush_result = stdout.flush();

    // Check for BrokenPipe (piped to head/less/etc) — exit silently.
    if is_broken_pipe(&result) || is_broken_pipe_io(&flush_result) {
        std::process::exit(0);
    }
    flush_result.context("flushing stdout")?;

    if let Err(e) = result {
        bail!("{e:#}");
    }

    // After the command, so it adds nothing to what the user waits for, and
    // never able to fail it.
    skill::update_quietly();
    skill::hint_once(matches!(cli.output, cli::OutputFormat::Json));

    // Save session after command (re-auth may have refreshed it).
    if !cli.no_cache {
        let sess = client.session().await;
        let cached = session::CachedSession {
            host: host.to_owned(),
            port: cli.port,
            user: user.clone(),
            token: sess.token,
            public_key: sess.public_key,
            created_at: session::unix_now(),
        };
        if let Err(e) = session::save(&cached) {
            tracing::warn!("failed to save session cache: {e}");
        }
    }

    Ok(())
}

/// Shown when the password arrives as a command-line argument.
const PASSWORD_FLAG_WARNING: &str = "warning: --password is visible in the process list and the \
shell history; set UGOS_PASSWORD or use --password-stdin instead";

/// `--password` and `--password-stdin` on one command line is a mistake worth
/// naming. A password that merely comes from `UGOS_PASSWORD` is not: stdin
/// then wins, so a shell with the variable set can still pipe another one in.
fn check_password_sources(matches: &clap::ArgMatches, cli: &cli::Cli) -> Result<()> {
    if cli.password_stdin && matches.value_source("password") == Some(ValueSource::CommandLine) {
        bail!("--password and --password-stdin cannot be combined");
    }
    Ok(())
}

/// First line of `reader`, without its line ending. An empty line is refused
/// rather than sent as a password.
fn password_from_reader(reader: &mut impl std::io::BufRead) -> Result<String> {
    let mut line = String::new();
    let _ = reader
        .read_line(&mut line)
        .context("reading the password from stdin")?;
    let password = line.trim_end_matches(['\r', '\n']);
    if password.is_empty() {
        bail!("--password-stdin read an empty line");
    }
    Ok(password.to_owned())
}

/// Read an environment variable, treating an empty value as unset.
fn env_var(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.is_empty())
}

/// Handle commands that need no NAS connection.
///
/// Returns `true` when the command was handled and the process should exit.
fn run_offline(cli: &cli::Cli) -> Result<bool> {
    if let cli::Resource::Skill(args) = &cli.command {
        let request = if args.install {
            skill::Request::Install { ask: !args.yes }
        } else if args.decline {
            skill::Request::Decline
        } else if args.remove {
            skill::Request::Remove
        } else {
            skill::Request::Status
        };
        let mut stdout = BufWriter::new(std::io::stdout().lock());
        skill::control(request, &mut stdout)?;
        stdout.flush().context("flushing stdout")?;
        return Ok(true);
    }

    let cli::Resource::Vm {
        action: cli::VmAction::Create(args),
    } = &cli.command
    else {
        return Ok(false);
    };
    if !args.dry_run {
        return Ok(false);
    }

    let spec = commands::vmspec::build(args)?;
    let mut stdout = BufWriter::new(std::io::stdout().lock());
    output::print_json(&mut stdout, &spec)?;
    stdout.flush().context("flushing stdout")?;
    Ok(true)
}

/// Write a notice to stderr, so it stays out of piped output.
///
/// A failed write is dropped: losing a notice must not abort the command
/// the user actually asked for.
fn notice(message: &str) {
    let mut err = std::io::stderr();
    let _ = writeln!(err, "{message}");
}

/// Settle on a TLS policy for this host, pinning on first contact.
///
/// Trust on first use: the first connection to an unknown host records the
/// certificate it presents, and every later one must match it. That first
/// contact is only as trustworthy as the network it happens on, so the
/// fingerprint is shown for the user to compare against the device.
async fn resolve_tls(host: &str, port: u16, insecure: bool, trust_new: bool) -> Result<TlsPolicy> {
    if insecure {
        notice("warning: certificate checking disabled (--tls-insecure)");
        return Ok(TlsPolicy::Insecure);
    }

    if trust_new {
        let seen = tls::probe_fingerprint(host, port).await?;
        known_hosts::put(host, port, &seen)?;
        notice(&format!(
            "recorded new certificate for {host}:{port}\n  {seen}"
        ));
        return Ok(TlsPolicy::Pinned(seen));
    }

    if let Some(fp) = known_hosts::get(host, port)? {
        return Ok(TlsPolicy::Pinned(fp));
    }

    let seen = tls::probe_fingerprint(host, port).await?;
    known_hosts::put(host, port, &seen)?;
    notice(&format!(
        "first contact with {host}:{port}, trusting its certificate\n  {seen}\n\
         Compare it against the NAS before using this connection for anything sensitive."
    ));
    Ok(TlsPolicy::Pinned(seen))
}

/// Build a [`UgosClient`], using the session cache when possible.
async fn build_client(
    host: &str,
    port: u16,
    creds: &Credentials,
    no_cache: bool,
    tls: &TlsPolicy,
) -> Result<UgosClient> {
    // Try cached session first.
    if !no_cache && let Some(cached) = session::load(host, port, &creds.username) {
        tracing::debug!("using cached session");
        let session = Session {
            token: cached.token,
            public_key: cached.public_key,
        };
        return Ok(UgosClient::from_session(
            host,
            port,
            creds.clone(),
            session,
            tls,
        )?);
    }

    // Fresh login.
    tracing::debug!("performing fresh login");
    let client = UgosClient::connect(host, port, creds.clone(), tls).await?;

    // Cache the new session.
    if !no_cache {
        let sess = client.session().await;
        let cached = session::CachedSession {
            host: host.to_owned(),
            port,
            user: creds.username.clone(),
            token: sess.token,
            public_key: sess.public_key,
            created_at: session::unix_now(),
        };
        if let Err(e) = session::save(&cached) {
            tracing::warn!("failed to save session cache: {e}");
        }
    }

    Ok(client)
}

/// Check if an `anyhow::Error` wraps a `BrokenPipe`.
fn is_broken_pipe(result: &Result<()>) -> bool {
    match result {
        Err(e) => e
            .downcast_ref::<std::io::Error>()
            .is_some_and(|io| io.kind() == std::io::ErrorKind::BrokenPipe),
        Ok(()) => false,
    }
}

/// Check if an `io::Result` is a `BrokenPipe`.
fn is_broken_pipe_io(result: &std::io::Result<()>) -> bool {
    match result {
        Err(e) => e.kind() == std::io::ErrorKind::BrokenPipe,
        Ok(()) => false,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::{check_password_sources, password_from_reader};
    use crate::cli::Cli;
    use clap::{CommandFactory as _, FromArgMatches as _};

    fn sources_ok(args: &[&str]) -> bool {
        let matches = Cli::command().try_get_matches_from(args).unwrap();
        let cli = Cli::from_arg_matches(&matches).unwrap();
        check_password_sources(&matches, &cli).is_ok()
    }

    #[test]
    fn the_flag_and_stdin_together_are_refused() {
        assert!(!sources_ok(&[
            "ugos-cli",
            "--password",
            "x",
            "--password-stdin",
            "vm",
            "list"
        ]));
    }

    #[test]
    fn stdin_alone_and_the_flag_alone_are_fine() {
        assert!(sources_ok(&["ugos-cli", "--password-stdin", "vm", "list"]));
        assert!(sources_ok(&["ugos-cli", "--password", "x", "vm", "list"]));
    }

    #[test]
    fn reads_the_first_line_and_drops_the_line_ending() {
        let mut input: &[u8] = b"hunter2\nsecond\n";
        assert_eq!(password_from_reader(&mut input).unwrap(), "hunter2");
        let mut input: &[u8] = b"hunter2\r\n";
        assert_eq!(password_from_reader(&mut input).unwrap(), "hunter2");
    }

    #[test]
    fn keeps_spaces_inside_the_password() {
        let mut input: &[u8] = b" a b \n";
        assert_eq!(password_from_reader(&mut input).unwrap(), " a b ");
    }

    #[test]
    fn refuses_an_empty_line_and_an_empty_stream() {
        let mut line: &[u8] = b"\n";
        assert!(password_from_reader(&mut line).is_err());
        let mut nothing: &[u8] = b"";
        assert!(password_from_reader(&mut nothing).is_err());
    }
}
