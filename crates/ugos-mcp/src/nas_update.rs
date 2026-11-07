//! Firmware, configuration backup, the thumbnail service and the about and
//! power tools.
//!
//! The same operations as `ugos-cli`. What installs, restores or resets needs
//! `force`, and the model is told to ask the user first.

use std::path::Path;

use rmcp::schemars;
use serde::Serialize;
use ugos_client::UgosClient;
use ugos_client::error::UgosError;
use ugos_client::ops::hw::{poweroff, reboot};
use ugos_client::ops::update::firmware::{self, ConfigChange, Interval};
use ugos_client::ops::update::{about, backup, index};

use crate::manage::failure;
use crate::nas_hw::ForceParam;

fn json<T: Serialize>(result: Result<T, UgosError>) -> String {
    match result {
        Ok(value) => serde_json::to_string_pretty(&value).unwrap_or_default(),
        Err(e) => failure(&e),
    }
}

fn said(result: Result<(), UgosError>, text: &str) -> String {
    result.map_or_else(|e| failure(&e), |()| text.to_owned())
}

/// A file on the machine that runs this server.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct FileParam {
    /// Absolute path of the file.
    pub file: String,
    /// Go on although this installs, trusts or overwrites. Ask the user
    /// first.
    #[serde(default)]
    pub force: bool,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// Where to write a backup.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SaveParam {
    /// Absolute path, named like `backup_restore.ugb`.
    pub file: String,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// How the NAS looks for updates.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct UpdateConfigParam {
    /// `notify`, `important` or `any`. The last two install by themselves and
    /// restart the NAS, which needs force.
    #[serde(default)]
    pub auto: Option<String>,
    /// Check every this many hours, days or months (1 to 72), with `unit`.
    #[serde(default)]
    pub every: Option<i64>,
    /// `hours`, `days` or `months`.
    #[serde(default)]
    pub unit: Option<String>,
    /// Let UGOS decide how often to check.
    #[serde(default)]
    pub default_interval: bool,
    /// Go on although the NAS then installs updates by itself. Ask the user
    /// first.
    #[serde(default)]
    pub force: bool,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// Restore services from a backup file.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct RestoreParam {
    /// Absolute path of `backup_restore.ugb`.
    pub file: String,
    /// Services as `category/item` or the item, or `all`.
    pub services: Vec<String>,
    /// Go on although the restore overwrites the configuration and restarts
    /// those services. Ask the user first.
    #[serde(default)]
    pub force: bool,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// Progress of a backup or a restore.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ProgressParam {
    /// A restore, not a backup.
    #[serde(default)]
    pub restore: bool,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// Pause the thumbnail service.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PauseParam {
    /// 1, 2 or 5 hours.
    pub hours: i64,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// Rebuild thumbnails.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct RegenerateParam {
    /// All ranges.
    #[serde(default)]
    pub all: bool,
    /// Ids of ranges, from the ranges tool.
    #[serde(default)]
    pub ranges: Vec<i64>,
    /// Only the thumbnails that are missing.
    #[serde(default)]
    pub missing_only: bool,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

pub async fn status(c: &UgosClient) -> String {
    let run = async {
        let status = firmware::status(c).await?;
        let flag = firmware::is_new(c).await?;
        Ok(serde_json::json!({ "status": status, "newer_exists": flag.has_version }))
    };
    json(run.await)
}

pub async fn config(c: &UgosClient, p: &UpdateConfigParam) -> String {
    let run = async {
        let auto_update = match p.auto.as_deref() {
            None => None,
            Some("notify") => Some(0),
            Some("important") => Some(1),
            Some("any") => Some(2),
            Some(_) => {
                return Err(UgosError::Refused(
                    "auto is notify, important or any".into(),
                ));
            }
        };
        let interval = match (p.every, p.unit.as_deref(), p.default_interval) {
            (None, None, false) => None,
            (None, None, true) => Some(Interval::Default),
            (Some(value), Some(unit), false) => {
                let unit = match unit {
                    "hours" => 1,
                    "days" => 2,
                    "months" => 3,
                    _ => return Err(UgosError::Refused("unit is hours, days or months".into())),
                };
                Some(Interval::Every { value, unit })
            }
            _ => {
                return Err(UgosError::Refused(
                    "every needs unit, and default_interval stands alone".into(),
                ));
            }
        };
        let change = ConfigChange {
            auto_update,
            interval,
        };
        if change == ConfigChange::default() {
            firmware::config(c).await
        } else {
            firmware::set_config(c, &change, p.force).await
        }
    };
    json(run.await)
}

