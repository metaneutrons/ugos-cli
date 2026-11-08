//! `version`: the File Version Explorer.

use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use ugos_client::UgosClient;
use ugos_client::ops::hw::parse::unix_utc;
use ugos_client::ops::versions::{self, Clash, FolderChange, FolderKind};

use super::hw::show;
use super::share::with_force_hint;
use crate::cli::{ClashArg, FolderKindArg, OnOff, OutputFormat, VersionAction};
use crate::output;
use crate::output::versions as rows;

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_secs()).ok())
        .unwrap_or(0)
}

const fn kind(k: FolderKindArg) -> FolderKind {
    match k {
        FolderKindArg::Share => FolderKind::Share,
        FolderKindArg::User => FolderKind::User,
        FolderKindArg::Domain => FolderKind::Domain,
    }
}

const fn clash(c: ClashArg) -> Clash {
    match c {
        ClashArg::Skip => Clash::Skip,
        ClashArg::Overwrite => Clash::Overwrite,
        ClashArg::KeepBoth => Clash::KeepBoth,
    }
}

async fn browse(
    client: &UgosClient,
    action: &VersionAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    match action {
        VersionAction::Ls { path, at, deleted } => {
            let when = at
                .at
                .as_deref()
                .map(unix_utc)
                .transpose()?
                .unwrap_or_else(now);
            let list = versions::list(client, path, when, *deleted).await?;
            let table: Vec<rows::ItemRow> = list.iter().map(Into::into).collect();
            output::print_list(w, &table, fmt)
        }
        VersionAction::History { path } => {
            let list = versions::history(client, path, now()).await?;
            let table: Vec<rows::VersionRow> = list.iter().map(Into::into).collect();
            output::print_list(w, &table, fmt)
        }
        VersionAction::Calendar { year, month } => {
            let marks = versions::calendar(client, *year, *month, now()).await?;
            show(w, fmt, &marks, &rows::calendar_rows(&marks.day))
        }
        _ => Ok(()),
    }
}

async fn restore_cmd(
    client: &UgosClient,
    action: &VersionAction,
    w: &mut impl Write,
) -> Result<()> {
    match action {
        VersionAction::Copy {
            path,
            version: number,
            to,
            clash: c,
            force,
        } => {
            versions::copy_version(client, path, number, to, clash(*c), *force, now())
                .await
                .map_err(with_force_hint)?;
            writeln!(w, "Copy started into {to}")?;
        }
        VersionAction::RestoreDeleted { paths } => {
            versions::restore_deleted(client, paths, now()).await?;
            writeln!(w, "Restored to where the items were deleted from")?;
        }
        VersionAction::ClearDeleted { paths, force } => {
            versions::clear_deleted(client, paths, *force, now())
                .await
                .map_err(with_force_hint)?;
            writeln!(w, "The versions of the deleted items are dropped")?;
        }
        _ => {}
    }
    Ok(())
}

/// Run a `version` subcommand.
pub async fn version(
    client: &UgosClient,
    action: &VersionAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    match action {
        VersionAction::Folders { kind: k } => {
            let all = versions::folders(client, kind(*k)).await?;
            let table: Vec<rows::FolderRow> = all.folders.iter().map(Into::into).collect();
            output::print_list(w, &table, fmt)
        }
        VersionAction::Set {
            name,
            enable,
            keep,
            rescan,
            force,
        } => {
            let change = FolderChange {
                enable: enable.map(OnOff::flag),
                keep: *keep,
                rescan: *rescan,
            };
            let f = versions::set_folder(client, name, &change, *force)
                .await
                .map_err(with_force_hint)?;
            output::print_list(w, &[rows::FolderRow::from(&f)], fmt)
        }
        VersionAction::Status => {
            let percent = versions::migration_progress(client).await?;
            writeln!(w, "History migration: {percent}%")?;
            Ok(())
        }
        VersionAction::Copy { .. }
        | VersionAction::RestoreDeleted { .. }
        | VersionAction::ClearDeleted { .. } => restore_cmd(client, action, w).await,
        _ => browse(client, action, fmt, w).await,
    }
}
