//! The Log Center: the policies for archiving and notifying, the archive
//! viewer, clearing the log and exporting it.
//!
//! Clearing the log and archiving remove entries from it for good, so they
//! need `force`, as the web UI asks a confirmation.

use std::path::Path;

use serde_json::{Value, json};

use crate::client::UgosClient;
use crate::error::{Result, UgosError};
use crate::types::logcenter::{
    ArchiveConfig, ArchiveFile, ArchiveRecord, NotifyConfig, NotifyRecord, RecordPage, UsersModules,
};

fn refused(message: impl Into<String>) -> UgosError {
    UgosError::Refused(message.into())
}

/// The modules whose entries are in the event log.
pub const EVENT_MODULES: [&str; 11] = [
    "control_panel",
    "file_manager",
    "app_center",
    "storage_manager",
    "snapshot",
    "login",
    "vault",
    "iscsi",
    "camera",
    "voicememo",
    "other",
];

/// Users and modules that appear in the event log, or the transfer log.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn users_and_modules(client: &UgosClient, transfer: bool) -> Result<UsersModules> {
    let log_type = if transfer { "1" } else { "0" };
    client
        .get_with_params(
            "log/usersandmodules",
            &[("visualizer", "false"), ("log_type", log_type)],
        )
        .await
}

/// Whether file manager operations are logged.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn filemgr(client: &UgosClient) -> Result<bool> {
    let v: Value = client.get("log/filemgr/config").await?;
    Ok(v.get("status").and_then(Value::as_bool).unwrap_or(false))
}

/// Switch the logging of file manager operations, and read it back.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn set_filemgr(client: &UgosClient, on: bool) -> Result<bool> {
    let _: Value = client
        .post("log/filemgr/config", &json!({ "status": on }))
        .await?;
    let after = filemgr(client).await?;
    if after != on {
        return Err(refused("UGOS said yes but the setting did not change"));
    }
    Ok(after)
}

/// The archive policy.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn archive_config(client: &UgosClient) -> Result<ArchiveConfig> {
    client.get("log/archive/config").await
}

/// What to change about the archive policy.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ArchiveChange {
    /// Run archiving or not.
    pub status: Option<bool>,
    /// Archive at 0, 500 or 1024 MB.
    pub data_mb: Option<i64>,
    /// Archive at 0, 1 000 000, 5 000 000 or 10 000 000 entries.
    pub entries: Option<i64>,
    /// Archive entries older than 0, 3, 6 or 12 months.
    pub months: Option<i64>,
    /// The folder for the archives.
    pub path: Option<String>,
}

fn one_of(value: i64, allowed: &[i64], what: &str) -> Result<()> {
    if allowed.contains(&value) {
        Ok(())
    } else {
        Err(refused(format!("{what} is one of {allowed:?}")))
    }
}

/// Change the archive policy and read it back.
///
/// # Errors
///
/// Returns [`UgosError::NeedsForce`] when archiving is on afterwards: it moves
/// entries out of the log.
pub async fn set_archive(
    client: &UgosClient,
    change: &ArchiveChange,
    force: bool,
) -> Result<ArchiveConfig> {
    let mut next = archive_config(client).await?;
    next.status = change.status.unwrap_or(next.status);
    if let Some(v) = change.data_mb {
        one_of(v, &[0, 500, 1024], "the data size in MB")?;
        next.data_bytes = v;
    }
    if let Some(v) = change.entries {
        one_of(v, &[0, 1_000_000, 5_000_000, 10_000_000], "the entry count")?;
        next.num = v;
    }
    if let Some(v) = change.months {
        one_of(v, &[0, 3, 6, 12], "the months")?;
        next.remain_time = v;
    }
    if let Some(p) = &change.path {
        next.path.clone_from(p);
    }
    if next.status && next.path.is_empty() {
        return Err(refused("archiving needs a folder for the archives"));
    }
    if next.status && next.data_bytes == 0 && next.num == 0 && next.remain_time == 0 {
        return Err(refused(
            "archiving needs a trigger: size, entries or months",
        ));
    }
    if next.status && !force {
        return Err(UgosError::NeedsForce(
            "archiving moves the entries it archives out of the current log".into(),
        ));
    }
    let body = json!({
        "status": next.status, "data_bytes": next.data_bytes, "num": next.num,
        "remain_time": next.remain_time, "path": next.path,
    });
    let _: Value = client.post("log/archive/config", &body).await?;
    let after = archive_config(client).await?;
    // UGOS keeps the triggers only while archiving is on.
    let triggers_differ = next.status
        && (after.data_bytes != next.data_bytes
            || after.num != next.num
            || after.remain_time != next.remain_time);
    if after.status != next.status || after.path != next.path || triggers_differ {
        return Err(refused(
            "UGOS said yes but the archive policy did not change",
        ));
    }
    Ok(after)
}

