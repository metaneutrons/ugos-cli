//! The recycle bins of shared folders and user folders, and their cleanup
//! plans.
//!
//! A bin is a `#recycle` folder inside the share (`/volume1/docs/#recycle`)
//! or the home folder. Everything that deletes for good, or empties a bin,
//! refuses a path that is not inside one: the same call with a live path
//! would delete live files.

pub mod plan;

use serde_json::{Value, json};

use crate::api::files::FilesApi;
use crate::client::UgosClient;
use crate::error::{Result, UgosError};
use crate::types::recycle::{RecycleBin, RecycleConf, RecycleFile, RecycleList};

fn refused(message: impl Into<String>) -> UgosError {
    UgosError::Refused(message.into())
}

/// The switches of the recycle bin page.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn conf(client: &UgosClient) -> Result<RecycleConf> {
    client.get("filemgr/getRecycleConf").await
}

/// Change one switch, or both, and read them back.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn set_conf(
    client: &UgosClient,
    desktop: Option<bool>,
    folder: Option<bool>,
) -> Result<RecycleConf> {
    // UGOS takes one key per call.
    if let Some(v) = desktop {
        let _: Value = client
            .post("filemgr/setRecycleConf", &json!({ "desktop_recycle": v }))
            .await?;
    }
    if let Some(v) = folder {
        let _: Value = client
            .post("filemgr/setRecycleConf", &json!({ "dir_recycle": v }))
            .await?;
    }
    let after = conf(client).await?;
    if desktop.is_some_and(|v| v != after.desktop_recycle)
        || folder.is_some_and(|v| v != after.dir_recycle)
    {
        return Err(refused(
            "UGOS said yes but the recycle bin switches did not change",
        ));
    }
    Ok(after)
}

/// The recycle bins of all shares and of the user folders.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure, and for an account that
/// is not an administrator.
pub async fn bins(client: &UgosClient) -> Result<RecycleList> {
    client.get("v2/filemgr/getRecycleList").await
}

/// A bin by the name of its share or user, or by its folder.
///
/// # Errors
///
/// Refuses a name that no bin has.
pub async fn find(client: &UgosClient, name: &str) -> Result<RecycleBin> {
    let all = bins(client).await?;
    all.recycle_shares
        .into_iter()
        .chain(all.recycle_users)
        .find(|b| b.name == name || b.path == name || b.recycle_path == name)
        .ok_or_else(|| refused(format!("there is no recycle bin {name:?}")))
}

/// Switch a bin on or off.
///
/// # Errors
///
/// Returns [`UgosError::NeedsForce`] when a bin that holds files is switched
/// off: what UGOS then does to them is not known.
pub async fn switch(
    client: &UgosClient,
    name: &str,
    on: bool,
    admin_only: Option<bool>,
    force: bool,
) -> Result<RecycleBin> {
    let bin = find(client, name).await?;
    if !on && bin.recycle_not_empty && !force {
        return Err(UgosError::NeedsForce(format!(
            "the bin of {} holds files, and what UGOS does to them when it is off is not known",
            bin.name
        )));
    }
    let mut body = json!({ "path": bin.path, "status": on });
    if let Some(only) = admin_only.filter(|_| bin.user_id == 0) {
        body["allow_only_admin_recycle"] = json!(only);
    }
    let _: Value = client.post("filemgr/switchRecycle", &body).await?;
    let after = find(client, &bin.path).await?;
    if after.status != on {
        return Err(refused("UGOS said yes but the recycle bin did not change"));
    }
    Ok(after)
}

/// Where to look: one of the three roots, or a folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Place {
    /// The bin of the account itself.
    Personal,
    /// The bins of the shared folders.
    Common,
    /// The bins of the user folders.
    Users,
    /// A folder inside a bin.
    Path(String),
}

impl Place {
    const fn root(&self) -> Option<i64> {
        match self {
            Self::Personal => Some(2),
            Self::Common => Some(3),
            Self::Users => Some(4),
            Self::Path(_) => None,
        }
    }
}

/// Whether a path is a bin or inside one.
#[must_use]
pub fn in_recycle(path: &str) -> bool {
    path.split('/').any(|part| part == "#recycle")
}

/// List a place.
///
/// # Errors
///
/// Refuses a folder that is not inside a bin.
pub async fn list(client: &UgosClient, place: &Place) -> Result<Vec<RecycleFile>> {
    let mut body = json!({
        "limit": 2000, "page": 1, "dir_recycle": false, "data_type": 2,
        "left_no_page_show": false, "is_access_recycle": true, "left_count": 5000,
        "sort_type": 7, "reverse": true,
    });
    match place {
        Place::Path(p) => {
            if !in_recycle(p) {
                return Err(refused("that folder is not inside a recycle bin"));
            }
            body["path"] = json!(p);
        }
        other => {
            body["root_type"] = json!(other.root());
            body["permission"] = json!(if matches!(other, Place::Common) { 4 } else { 0 });
        }
    }
    let answer: Value = client
        .post_encrypted("v2/filemgr/getDirFileListV2", &body)
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

fn check_paths(paths: &[String]) -> Result<()> {
    if paths.is_empty() {
        return Err(refused("name at least one path inside a recycle bin"));
    }
    if let Some(bad) = paths.iter().find(|p| !in_recycle(p)) {
        return Err(refused(format!("{bad} is not inside a recycle bin")));
    }
    Ok(())
}

/// Put files back where they were deleted from. UGOS takes no destination
/// and no rule for a name that is taken.
///
/// # Errors
///
/// Refuses a path that is not inside a bin.
pub async fn restore(client: &UgosClient, paths: &[String]) -> Result<()> {
    check_paths(paths)?;
    let _: Value = client
        .post("filemgr/restoreRecycle", &json!({ "paths": paths }))
        .await?;
    Ok(())
}

/// Empty bins, by the path of their `#recycle` folder, or a whole root. The
/// files are gone for good.
///
/// # Errors
///
/// Returns [`UgosError::NeedsForce`] without `force`.
pub async fn empty(
    client: &UgosClient,
    paths: &[String],
    root: Option<&Place>,
    force: bool,
) -> Result<()> {
    let mut body = json!({ "paths": paths });
    match root.and_then(Place::root) {
        Some(r) if paths.is_empty() => body["root_type"] = json!(r),
        _ => check_paths(paths)?,
    }
    if !force {
        return Err(UgosError::NeedsForce(
            "emptying a recycle bin deletes what is in it for good".into(),
        ));
    }
    let _: Value = client.post("filemgr/emptyRecycleBin", &body).await?;
    Ok(())
}

/// Delete files in a bin for good.
///
/// # Errors
///
/// Returns [`UgosError::NeedsForce`] without `force`, and refuses a path that
/// is not inside a bin: the same call would delete a live file.
pub async fn delete_forever(client: &UgosClient, paths: &[String], force: bool) -> Result<()> {
    check_paths(paths)?;
    if !force {
        return Err(UgosError::NeedsForce(
            "deleting from a recycle bin cannot be undone".into(),
        ));
    }
    client.fs_remove(paths, true).await
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests;
