//! Configuration backup, restore and the two resets.

use std::path::Path;

use serde_json::{Value, json};

use super::refused;
use crate::api::syslog::SysLogApi;
use crate::client::UgosClient;
use crate::error::{Result, UgosError};
use crate::types::update::{BackupInfo, TaskProgress};

/// The largest file the web UI takes for a restore.
const LARGEST: u64 = 100 * 1024 * 1024;

/// Save the configuration of the NAS to a `.ugb` file.
///
/// UGOS only restores a file whose name contains `backup_restore`, so the
/// name is checked here as well.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn save_local(client: &UgosClient, dest: &Path) -> Result<u64> {
    let name = dest
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    if !name.contains("backup_restore") || dest.extension().and_then(|e| e.to_str()) != Some("ugb")
    {
        return Err(refused(
            "name the file backup_restore.ugb (UGOS restores only a .ugb with backup_restore in its name)",
        ));
    }
    let uid = client.current_user().await?.uid.to_string();
    let resp = client
        .get_bytes(
            "backup_restore/backupnow",
            &[("uid", &uid), ("backup_local", "1")],
        )
        .await?;
    let bytes = resp.bytes().await?;
    if bytes.is_empty() {
        return Err(refused("UGOS sent an empty backup"));
    }
    std::fs::write(dest, &bytes)
        .map_err(|e| UgosError::io(format!("writing '{}'", dest.display()), e))?;
    Ok(bytes.len() as u64)
}

/// Whether the cloud backup runs on a schedule.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn cloud_scheduled(client: &UgosClient) -> Result<bool> {
    let v: Value = client.get("backup_restore/status").await?;
    Ok(v.get("result").and_then(Value::as_i64).unwrap_or(0) != 0)
}

/// The latest cloud backup. Empty fields mean there is none.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn cloud_info(client: &UgosClient) -> Result<BackupInfo> {
    client
        .get_with_params("backup_restore/backup/info", &[("enable", "0")])
        .await
}

/// Progress of a backup (`0`) or a restore (`1`). `None` when no task runs.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn progress(client: &UgosClient, restore: bool) -> Result<Option<TaskProgress>> {
    let kind = if restore { "1" } else { "0" };
    match client
        .get_with_params("backup_restore/progress", &[("type", kind)])
        .await
    {
        Ok(p) => Ok(Some(p)),
        // 1109: there is no task in the queue.
        Err(UgosError::Api { code: 1109, .. }) => Ok(None),
        Err(e) => Err(e),
    }
}

/// A restorable service: where it sits in the tree, and its name.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct RestoreService {
    /// `category/item`.
    pub path: String,
    /// The service names behind it.
    pub names: Vec<String>,
}

/// Flatten the restore tree of `services/list`.
#[must_use]
pub fn flatten_services(tree: &Value) -> Vec<RestoreService> {
    let mut out = Vec::new();
    let Some(categories) = tree.as_object() else {
        return out;
    };
    for (category, items) in categories {
        let Some(items) = items.as_object() else {
            continue;
        };
        for (item, node) in items {
            let names: Vec<String> = match node {
                Value::Array(list) => list.iter().filter_map(service_name).collect(),
                other => service_name(other).into_iter().collect(),
            };
            let names: Vec<String> = names.into_iter().filter(|n| !n.is_empty()).collect();
            if !names.is_empty() {
                out.push(RestoreService {
                    path: format!("{category}/{item}"),
                    names,
                });
            }
        }
    }
    out
}

fn service_name(node: &Value) -> Option<String> {
    node.get("service_name")
        .and_then(Value::as_str)
        .map(str::to_owned)
}

/// The services a restore can bring back.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn services(client: &UgosClient) -> Result<Vec<RestoreService>> {
    let tree: Value = client.get("backup_restore/services/list").await?;
    Ok(flatten_services(&tree))
}

/// The names to restore for a choice of `category/item` paths, or all.
///
/// # Errors
///
/// Refuses a path that is not in the tree, and an empty choice.
pub fn pick_services(all: &[RestoreService], wanted: &[String]) -> Result<Vec<String>> {
    let mut names = Vec::new();
    if wanted.is_empty() {
        return Err(refused(
            "name the services to restore, or ask for all of them",
        ));
    }
    if wanted.iter().any(|w| w == "all") {
        for service in all {
            names.extend(service.names.iter().cloned());
        }
    } else {
        for want in wanted {
            let found = all
                .iter()
                .find(|s| {
                    s.path.eq_ignore_ascii_case(want)
                        || s.path.rsplit('/').next() == Some(want.as_str())
                })
                .ok_or_else(|| {
                    refused(format!(
                        "{want:?} is no restorable service; list them first"
                    ))
                })?;
            names.extend(found.names.iter().cloned());
        }
    }
    names.sort();
    names.dedup();
    Ok(names)
}

