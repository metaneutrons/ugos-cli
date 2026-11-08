//! `logcenter`: archiving, notifications, clearing and export of the log.

use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Result, bail};
use ugos_client::UgosClient;
use ugos_client::ops::hw::parse::unix_utc;
use ugos_client::ops::logcenter::{self, ArchiveChange, Clear, Export, NotifyChange};

use super::hw::show;
use super::share::with_force_hint;
use crate::cli::{
    ArchiveAction, ClearArgs, ExportArgs, FilemgrAction, LogCenterAction, NotifyAction, OnOff,
    OutputFormat,
};
use crate::output;
use crate::output::logcenter as rows;

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_secs()).ok())
        .unwrap_or(0)
}

fn time(text: Option<&String>) -> Result<Option<i64>> {
    Ok(text.map(|t| unix_utc(t)).transpose()?)
}

async fn archive_cmd(
    client: &UgosClient,
    action: &ArchiveAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    match action {
        ArchiveAction::Show => {
            let c = logcenter::archive_config(client).await?;
            show(w, fmt, &c, &rows::archive_rows(&c))
        }
        ArchiveAction::Set(a) => {
            let change = ArchiveChange {
                status: a.enable.map(OnOff::flag),
                data_mb: a.mb,
                entries: a.entries,
                months: a.months,
                path: a.path.clone(),
            };
            let c = logcenter::set_archive(client, &change, a.force)
                .await
                .map_err(with_force_hint)?;
            show(w, fmt, &c, &rows::archive_rows(&c))
        }
        ArchiveAction::Records { page } => {
            let p = logcenter::archive_records(client, *page).await?;
            let table: Vec<rows::ArchiveRecordRow> = p.list.iter().map(Into::into).collect();
            output::print_list(w, &table, fmt)
        }
        ArchiveAction::Files => {
            let list = logcenter::viewer_files(client).await?;
            let table: Vec<rows::ArchiveFileRow> = list.iter().map(Into::into).collect();
            output::print_list(w, &table, fmt)
        }
        ArchiveAction::Add { path } => {
            logcenter::add_viewer_file(client, path).await?;
            writeln!(w, "Added {path} to the viewer")?;
            Ok(())
        }
        ArchiveAction::Open { id } => {
            logcenter::open_viewer_file(client, *id).await?;
            writeln!(w, "Archive {id} is the active log source")?;
            Ok(())
        }
        ArchiveAction::Remove { id } => {
            logcenter::remove_viewer_file(client, *id).await?;
            writeln!(w, "Archive {id} is removed from the viewer")?;
            Ok(())
        }
    }
}

async fn notify_cmd(
    client: &UgosClient,
    action: &NotifyAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    match action {
        NotifyAction::Show => {
            let c = logcenter::notify_config(client).await?;
            show(w, fmt, &c, &rows::notify_rows(&c))
        }
        NotifyAction::Set(a) => {
            let some = |v: &Vec<String>| (!v.is_empty()).then(|| v.clone());
            let change = NotifyChange {
                status: a.enable.map(OnOff::flag),
                keyword: a.keyword.clone(),
                levels: some(&a.levels),
                modules: some(&a.modules),
                operators: some(&a.operators),
            };
            let c = logcenter::set_notify(client, &change).await?;
            show(w, fmt, &c, &rows::notify_rows(&c))
        }
        NotifyAction::Records { page } => {
            let p = logcenter::notify_records(client, *page).await?;
            let table: Vec<rows::NotifyRecordRow> = p.list.iter().map(Into::into).collect();
            output::print_list(w, &table, fmt)
        }
    }
}

async fn clear_cmd(client: &UgosClient, a: &ClearArgs, w: &mut impl Write) -> Result<()> {
    let what = match (a.all, a.older_than_days, &a.before) {
        (true, None, None) => Clear::All,
        (false, Some(days), None) if days > 0 => Clear::Before(now() - days * 86_400),
        (false, None, Some(t)) => Clear::Before(unix_utc(t)?),
        _ => bail!("give --all, --older-than-days N or --before TIME"),
    };
    logcenter::clear(client, a.transfer, what, a.force)
        .await
        .map_err(with_force_hint)?;
    writeln!(w, "The log is cleared")?;
    Ok(())
}

async fn export_cmd(client: &UgosClient, a: &ExportArgs, w: &mut impl Write) -> Result<()> {
    let what = Export {
        html: a.html,
        transfer: a.transfer,
        from: time(a.from.as_ref())?,
        to: time(a.to.as_ref())?,
    };
    let size = logcenter::export(client, &what, &a.file).await?;
    writeln!(w, "Saved {size} bytes to {}", a.file.display())?;
    Ok(())
}

/// Run a `logcenter` subcommand.
pub async fn logcenter(
    client: &UgosClient,
    action: &LogCenterAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    match action {
        LogCenterAction::Modules { transfer } => {
            let u = logcenter::users_and_modules(client, *transfer).await?;
            show(w, fmt, &u, &rows::users_modules_rows(&u))
        }
        LogCenterAction::Archive { action } => archive_cmd(client, action, fmt, w).await,
        LogCenterAction::Notify { action } => notify_cmd(client, action, fmt, w).await,
        LogCenterAction::Filemgr { action } => {
            let on = match action {
                FilemgrAction::Show => logcenter::filemgr(client).await?,
                FilemgrAction::Set { state } => {
                    logcenter::set_filemgr(client, state.flag()).await?
                }
            };
            writeln!(
                w,
                "File manager operations are logged: {}",
                if on { "yes" } else { "no" }
            )?;
            Ok(())
        }
        LogCenterAction::Clear(a) => clear_cmd(client, a, w).await,
        LogCenterAction::Export(a) => export_cmd(client, a, w).await,
    }
}
