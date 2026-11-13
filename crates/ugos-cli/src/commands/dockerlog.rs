//! `docker logs` and `docker oplog`: what a container wrote, and what was
//! done in the Docker app.

use std::io::Write;

use anyhow::{Result, bail};
use ugos_client::UgosClient;
use ugos_client::ops::dockerstream::{self, BACKLOG_MAX, BACKLOG_PAUSE, LogStream};
use ugos_client::ops::dockerview::{self, OpLogQuery};

use crate::cli::{DockerAction, OutputFormat};
use crate::output;
use crate::output::docker::OpLogRow;

/// Print a container log. The note that UGOS cut it off goes to stderr, so a
/// pipe gets only the log.
pub fn container_log_out(
    w: &mut impl Write,
    text: &str,
    cut: bool,
    fmt: OutputFormat,
) -> Result<()> {
    match fmt {
        OutputFormat::Table => w.write_all(text.as_bytes())?,
        OutputFormat::Json => {
            output::print_json(w, &serde_json::json!({"log": text, "cut_by_ugos": cut}))?;
        }
    }
    if cut && fmt == OutputFormat::Table {
        let _ = writeln!(
            std::io::stderr(),
            "note: UGOS stops the log at its limit, older lines are not shown"
        );
    }
    Ok(())
}

/// Run `docker oplog`.
pub async fn docker_oplog(
    client: &UgosClient,
    action: &DockerAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    let DockerAction::Oplog {
        user,
        failed,
        contains,
        oldest_first,
        limit,
        page,
        users,
        export,
    } = action
    else {
        bail!("not an oplog command");
    };
    if *users {
        let names = dockerview::oplog_users(client).await?;
        return output::print_json(w, &names);
    }
    if *export {
        let text = dockerview::oplog_export(client, *failed).await?;
        return Ok(w.write_all(text.as_bytes())?);
    }
    let query = OpLogQuery {
        operator: user.clone(),
        failures_only: *failed,
        contains: contains.clone(),
        oldest_first: *oldest_first,
    };
    let (logs, total) = dockerview::oplog(client, &query, *page, *limit).await?;
    let rows: Vec<OpLogRow> = logs.iter().map(Into::into).collect();
    output::print_list(w, &rows, fmt)?;
    if fmt == OutputFormat::Table && u64::from(*limit) * u64::from(*page) < total {
        writeln!(w, "{total} entries in all, see --page")?;
    }
    Ok(())
}

/// `docker logs --follow` and `docker project-logs`: read the stream the web
/// UI reads, print the last lines, and with `--follow` keep going until
/// Ctrl-C.
pub async fn stream_logs(
    client: &UgosClient,
    action: &DockerAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    let (mut stream, lines, follow) = match action {
        DockerAction::Logs {
            id, lines, follow, ..
        } => (LogStream::container(client, id).await?, *lines, *follow),
        DockerAction::ProjectLogs {
            name,
            lines,
            follow,
        } => (LogStream::project(client, name).await?, *lines, *follow),
        _ => bail!("not a log command"),
    };
    if follow && fmt == OutputFormat::Json {
        bail!("--follow prints text, it cannot be combined with -o json");
    }
    let backlog = stream.backlog(BACKLOG_PAUSE, BACKLOG_MAX).await?;
    let text = dockerview::tail(&dockerstream::plain(&backlog), lines);
    if fmt == OutputFormat::Json {
        stream.close().await;
        return output::print_json(w, &serde_json::json!({ "log": text }));
    }
    w.write_all(text.as_bytes())?;
    w.flush()?;
    if !follow {
        stream.close().await;
        return Ok(());
    }
    loop {
        tokio::select! {
            piece = stream.next_text() => match piece? {
                Some(piece) => {
                    w.write_all(dockerstream::plain(&piece).as_bytes())?;
                    w.flush()?;
                }
                None => return Ok(()),
            },
            _ = tokio::signal::ctrl_c() => {
                stream.close().await;
                return Ok(());
            }
        }
    }
}