/// The archives that were made.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn archive_records(client: &UgosClient, page: u32) -> Result<RecordPage<ArchiveRecord>> {
    client
        .get_with_params(
            "log/archive/record",
            &[("page", &page.to_string()), ("size", "20")],
        )
        .await
}

/// The archives the viewer knows.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn viewer_files(client: &UgosClient) -> Result<Vec<ArchiveFile>> {
    let v: Value = client.get("log/archive/visualizer/files").await?;
    match v.get("result") {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(list) => Ok(serde_json::from_value(list.clone())?),
    }
}

/// Add a `.ulog` archive to the viewer.
///
/// # Errors
///
/// Refuses a file that is not a `.ulog`.
pub async fn add_viewer_file(client: &UgosClient, path: &str) -> Result<()> {
    if Path::new(path).extension().and_then(|e| e.to_str()) != Some("ulog") {
        return Err(refused("an archive is a .ulog file"));
    }
    let v: Value = client
        .post("log/archive/visualizer/files", &json!({ "files": [path] }))
        .await?;
    if v.get("result").and_then(Value::as_bool) == Some(false) {
        return Err(refused("UGOS did not take that archive"));
    }
    Ok(())
}

/// Make an archive the active log source of the viewer.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn open_viewer_file(client: &UgosClient, id: i64) -> Result<()> {
    let _: Value = client
        .post(
            &format!("log/archive/visualizer/open?archive_id={id}"),
            &json!({}),
        )
        .await?;
    Ok(())
}

/// Remove an archive from the viewer. The file stays.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn remove_viewer_file(client: &UgosClient, id: i64) -> Result<()> {
    let _: Value = client
        .delete_with_body(
            &format!("log/archive/visualizer/files?archive_id={id}"),
            &json!({}),
        )
        .await?;
    Ok(())
}

/// The notification policy.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn notify_config(client: &UgosClient) -> Result<NotifyConfig> {
    client.get("log/notify/config").await
}

/// What to change about the notification policy.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NotifyChange {
    /// Notify or not.
    pub status: Option<bool>,
    /// A keyword or expression.
    pub keyword: Option<String>,
    /// `info`, `warning`, `critical`.
    pub levels: Option<Vec<String>>,
    /// Module ids.
    pub modules: Option<Vec<String>>,
    /// User names.
    pub operators: Option<Vec<String>>,
}

