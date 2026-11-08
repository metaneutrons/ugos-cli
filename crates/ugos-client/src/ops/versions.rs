//! The File Version Explorer: which folders keep versions, the history of a
//! file, and getting an older version or a deleted file back.
//!
//! Switching versions off, keeping fewer of them and clearing the history of
//! deleted files remove versions for good, so they need `force`, as the web
//! UI asks a confirmation.

use serde_json::{Value, json};

use crate::client::UgosClient;
use crate::error::{Result, UgosError};
use crate::types::versions::{FileVersion, MonthMarks, VersionFolder, VersionFolders, VersionItem};

fn refused(message: impl Into<String>) -> UgosError {
    UgosError::Refused(message.into())
}

/// Whose folders.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FolderKind {
    /// Shared folders.
    Share,
    /// Home folders.
    User,
    /// Home folders of domain users.
    Domain,
}

impl FolderKind {
    const fn name(self) -> &'static str {
        match self {
            Self::Share => "share",
            Self::User => "user",
            Self::Domain => "domain",
        }
    }
}

/// The folders of a kind and their version settings.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn folders(client: &UgosClient, kind: FolderKind) -> Result<VersionFolders> {
    client
        .get_with_params(
            "versionExplorer/folderInfo",
            &[
                ("type", kind.name()),
                ("page", "1"),
                ("size", "2048"),
                ("name", ""),
            ],
        )
        .await
}

/// A folder by name or path, searching shares and then home folders.
///
/// # Errors
///
/// Refuses a name that no folder has.
pub async fn find(client: &UgosClient, name: &str) -> Result<VersionFolder> {
    for kind in [FolderKind::Share, FolderKind::User, FolderKind::Domain] {
        if let Some(f) = folders(client, kind)
            .await?
            .folders
            .into_iter()
            .find(|f| f.name == name || f.path == name)
        {
            return Ok(f);
        }
    }
    Err(refused(format!(
        "there is no folder {name:?} that can keep versions"
    )))
}

/// What to change about a folder.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FolderChange {
    /// Keep versions or not.
    pub enable: Option<bool>,
    /// How many versions of a file to keep, 1 to 32.
    pub keep: Option<i64>,
    /// Scan the folder again, for a folder that is on.
    pub rescan: bool,
}

fn change_error(e: UgosError) -> UgosError {
    match e {
        UgosError::Api { code: 5009, .. } => {
            refused("the volume has not enough space for the versions")
        }
        UgosError::Api { code: 50000, .. } => refused("UGOS does not find that folder"),
        UgosError::Api { code: 50002, .. } => refused("the volume cannot keep versions"),
        UgosError::Api { code: 50003, .. } => {
            refused("UGOS migrates the old histories: wait for it (version status)")
        }
        UgosError::Api { code: 50001, msg } => {
            refused(format!("UGOS still deletes old versions: {msg}"))
        }
        other => other,
    }
}

/// Change the version settings of a folder and read them back.
///
/// # Errors
///
/// Returns [`UgosError::NeedsForce`] for switching versions off and for
/// keeping fewer of them: UGOS deletes the versions that no longer fit.
pub async fn set_folder(
    client: &UgosClient,
    name: &str,
    change: &FolderChange,
    force: bool,
) -> Result<VersionFolder> {
    let folder = find(client, name).await?;
    let enable = change.enable.unwrap_or(folder.enable);
    let keep = change.keep.unwrap_or(if folder.save_number == 0 {
        5
    } else {
        folder.save_number
    });
    if !(1..=32).contains(&keep) {
        return Err(refused("keep 1 to 32 versions"));
    }
    if change.rescan && !folder.enable {
        return Err(refused("rescanning needs a folder that keeps versions"));
    }
    let drops = folder.enable && (!enable || keep < folder.save_number);
    if drops && !force {
        return Err(UgosError::NeedsForce(
            "UGOS deletes the versions that are switched off or no longer fit".into(),
        ));
    }
    let body = json!({
        "name": folder.name, "path": folder.path, "save_number": keep,
        "enable": enable, "rescan": change.rescan,
    });
    let sent: Result<Value> = client.post("versionExplorer/folderInfo", &body).await;
    let _ = sent.map_err(change_error)?;
    let after = find(client, &folder.path).await?;
    if after.enable != enable || (enable && after.save_number != keep) {
        return Err(refused(
            "UGOS said yes but the version settings did not change",
        ));
    }
    Ok(after)
}

/// How far UGOS is with migrating old histories, in percent.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn migration_progress(client: &UgosClient) -> Result<i64> {
    let v: Value = client
        .get("versionExplorer/dataCompatibilityProgress")
        .await?;
    Ok(v.get("result").and_then(Value::as_i64).unwrap_or(0))
}

/// List a folder as it was at a time, with the files deleted since if asked.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn list(
    client: &UgosClient,
    path: &str,
    at: i64,
    show_deleted: bool,
) -> Result<Vec<VersionItem>> {
    let body = json!({
        "path": path, "deadline_timestamp": at, "show_deleted": show_deleted,
        "data_type": 2, "sort_type": 1, "reverse": false, "page": 1, "limit": 2000,
        "left_count": 5000,
    });
    let answer: Value = client
        .post_encrypted("v2/versionExplorer/versionListInfo", &body)
        .await?;
    let files = answer
        .pointer("/right_files/files")
        .cloned()
        .unwrap_or(Value::Null);
    if files.is_null() {
        return Ok(Vec::new());
    }
    Ok(serde_json::from_value(files)?)
}

