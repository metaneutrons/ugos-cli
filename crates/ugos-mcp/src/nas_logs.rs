//! The system log and the Log Center: the logic behind the `ugos_syslog` and
//! `ugos_logcenter_*` tools.
//!
//! The same operations as `ugos-cli`. Clearing the log and archiving remove
//! entries from it and need `force`, and the model is told to ask the user
//! first.

use std::path::Path;

use rmcp::schemars;
use serde::Serialize;
use ugos_client::UgosClient;
use ugos_client::api::syslog::{LogFilter, SysLogApi};
use ugos_client::error::UgosError;
use ugos_client::ops::hw::parse::unix_utc;
use ugos_client::ops::logcenter::{self, ArchiveChange, Clear, Export, NotifyChange};

use crate::manage::failure;

fn json<T: Serialize>(result: Result<T, UgosError>) -> String {
    match result {
        Ok(value) => serde_json::to_string_pretty(&value).unwrap_or_default(),
        Err(e) => failure(&e),
    }
}

fn done(result: Result<(), UgosError>, text: &str) -> String {
    result.map_or_else(|e| failure(&e), |()| text.to_owned())
}

fn refused(message: &str) -> UgosError {
    UgosError::Refused(message.into())
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_secs()).ok())
        .unwrap_or(0)
}

/// Query the log of the NAS.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SyslogParam {
    /// A module, like `login`.
    #[serde(default)]
    pub module: Option<String>,
    /// A level: `info`, `warning` or `critical`.
    #[serde(default)]
    pub level: Option<String>,
    /// A user.
    #[serde(default)]
    pub operator: Option<String>,
    /// Free text.
    #[serde(default)]
    pub keyword: Option<String>,
    /// The page, from 1.
    #[serde(default)]
    pub page: Option<u32>,
    /// Entries per page, 20 when not given.
    #[serde(default)]
    pub size: Option<u32>,
    /// The transfer log (Samba, FTP, `WebDAV`, file manager).
    #[serde(default)]
    pub transfer: bool,
    /// From this time on, `YYYY-MM-DD HH:MM:SS` in UTC.
    #[serde(default)]
    pub from: Option<String>,
    /// Up to this time, `YYYY-MM-DD HH:MM:SS` in UTC.
    #[serde(default)]
    pub to: Option<String>,
    /// Oldest first.
    #[serde(default)]
    pub oldest_first: bool,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// Which log.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TransferParam {
    /// The transfer log instead of the event log.
    #[serde(default)]
    pub transfer: bool,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// A page of a record list.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct RecordsParam {
    /// The page, from 0.
    #[serde(default)]
    pub page: u32,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// The archive policy.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ArchiveSetParam {
    /// Run archiving or not.
    #[serde(default)]
    pub enable: Option<bool>,
    /// Archive at this many MB: 0, 500 or 1024.
    #[serde(default)]
    pub mb: Option<i64>,
    /// Archive at this many entries: 0, 1000000, 5000000 or 10000000.
    #[serde(default)]
    pub entries: Option<i64>,
    /// Archive entries older than this many months: 0, 3, 6 or 12.
    #[serde(default)]
    pub months: Option<i64>,
    /// The folder for the archives.
    #[serde(default)]
    pub path: Option<String>,
    /// Go on although archiving moves entries out of the log. Ask the user
    /// first.
    #[serde(default)]
    pub force: bool,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// An archive in the viewer.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ArchiveRefParam {
    /// The archive id, for open and remove.
    #[serde(default)]
    pub id: Option<i64>,
    /// Path of a `.ulog` file, for add.
    #[serde(default)]
    pub path: Option<String>,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// The notification policy.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct NotifySetParam {
    /// Notify or not.
    #[serde(default)]
    pub enable: Option<bool>,
    /// A keyword or expression.
    #[serde(default)]
    pub keyword: Option<String>,
    /// Levels: `info`, `warning`, `critical`.
    #[serde(default)]
    pub levels: Option<Vec<String>>,
    /// Modules, like `login`.
    #[serde(default)]
    pub modules: Option<Vec<String>>,
    /// User names.
    #[serde(default)]
    pub operators: Option<Vec<String>>,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// The file manager logging switch.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct FilemgrParam {
    /// On or off; just read when not given.
    #[serde(default)]
    pub on: Option<bool>,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// Clear the log.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ClearParam {
    /// The transfer log instead of the event log.
    #[serde(default)]
    pub transfer: bool,
    /// Everything.
    #[serde(default)]
    pub all: bool,
    /// Entries older than this many days.
    #[serde(default)]
    pub older_than_days: Option<i64>,
    /// Entries older than this time, `YYYY-MM-DD HH:MM:SS` in UTC.
    #[serde(default)]
    pub before: Option<String>,
    /// Go on: cleared entries cannot be retrieved. Ask the user first.
    #[serde(default)]
    pub force: bool,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// Save the log to a file.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ExportParam {
    /// Absolute path to write on this machine.
    pub file: String,
    /// HTML instead of CSV.
    #[serde(default)]
    pub html: bool,
    /// The transfer log instead of the event log.
    #[serde(default)]
    pub transfer: bool,
    /// From this time on, `YYYY-MM-DD HH:MM:SS` in UTC.
    #[serde(default)]
    pub from: Option<String>,
    /// Up to this time, `YYYY-MM-DD HH:MM:SS` in UTC.
    #[serde(default)]
    pub to: Option<String>,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