/// Change the notification policy and read it back.
///
/// # Errors
///
/// Refuses an unknown level or module, and a policy without any filter.
pub async fn set_notify(client: &UgosClient, change: &NotifyChange) -> Result<NotifyConfig> {
    let mut next = notify_config(client).await?;
    next.status = change.status.unwrap_or(next.status);
    if let Some(k) = &change.keyword {
        next.keywords = vec![k.clone()];
    }
    if let Some(levels) = &change.levels {
        if let Some(bad) = levels
            .iter()
            .find(|l| !["info", "warning", "critical"].contains(&l.as_str()))
        {
            return Err(refused(format!(
                "{bad:?} is no level: info, warning or critical"
            )));
        }
        next.levels.clone_from(levels);
    }
    if let Some(modules) = &change.modules {
        if let Some(bad) = modules
            .iter()
            .find(|m| !EVENT_MODULES.contains(&m.as_str()))
        {
            return Err(refused(format!("{bad:?} is no module of the event log")));
        }
        next.modules.clone_from(modules);
    }
    if let Some(operators) = &change.operators {
        next.operators.clone_from(operators);
    }
    let keyword = next.keywords.first().cloned().unwrap_or_default();
    if next.status
        && keyword.is_empty()
        && next.levels.is_empty()
        && next.modules.is_empty()
        && next.operators.is_empty()
    {
        return Err(refused("a notification needs at least one filter"));
    }
    let body = json!({
        "status": next.status, "keywords": [keyword], "levels": next.levels,
        "modules": next.modules, "operators": next.operators,
    });
    let _: Value = client.post("log/notify/config", &body).await?;
    let after = notify_config(client).await?;
    if after.status != next.status || after.levels != next.levels || after.modules != next.modules {
        return Err(refused(
            "UGOS said yes but the notification policy did not change",
        ));
    }
    Ok(after)
}

/// The notifications that were sent.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn notify_records(client: &UgosClient, page: u32) -> Result<RecordPage<NotifyRecord>> {
    client
        .get_with_params(
            "log/notify/record",
            &[("page", &page.to_string()), ("size", "20")],
        )
        .await
}

/// How much to clear.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Clear {
    /// Everything.
    All,
    /// What is older than this time, Unix seconds.
    Before(i64),
}

/// Clear the event log or the transfer log for good.
///
/// # Errors
///
/// Returns [`UgosError::NeedsForce`] without `force`.
pub async fn clear(client: &UgosClient, transfer: bool, what: Clear, force: bool) -> Result<()> {
    if !force {
        return Err(UgosError::NeedsForce(
            "cleared log entries cannot be retrieved".into(),
        ));
    }
    // UGOS wants an empty `log_type` for the event log.
    let log_type = if transfer { "1" } else { "" };
    let path = match what {
        Clear::All => format!("log/delete?log_type={log_type}"),
        Clear::Before(time) => format!("log/delete?log_type={log_type}&to_time={time}"),
    };
    let _: Value = client.delete_with_body(&path, &json!({})).await?;
    Ok(())
}

/// What to export.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Export {
    /// `csv` or `html`.
    pub html: bool,
    /// The transfer log instead of the event log.
    pub transfer: bool,
    /// From this time, Unix seconds.
    pub from: Option<i64>,
    /// Up to this time, Unix seconds.
    pub to: Option<i64>,
}

/// Save the log to a file.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn export(client: &UgosClient, what: &Export, dest: &Path) -> Result<u64> {
    let from = what.from.map(|t| t.to_string());
    let to = what.to.map(|t| t.to_string());
    let mut params = vec![
        ("visualizer", "false"),
        ("export_type", if what.html { "html" } else { "csv" }),
        ("lang", "en-US"),
    ];
    if what.transfer {
        params.push(("log_type", "1"));
    }
    if let Some(v) = &from {
        params.push(("from_time", v));
    }
    if let Some(v) = &to {
        params.push(("to_time", v));
    }
    let bytes = client
        .get_bytes("log/export", &params)
        .await?
        .bytes()
        .await?;
    std::fs::write(dest, &bytes)
        .map_err(|e| UgosError::io(format!("writing '{}'", dest.display()), e))?;
    Ok(bytes.len() as u64)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::{EVENT_MODULES, one_of};

    #[test]
    fn only_the_values_of_the_web_ui_pass() {
        assert!(one_of(500, &[0, 500, 1024], "size").is_ok());
        assert!(one_of(501, &[0, 500, 1024], "size").is_err());
        assert!(EVENT_MODULES.contains(&"login"));
        assert!(!EVENT_MODULES.contains(&"samba"));
    }
}