/// The folder part of a path.
fn parent(path: &str) -> &str {
    match path.trim_end_matches('/').rsplit_once('/') {
        Some(("", _)) | None => "/",
        Some((dir, _)) => dir,
    }
}

/// An item of a folder listing by its full path, deleted ones included.
async fn item(client: &UgosClient, path: &str, now: i64) -> Result<VersionItem> {
    let path = path.trim_end_matches('/');
    list(client, parent(path), now, true)
        .await?
        .into_iter()
        .find(|i| i.path == path)
        .ok_or_else(|| refused(format!("UGOS lists no {path}, deleted or not")))
}

/// An inode as query text: UGOS sends a number or a text.
fn inode_text(inode: &Value) -> String {
    inode
        .as_str()
        .map_or_else(|| inode.to_string(), str::to_owned)
}

/// Every version of one file.
///
/// # Errors
///
/// Refuses a path that is not listed.
pub async fn history(client: &UgosClient, path: &str, now: i64) -> Result<Vec<FileVersion>> {
    let found = item(client, path, now).await?;
    let answer: Value = client
        .get_with_params(
            "versionExplorer/getVersionAllList",
            &[
                ("path", &found.path),
                ("inode", &inode_text(&found.inode)),
                ("uuid", &found.uuid),
            ],
        )
        .await?;
    match answer.get("result") {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(list) => Ok(serde_json::from_value(list.clone())?),
    }
}

/// What to do when the destination has a file of that name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Clash {
    /// Leave the old file.
    Skip,
    /// Replace the old file.
    Overwrite,
    /// Keep both, under a new name.
    KeepBoth,
}

/// Copy an older version of a file or folder into a folder on the NAS. It is
/// a copy: the live file stays as it is.
///
/// # Errors
///
/// Returns [`UgosError::NeedsForce`] for `Overwrite`, and refuses a version
/// the file does not have.
pub async fn copy_version(
    client: &UgosClient,
    path: &str,
    version_number: &str,
    dst: &str,
    clash: Clash,
    force: bool,
    now: i64,
) -> Result<()> {
    if clash == Clash::Overwrite && !force {
        return Err(UgosError::NeedsForce(
            "overwriting replaces the file of that name in the destination".into(),
        ));
    }
    let versions = history(client, path, now).await?;
    let version = versions
        .iter()
        .find(|v| v.version_number == version_number)
        .ok_or_else(|| refused(format!("the file has no version {version_number}")))?;
    let check: Value = client
        .get_with_params("filemgr/determinePath", &[("path", dst)])
        .await?;
    if check
        .get("result")
        .is_some_and(|r| !r.is_null() && r != &json!(""))
    {
        return Err(refused(format!("{dst} is not a folder UGOS can copy into")));
    }
    let body = json!({
        "paths": [{ "path": version.path, "file_type": version.file_type,
                    "version_number": version.version_number }],
        "dst": dst, "deadline_timestamp": version.version_timestamp.to_string(),
        "operation_type": match clash { Clash::Skip => 0, Clash::Overwrite => 1, Clash::KeepBoth => 3 },
        "contain_delete": true,
    });
    let _: Value = client
        .post_encrypted("v2/versionExplorer/copyVersionToNAS", &body)
        .await?;
    Ok(())
}

async fn deleted_items(client: &UgosClient, paths: &[String], now: i64) -> Result<Vec<Value>> {
    if paths.is_empty() {
        return Err(refused("name at least one deleted file or folder"));
    }
    let mut out = Vec::new();
    for path in paths {
        let found = item(client, path, now).await?;
        if !found.is_deleted {
            return Err(refused(format!("{path} is not deleted")));
        }
        out.push(json!({
            "path": found.path, "inode": found.inode, "uuid": found.uuid,
            "is_dir": found.file_type == 1,
        }));
    }
    Ok(out)
}

/// Put deleted files or folders back where they were.
///
/// # Errors
///
/// Refuses a path that is not a deleted item.
pub async fn restore_deleted(client: &UgosClient, paths: &[String], now: i64) -> Result<()> {
    let body = deleted_items(client, paths, now).await?;
    let _: Value = client.post("versionExplorer/restoreDelDir", &body).await?;
    Ok(())
}

/// Drop all versions of deleted files or folders for good.
///
/// # Errors
///
/// Returns [`UgosError::NeedsForce`] without `force`.
pub async fn clear_deleted(
    client: &UgosClient,
    paths: &[String],
    force: bool,
    now: i64,
) -> Result<()> {
    let body = deleted_items(client, paths, now).await?;
    if !force {
        return Err(UgosError::NeedsForce(
            "the versions of these deleted items are dropped for good".into(),
        ));
    }
    let _: Value = client
        .post("versionExplorer/clearDelDirVersion", &body)
        .await?;
    Ok(())
}

/// The days of a month with something made, changed or deleted.
///
/// # Errors
///
/// Refuses a month outside 1 to 12.
pub async fn calendar(client: &UgosClient, year: i64, month: i64, now: i64) -> Result<MonthMarks> {
    if !(1..=12).contains(&month) {
        return Err(refused("the month is 1 to 12"));
    }
    client
        .get_with_params(
            "versionExplorer/getMonthVersion",
            &[
                ("year", &year.to_string()),
                ("month", &month.to_string()),
                ("current_time", &now.to_string()),
            ],
        )
        .await
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::parent;

    #[test]
    fn the_parent_of_a_path_is_its_folder() {
        assert_eq!(parent("/volume1/docs/a.txt"), "/volume1/docs");
        assert_eq!(parent("/volume1/docs/dir/"), "/volume1/docs");
        assert_eq!(parent("/volume1"), "/");
        assert_eq!(parent("a"), "/");
    }
}