pub async fn install(c: &UgosClient, p: &ForceParam) -> String {
    json(firmware::install(c, p.force).await)
}

pub async fn upload(c: &UgosClient, p: &FileParam) -> String {
    said(
        firmware::upload(c, Path::new(&p.file), p.force, &|_, _| {}).await,
        "The image is sent; the NAS installs it and restarts. Follow it with the progress tool",
    )
}

pub async fn progress(c: &UgosClient) -> String {
    json(firmware::manual_status(c).await)
}

pub async fn key(c: &UgosClient, p: &FileParam) -> String {
    said(
        firmware::import_key(c, Path::new(&p.file), p.force).await,
        "The key is trusted",
    )
}

pub async fn clean(c: &UgosClient) -> String {
    said(firmware::clean(c).await, "The stored failure is cleared")
}

pub async fn backup_save(c: &UgosClient, p: &SaveParam) -> String {
    json(backup::save_local(c, Path::new(&p.file)).await)
}

pub async fn backup_info(c: &UgosClient) -> String {
    let run = async {
        Ok(serde_json::json!({
            "scheduled": backup::cloud_scheduled(c).await?,
            "latest": backup::cloud_info(c).await?,
        }))
    };
    json(run.await)
}

pub async fn backup_services(c: &UgosClient) -> String {
    json(backup::services(c).await)
}

pub async fn backup_restore(c: &UgosClient, p: &RestoreParam) -> String {
    said(
        backup::restore(c, Path::new(&p.file), &p.services, p.force)
            .await
            .map(|_| ()),
        "The restore started; follow it with the backup progress tool (restore: true)",
    )
}

pub async fn backup_progress(c: &UgosClient, p: &ProgressParam) -> String {
    json(backup::progress(c, p.restore).await)
}

pub async fn reset_network(c: &UgosClient, p: &ForceParam) -> String {
    json(backup::reset_network(c, p.force).await)
}

pub async fn reset_factory(c: &UgosClient, p: &ForceParam) -> String {
    json(backup::reset_factory(c, p.force).await)
}

pub async fn index_status(c: &UgosClient) -> String {
    json(index::status(c).await)
}

pub async fn index_ranges(c: &UgosClient) -> String {
    json(index::ranges(c).await)
}

pub async fn index_pause(c: &UgosClient, p: &PauseParam) -> String {
    json(index::pause(c, p.hours).await)
}

pub async fn index_resume(c: &UgosClient) -> String {
    json(index::resume(c).await)
}

pub async fn index_regenerate(c: &UgosClient, p: &RegenerateParam) -> String {
    json(index::regenerate(c, p.all, p.missing_only, &p.ranges).await)
}

pub async fn about_show(c: &UgosClient) -> String {
    let run = async {
        let machine = about::machine(c).await?;
        let (start, end) = about::repair(c).await?;
        Ok(
            serde_json::json!({ "machine": machine, "warranty_start_ms": start, "warranty_end_ms": end }),
        )
    };
    json(run.await)
}

pub async fn ports(c: &UgosClient) -> String {
    json(about::ports(c).await)
}

pub async fn power(c: &UgosClient, p: &ForceParam, off: bool) -> String {
    if !p.force {
        return failure(&UgosError::NeedsForce(
            if off {
                "this powers the NAS off"
            } else {
                "this restarts the NAS"
            }
            .into(),
        ));
    }
    if off {
        said(poweroff(c).await, "The NAS powers off")
    } else {
        said(reboot(c).await, "The NAS restarts")
    }
}
