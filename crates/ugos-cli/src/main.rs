//! UGOS CLI — command-line interface for UGREEN NAS management.

mod cli;
mod commands;
mod output;
mod session;

use std::io::{BufWriter, Write};

use anyhow::{Context, Result, bail};
use clap::Parser;
use ugos_client::{Credentials, Session, UgosClient};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = cli::Cli::parse();

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
    let password = cli
        .password
        .as_deref()
        .context("--password or UGOS_PASSWORD required")?;

    let creds = Credentials {
        username: user.clone(),
        password: password.to_owned(),
    };

    let client = build_client(host, cli.port, &creds, cli.no_cache).await?;

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

/// Read an environment variable, treating an empty value as unset.
fn env_var(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.is_empty())
}

/// Handle commands that need no NAS connection.
///
/// Returns `true` when the command was handled and the process should exit.
fn run_offline(cli: &cli::Cli) -> Result<bool> {
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

/// Build a [`UgosClient`], using the session cache when possible.
async fn build_client(
    host: &str,
    port: u16,
    creds: &Credentials,
    no_cache: bool,
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
        )?);
    }

    // Fresh login.
    tracing::debug!("performing fresh login");
    let client = UgosClient::connect(host, port, creds.clone()).await?;

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