pub async fn syslog(c: &UgosClient, p: &SyslogParam) -> String {
    let run = async {
        let filter = LogFilter {
            module: p.module.as_deref(),
            level: p.level.as_deref(),
            operator: p.operator.as_deref(),
            keyword: p.keyword.as_deref(),
            page: p.page.unwrap_or(1),
            size: p.size.unwrap_or(20),
            transfer: p.transfer,
            from: p.from.as_deref().map(unix_utc).transpose()?,
            to: p.to.as_deref().map(unix_utc).transpose()?,
            ascending: p.oldest_first,
        };
        c.syslog(&filter).await
    };
    json(run.await)
}

pub async fn modules(c: &UgosClient, p: &TransferParam) -> String {
    json(logcenter::users_and_modules(c, p.transfer).await)
}

pub async fn archive_show(c: &UgosClient) -> String {
    json(logcenter::archive_config(c).await)
}

pub async fn archive_set(c: &UgosClient, p: &ArchiveSetParam) -> String {
    let change = ArchiveChange {
        status: p.enable,
        data_mb: p.mb,
        entries: p.entries,
        months: p.months,
        path: p.path.clone(),
    };
    json(logcenter::set_archive(c, &change, p.force).await)
}

pub async fn archive_records(c: &UgosClient, p: &RecordsParam) -> String {
    json(logcenter::archive_records(c, p.page).await)
}

pub async fn archive_files(c: &UgosClient) -> String {
    json(logcenter::viewer_files(c).await)
}

pub async fn archive_add(c: &UgosClient, p: &ArchiveRefParam) -> String {
    let run = async {
        let path = p
            .path
            .as_deref()
            .ok_or_else(|| refused("name the .ulog file: path"))?;
        logcenter::add_viewer_file(c, path).await
    };
    done(run.await, "Added to the viewer")
}

pub async fn archive_open(c: &UgosClient, p: &ArchiveRefParam) -> String {
    let run = async {
        let id = p.id.ok_or_else(|| refused("name the archive: id"))?;
        logcenter::open_viewer_file(c, id).await
    };
    done(run.await, "The archive is the active log source")
}

pub async fn archive_remove(c: &UgosClient, p: &ArchiveRefParam) -> String {
    let run = async {
        let id = p.id.ok_or_else(|| refused("name the archive: id"))?;
        logcenter::remove_viewer_file(c, id).await
    };
    done(run.await, "Removed from the viewer")
}

pub async fn notify_show(c: &UgosClient) -> String {
    json(logcenter::notify_config(c).await)
}

pub async fn notify_set(c: &UgosClient, p: &NotifySetParam) -> String {
    let change = NotifyChange {
        status: p.enable,
        keyword: p.keyword.clone(),
        levels: p.levels.clone(),
        modules: p.modules.clone(),
        operators: p.operators.clone(),
    };
    json(logcenter::set_notify(c, &change).await)
}

pub async fn notify_records(c: &UgosClient, p: &RecordsParam) -> String {
    json(logcenter::notify_records(c, p.page).await)
}

pub async fn filemgr(c: &UgosClient, p: &FilemgrParam) -> String {
    let run = async {
        match p.on {
            Some(on) => logcenter::set_filemgr(c, on).await,
            None => logcenter::filemgr(c).await,
        }
    };
    json(run.await)
}

pub async fn clear(c: &UgosClient, p: &ClearParam) -> String {
    let run = async {
        let what = match (p.all, p.older_than_days, &p.before) {
            (true, None, None) => Clear::All,
            (false, Some(days), None) if days > 0 => Clear::Before(now() - days * 86_400),
            (false, None, Some(t)) => Clear::Before(unix_utc(t)?),
            _ => return Err(refused("give all, older_than_days or before")),
        };
        logcenter::clear(c, p.transfer, what, p.force).await
    };
    done(run.await, "The log is cleared")
}

pub async fn export(c: &UgosClient, p: &ExportParam) -> String {
    let run = async {
        let what = Export {
            html: p.html,
            transfer: p.transfer,
            from: p.from.as_deref().map(unix_utc).transpose()?,
            to: p.to.as_deref().map(unix_utc).transpose()?,
        };
        logcenter::export(c, &what, Path::new(&p.file)).await
    };
    json(run.await)
}
