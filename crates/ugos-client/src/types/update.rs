//! Types for firmware updates, configuration backups and the about pages.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The firmware that UGOS found.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteFirmware {
    /// Version name, like `1.21.1.0016`.
    #[serde(default, rename = "versionName")]
    pub version: String,
    /// `1` marks an important update.
    #[serde(default, rename = "versionType")]
    pub version_type: i64,
    /// Whether it is a beta.
    #[serde(default)]
    pub beta: bool,
    /// Size in bytes.
    #[serde(default, rename = "byteSize")]
    pub bytes: i64,
    /// The model it is for.
    #[serde(default, rename = "modelName")]
    pub model: String,
    /// Release time, Unix milliseconds.
    #[serde(default)]
    pub pubtime: i64,
    /// What changed, in the language of the NAS.
    #[serde(default)]
    pub desc: String,
}

/// `firmware/status`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirmwareStatus {
    /// The installed version.
    #[serde(default)]
    pub current_version: String,
    /// `0` failed, `1` up to date, `2` update available, `3` checking,
    /// `4` manual upload running.
    #[serde(default)]
    pub current_stat: i64,
    /// `download`, `install` or `complete`.
    #[serde(default)]
    pub current_stage: String,
    /// Progress in percent, `-1` after a failure.
    #[serde(default)]
    pub process: i64,
    /// Whether a download or an install is running.
    #[serde(default)]
    pub running: bool,
    /// An error code, `0` for none.
    #[serde(default)]
    pub error_code: i64,
    /// The text of the error.
    #[serde(default)]
    pub error_msg: String,
    /// Why the check failed, when `current_stat` is `0`.
    #[serde(default)]
    pub request_failed_reason: i64,
    /// Whether the installed build is a beta.
    #[serde(default)]
    pub beta: bool,
    /// When the update server was asked last, Unix seconds.
    #[serde(default)]
    pub prev_check_timestamp: i64,
    /// The update found, if any.
    #[serde(default)]
    pub remote_base: Option<RemoteFirmware>,
}

/// `firmware/version/is_new`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionFlag {
    /// Whether a newer firmware exists.
    #[serde(default)]
    pub has_version: bool,
    /// The installed version number.
    #[serde(default)]
    pub current_version: i64,
    /// The newest version number.
    #[serde(default)]
    pub publish_version: i64,
}

/// `firmware/config`: how the NAS looks for updates.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirmwareConfig {
    /// `0` tell me, `1` install important updates, `2` install any update.
    #[serde(default)]
    pub auto_update: i64,
    /// Whether the check interval is set by hand.
    #[serde(default)]
    pub custom_interval_time: bool,
    /// The interval, in `unit`s.
    #[serde(default)]
    pub interval_time: i64,
    /// `1` hours, `2` days, `3` months.
    #[serde(default)]
    pub unit: i64,
    /// The last check, Unix seconds.
    #[serde(default)]
    pub pre_check_time: i64,
}

/// `firmware/manual/status`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManualStatus {
    /// `install` or `complete`.
    #[serde(default)]
    pub current_stage: String,
    /// Progress in percent.
    #[serde(default)]
    pub process: i64,
    /// An error code, `0` for none.
    #[serde(default)]
    pub error_code: i64,
}

/// The metadata of a configuration backup.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupInfo {
    /// Name of the NAS that made it.
    #[serde(default, rename = "serverName")]
    pub server_name: String,
    /// When, Unix seconds, `0` for never.
    #[serde(default, rename = "lastBackupTime")]
    pub last_backup_time: i64,
    /// The UGREEN account.
    #[serde(default)]
    pub account: String,
    /// The system version it was made on.
    #[serde(default, rename = "systemVersion")]
    pub system_version: String,
    /// The serial number of the NAS.
    #[serde(default)]
    pub sn: String,
}

/// Progress of a backup or a restore.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskProgress {
    /// `canBackup`, `proccessing` (sic) or `failed`.
    #[serde(default)]
    pub status: String,
    /// Work to do.
    #[serde(default)]
    pub total: i64,
    /// Work done.
    #[serde(default)]
    pub completed: i64,
}

/// A service and the ports it listens on, from `sysinfo/collect/getInfo`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServicePorts {
    /// Name.
    #[serde(default)]
    pub name: String,
    /// What it is.
    #[serde(default)]
    pub description: String,
    /// TCP ports.
    #[serde(default)]
    pub ports: String,
    /// UDP ports.
    #[serde(default, rename = "udpPorts")]
    pub udp_ports: String,
    /// `1` a system service, `2` an application.
    #[serde(default, rename = "type")]
    pub kind: i64,
}

/// A list wrapped in `{result: [...]}`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ServiceList {
    /// The services.
    #[serde(default)]
    pub result: Vec<ServicePorts>,
}

/// `getThumbStatus`: the thumbnail service.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThumbStatus {
    /// `1` generating, `2` complete, `3` paused, `4` failed, `5` scanning.
    #[serde(default)]
    pub status: i64,
    /// Progress in percent.
    #[serde(default)]
    pub progress: i64,
    /// Done.
    #[serde(default)]
    pub done: i64,
    /// To do in all.
    #[serde(default)]
    pub total: i64,
    /// Seconds until a pause ends.
    #[serde(default)]
    pub count_down_time: i64,
    /// The text of an error.
    #[serde(default)]
    pub error_string: String,
}

/// A folder range for thumbnails.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PregenRange {
    /// Id.
    #[serde(default)]
    pub id: i64,
    /// Name.
    #[serde(default)]
    pub title: String,
    /// `1` can be chosen.
    #[serde(default)]
    pub status: i64,
    /// The folders.
    #[serde(default)]
    pub dirs: Vec<String>,
}

/// `getPregenList`.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PregenList {
    /// Quality of image thumbnails.
    #[serde(default)]
    pub image_quality: i64,
    /// Quality of video thumbnails.
    #[serde(default)]
    pub video_quality: i64,
    /// The ranges.
    #[serde(default, deserialize_with = "super::common::null_vec_as_empty")]
    pub list: Vec<PregenRange>,
}

/// Anything the about pages answer, kept as it comes.
pub type Raw = Value;
