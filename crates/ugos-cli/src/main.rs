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

    let host = cli
        .host
        .as_deref()
        .context("--host or UGOS_HOST required")?;
    let user = cli
        .user
        .as_deref()
        .context("--user or UGOS_USER required")?;
    let password = cli
        .password
        .as_deref()
        .context("--password or UGOS_PASSWORD required")?;

    let creds = Credentials {
        username: user.to_owned(),
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
            user: user.to_owned(),
            token: sess.token,
            created_at: session::unix_now(),
        };
        if let Err(e) = session::save(&cached) {
            tracing::warn!("failed to save session cache: {e}");
        }
    }

    Ok(())
}

/// Build a [`UgosClient`], using the session cache when possible.
async fn build_client(
    host: &str,
    port: u16,
    creds: &Credentials,
    no_cache: bool,
) -> Result<UgosClient> {
    // Try cached session first.
    if !no_cache {
        if let Some(cached) = session::load(host, port, &creds.username) {
            tracing::debug!("using cached session");
            let session = Session {
                token: cached.token,
            };
            return Ok(UgosClient::from_session(
                host,
                port,
                creds.clone(),
                session,
            )?);
        }
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
