//! File Version Explorer: the logic behind the `ugos_version_*` tools.
//!
//! The same operations as `ugos-cli`. What drops versions for good needs
//! `force`, and the model is told to ask the user first.

use rmcp::schemars;
use serde::Serialize;
use ugos_client::UgosClient;
use ugos_client::error::UgosError;
use ugos_client::ops::hw::parse::unix_utc;
use ugos_client::ops::versions::{self, Clash, FolderChange, FolderKind};

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

/// Whose folders.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct VersionFoldersParam {
    /// `share` (default), `user` or `domain`.
    #[serde(default)]
    pub kind: Option<String>,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// Version settings of a folder.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct VersionSetParam {
    /// The share or user, or its folder.
    pub name: String,
    /// Keep versions or not.
    #[serde(default)]
    pub enable: Option<bool>,
    /// How many versions of a file to keep, 1 to 32.
    #[serde(default)]
    pub keep: Option<i64>,
    /// Scan the folder again.
    #[serde(default)]
    pub rescan: bool,
    /// Go on although UGOS deletes versions for it. Ask the user first.
    #[serde(default)]
    pub force: bool,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// List a folder at a time.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct VersionLsParam {
    /// The folder.
    pub path: String,
    /// The time to look at, `YYYY-MM-DD HH:MM:SS` in UTC; now when not given.
    #[serde(default)]
    pub at: Option<String>,
    /// Include what was deleted since.
    #[serde(default)]
    pub deleted: bool,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// One file.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct VersionPathParam {
    /// The file, deleted or not.
    pub path: String,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// Copy a version.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct VersionCopyParam {
    /// The file or folder.
    pub path: String,
    /// The version, from the history tool.
    pub version: String,
    /// The destination folder.
    pub to: String,
    /// `skip`, `overwrite` or `keep_both` (default) when the name is taken.
    #[serde(default)]
    pub clash: Option<String>,
    /// Go on although this overwrites a file. Ask the user first.
    #[serde(default)]
    pub force: bool,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// Deleted items.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct VersionDeletedParam {
    /// Deleted files or folders.
    pub paths: Vec<String>,
    /// Clearing only: go on, the versions are gone for good. Ask the user
    /// first.
    #[serde(default)]
    pub force: bool,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// A month of the calendar.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct VersionCalendarParam {
    /// The year.
    pub year: i64,
    /// The month, 1 to 12.
    pub month: i64,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

pub async fn folders(c: &UgosClient, p: &VersionFoldersParam) -> String {
    let run = async {
        let kind = match p.kind.as_deref() {
            None | Some("share") => FolderKind::Share,
            Some("user") => FolderKind::User,
            Some("domain") => FolderKind::Domain,
            Some(_) => return Err(refused("the kind is share, user or domain")),
        };
        versions::folders(c, kind).await
    };
    json(run.await)
}

pub async fn set(c: &UgosClient, p: &VersionSetParam) -> String {
    let change = FolderChange {
        enable: p.enable,
        keep: p.keep,
        rescan: p.rescan,
    };
    json(versions::set_folder(c, &p.name, &change, p.force).await)
}

pub async fn status(c: &UgosClient) -> String {
    json(versions::migration_progress(c).await)
}

pub async fn ls(c: &UgosClient, p: &VersionLsParam) -> String {
    let run = async {
        let at =
            p.at.as_deref()
                .map(unix_utc)
                .transpose()?
                .unwrap_or_else(now);
        versions::list(c, &p.path, at, p.deleted).await
    };
    json(run.await)
}

pub async fn history(c: &UgosClient, p: &VersionPathParam) -> String {
    json(versions::history(c, &p.path, now()).await)
}

pub async fn copy(c: &UgosClient, p: &VersionCopyParam) -> String {
    let run = async {
        let clash = match p.clash.as_deref() {
            Some("skip") => Clash::Skip,
            Some("overwrite") => Clash::Overwrite,
            None | Some("keep_both") => Clash::KeepBoth,
            Some(_) => return Err(refused("clash is skip, overwrite or keep_both")),
        };
        versions::copy_version(c, &p.path, &p.version, &p.to, clash, p.force, now()).await
    };
    done(run.await, "The copy started")
}

pub async fn restore_deleted(c: &UgosClient, p: &VersionDeletedParam) -> String {
    done(
        versions::restore_deleted(c, &p.paths, now()).await,
        "Restored",
    )
}

pub async fn clear_deleted(c: &UgosClient, p: &VersionDeletedParam) -> String {
    done(
        versions::clear_deleted(c, &p.paths, p.force, now()).await,
        "The versions of the deleted items are dropped",
    )
}

pub async fn calendar(c: &UgosClient, p: &VersionCalendarParam) -> String {
    json(versions::calendar(c, p.year, p.month, now()).await)
}