/// What the web UI checks about a backup file before it sends it: the name
/// and the size.
///
/// # Errors
///
/// Refuses a file that is not a `.ugb`, lacks `backup_restore` in its name,
/// or is empty or larger than 100 MiB.
pub fn check_backup(name: &str, size: u64) -> Result<()> {
    if Path::new(name).extension().and_then(|e| e.to_str()) != Some("ugb")
        || !name.contains("backup_restore")
    {
        return Err(refused(
            "a backup file is a .ugb with backup_restore in its name",
        ));
    }
    if size == 0 || size > LARGEST {
        return Err(refused("a backup file is up to 100 MiB"));
    }
    Ok(())
}

fn check_backup_file(file: &Path) -> Result<()> {
    let name = file
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    let size = std::fs::metadata(file)
        .map_err(|e| UgosError::io(format!("reading '{}'", file.display()), e))?
        .len();
    check_backup(name, size)
}

/// Restore services from a backup file. UGOS overwrites their configuration
/// and restarts them.
///
/// Returns the ports the web UI reconnects to.
///
/// # Errors
///
/// Returns [`UgosError::NeedsForce`] without `force`, and
/// [`UgosError::Refused`] for a file or a choice the web UI would refuse.
pub async fn restore(
    client: &UgosClient,
    file: &Path,
    wanted: &[String],
    force: bool,
) -> Result<Value> {
    check_backup_file(file)?;
    let names = pick_services(&services(client).await?, wanted)?;
    if !force {
        return Err(UgosError::NeedsForce(
            "a restore overwrites the configuration and restarts those services".into(),
        ));
    }
    let bytes = std::fs::read(file)
        .map_err(|e| UgosError::io(format!("reading '{}'", file.display()), e))?;
    let name = file
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("backup_restore.ugb")
        .to_owned();
    let uid = client.current_user().await?.uid.to_string();
    let form = reqwest::multipart::Form::new()
        .text("uid", uid)
        .text("is_local", "1")
        .part(
            "file",
            reqwest::multipart::Part::bytes(bytes).file_name(name),
        );
    let _: Value = client.post_multipart("backup_restore/import", form).await?;
    let info: BackupInfo = client.get("backup_restore/backup/info/local").await?;
    if info.last_backup_time == 0 {
        return Err(refused("UGOS found no configuration in that file"));
    }
    client
        .post(
            "backup_restore/restore",
            &json!({ "services": names, "is_local": true }),
        )
        .await
}

/// The addresses the NAS may have after a reset.
fn addresses(answer: &Value) -> Vec<String> {
    answer
        .get("result")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(|e| e.get("ipaddr").and_then(Value::as_str))
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// Reset the network settings to their defaults. The NAS may get another
/// address, and this session ends.
///
/// # Errors
///
/// Returns [`UgosError::NeedsForce`] without `force`.
pub async fn reset_network(client: &UgosClient, force: bool) -> Result<Vec<String>> {
    if !force {
        return Err(UgosError::NeedsForce(
            "resetting the network may give the NAS another address and ends this session".into(),
        ));
    }
    let answer: Value = client.get("backup_restore/network/reset").await?;
    Ok(addresses(&answer))
}

/// Reset the NAS to factory state: data and configuration are gone.
///
/// UGOS asks for the password of the account, which this client sends sealed.
///
/// # Errors
///
/// Returns [`UgosError::NeedsForce`] without `force`.
pub async fn reset_factory(client: &UgosClient, force: bool) -> Result<Vec<String>> {
    if !force {
        return Err(UgosError::NeedsForce(
            "a factory reset deletes the data and the configuration of the NAS for good".into(),
        ));
    }
    match client.get_confirmed::<Value>("backup_restore/reset").await {
        Ok(answer) => Ok(addresses(&answer)),
        // The device is already going down.
        Err(UgosError::Api { code: 403, .. }) => Ok(Vec::new()),
        Err(e) => Err(e),
    }
}
