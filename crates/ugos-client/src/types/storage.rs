//! Types for the Storage Manager: pools, volumes and disks.
//!
//! Status fields are numeric codes that UGOS does not name in the API. `0` is
//! what a healthy pool and volume show; the other values were not seen.

use serde::{Deserialize, Serialize};

use super::common::null_vec_as_empty;

/// A disk that belongs to a pool.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PoolDisk {
    /// Device path, e.g. `/dev/nvme0n1`.
    #[serde(default)]
    pub dev_name: String,
    /// Name the web UI shows, e.g. `M.2 Hard Drive 1`.
    #[serde(default)]
    pub label: String,
    /// Usable size in bytes.
    #[serde(default)]
    pub size: i64,
    /// Slot, e.g. `nvme1`.
    #[serde(default)]
    pub slot: String,
    /// Status code, `0` for a healthy member.
    #[serde(default)]
    pub status: i64,
}

/// When something runs on a schedule.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Schedule {
    /// First run as text.
    #[serde(default)]
    pub first_run_datetime: String,
    /// Repeat every `interval` units.
    #[serde(default)]
    pub interval: i64,
    /// Code of the unit of `interval`; the web UI names it, the API does not.
    #[serde(default)]
    pub interval_unit: i64,
}

/// Trim (discard) setting of a pool.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Trim {
    /// Whether it runs on a schedule.
    #[serde(default)]
    pub enable: bool,
    /// Whether the pool's disks support it.
    #[serde(default)]
    pub is_support: bool,
    /// When it runs next.
    #[serde(default)]
    pub next_start_time: String,
    /// The schedule.
    #[serde(default)]
    pub scheduled: Schedule,
}

/// Data scrubbing of a pool.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scrub {
    /// Whether it runs on a schedule.
    #[serde(default)]
    pub is_cron: bool,
    /// When it last ran, `--` for never.
    #[serde(default)]
    pub last_time: String,
    /// When it runs next, `--` for never.
    #[serde(default)]
    pub next_start_time: String,
    /// Progress in percent while it runs.
    #[serde(default)]
    pub process: i64,
    /// Status code.
    #[serde(default)]
    pub status: i64,
}

/// A storage pool, the RAID set that volumes live on.
// Several `support_*` switches, one field each, as UGOS reports them.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Pool {
    /// Name, e.g. `pool1`.
    #[serde(default)]
    pub name: String,
    /// Name the web UI shows, e.g. `Storage Pool 1`.
    #[serde(default)]
    pub label: String,
    /// RAID level, e.g. `raid5`.
    #[serde(default)]
    pub level: String,
    /// Status code, `0` when healthy.
    #[serde(default)]
    pub status: i64,
    /// Capacity in bytes.
    #[serde(default)]
    pub total: i64,
    /// Used in bytes.
    #[serde(default)]
    pub used: i64,
    /// Free in bytes.
    #[serde(default)]
    pub free: i64,
    /// Number of disks.
    #[serde(default)]
    pub total_disk_num: i64,
    /// Progress in percent of a rebuild or sync, `0` when idle.
    #[serde(default)]
    pub progress: i64,
    /// Seconds a running job still needs, `-1` when none runs.
    #[serde(default)]
    pub remaining_time: i64,
    /// The member disks.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub disks: Vec<PoolDisk>,
    /// Hot spares, whatever shape UGOS gives them.
    #[serde(default)]
    pub spare: Option<serde_json::Value>,
    /// Trim setting.
    #[serde(default)]
    pub trim: Option<Trim>,
    /// Scrub setting and state.
    #[serde(default)]
    pub data_scrub: Option<Scrub>,
    /// Whether the pool can be deleted.
    #[serde(default)]
    pub support_delete: bool,
    /// Whether it can be extended.
    #[serde(default)]
    pub support_extend: bool,
    /// Whether it can take a hot spare.
    #[serde(default)]
    pub support_hot_spare: bool,
    /// Whether it can take an SSD cache.
    #[serde(default)]
    pub support_ssd_cache: bool,
}

/// A volume, the file system on a pool.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StorageVolume {
    /// Name, e.g. `volume1`.
    #[serde(default)]
    pub name: String,
    /// Name the web UI shows, e.g. `Volume 1`.
    #[serde(default)]
    pub label: String,
    /// The pool it lives on.
    #[serde(default)]
    pub poolname: String,
    /// Device path of the logical volume.
    #[serde(default)]
    pub device: String,
    /// File system, e.g. `btrfs`.
    #[serde(default)]
    pub filesystem: String,
    /// Mount path, e.g. `/volume1`.
    #[serde(default)]
    pub mntpath: String,
    /// Every path it is mounted at.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub mntpaths: Vec<String>,
    /// Capacity in bytes.
    #[serde(default)]
    pub total: i64,
    /// Used in bytes.
    #[serde(default)]
    pub used: i64,
    /// Available in bytes.
    #[serde(default)]
    pub available: i64,
    /// Status code, `0` when healthy.
    #[serde(default)]
    pub status: i64,
    /// Health code, `0` when healthy.
    #[serde(default)]
    pub health: i64,
    /// Whether it holds the home folders.
    #[serde(default)]
    pub is_home: bool,
    /// Whether it is ready to use.
    #[serde(default)]
    pub ready_to_use: bool,
    /// Free space, in percent, below which UGOS warns.
    #[serde(default)]
    pub space_alert_percent: i64,
    /// Free text the owner gave it.
    #[serde(default)]
    pub describe: String,
}

/// A disk, as the Storage Manager lists it.
// Flags as UGOS reports them, one field each.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Disk {
    /// Short name, e.g. `nvme0n1`.
    #[serde(default)]
    pub name: String,
    /// Device path, e.g. `/dev/nvme0n1`.
    #[serde(default)]
    pub dev_name: String,
    /// Name the web UI shows.
    #[serde(default)]
    pub label: String,
    /// Slot, e.g. `nvme1`.
    #[serde(default)]
    pub slot: String,
    /// Maker.
    #[serde(default)]
    pub brand: String,
    /// Model.
    #[serde(default)]
    pub model: String,
    /// Serial number.
    #[serde(default)]
    pub serial: String,
    /// Size in bytes.
    #[serde(default)]
    pub size: i64,
    /// `m2`, `sata`, ...
    #[serde(default)]
    pub interface_type: String,
    /// Status code, `1` for a disk in use.
    #[serde(default)]
    pub status: i64,
    /// Temperature in °C.
    #[serde(default)]
    pub temperature: i64,
    /// Hours it has been on.
    #[serde(default)]
    pub power_on_hours: i64,
    /// The pool it belongs to, empty for none.
    #[serde(default)]
    pub used_for: String,
    /// Whether it is active.
    #[serde(default)]
    pub activate: bool,
    /// Whether it sleeps.
    #[serde(default)]
    pub is_standby: bool,
    /// Whether it reports SMART data.
    #[serde(default)]
    pub is_support_smart: bool,
}

/// Fixed facts about a disk.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiskAttribute {
    /// Maker.
    #[serde(default)]
    pub brand: String,
    /// Model.
    #[serde(default)]
    pub model: String,
    /// Serial number.
    #[serde(default)]
    pub serial_number: String,
    /// Firmware version.
    #[serde(default)]
    pub firmware: String,
    /// Interface, e.g. `M.2`.
    #[serde(default)]
    pub interface: String,
    /// Protocol, e.g. `NVMe`.
    #[serde(default)]
    pub standard_agreement: String,
    /// Size in bytes.
    #[serde(default)]
    pub size: i64,
    /// Slot.
    #[serde(default)]
    pub slot: String,
    /// Times it was powered on.
    #[serde(default)]
    pub power_on_count: i64,
    /// Hours it has been on.
    #[serde(default)]
    pub power_on_hours: i64,
    /// Rotation speed, `0` for flash.
    #[serde(default)]
    pub rotate: i64,
}

/// One reading of a disk's SMART report.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SmartItem {
    /// Name of the attribute, e.g. `temperature`.
    #[serde(default)]
    pub name: String,
    /// Label the web UI shows.
    #[serde(default)]
    pub label: String,
    /// The value, a number or a list.
    #[serde(default)]
    pub value: serde_json::Value,
}

/// The last or next SMART test of a disk.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SmartRun {
    /// When it last ran, `0` for never.
    #[serde(default)]
    pub last_time: i64,
    /// When it runs next.
    #[serde(default)]
    pub next_time: i64,
    /// Status code of the last run, `1` when it passed.
    #[serde(default)]
    pub status: i64,
}

/// A disk's SMART state and report.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Smart {
    /// Whether a test runs now.
    #[serde(default)]
    pub is_testing: bool,
    /// Progress in percent of a running test.
    #[serde(default)]
    pub progress: i64,
    /// Minutes left of a running test.
    #[serde(default)]
    pub remain_time: i64,
    /// Minutes a short test takes.
    #[serde(default)]
    pub short_test_cost: i64,
    /// Minutes a long test takes.
    #[serde(default)]
    pub long_test_cost: i64,
    /// Last and next run.
    #[serde(default)]
    pub last: SmartRun,
    /// The readings.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub report: Vec<SmartItem>,
}

/// One run of a disk test, from the history.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestRecord {
    /// When it ran.
    #[serde(default)]
    pub time: i64,
    /// Result code, `1` for a pass.
    #[serde(default)]
    pub status: i64,
    /// `0` short test, `1` long test, as the history shows them.
    #[serde(default)]
    pub smart_type: i64,
    /// Kind of record.
    #[serde(default)]
    pub r#type: i64,
    /// Error code, empty when none.
    #[serde(default)]
    pub err_code: String,
}

/// A scheduled disk test plan.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TestPlan {
    /// Plan id.
    #[serde(default)]
    pub cron_id: i64,
    /// Plan name.
    #[serde(default)]
    pub name: String,
    /// Whether UGOS made it.
    #[serde(default)]
    pub is_default: bool,
    /// When it runs next.
    #[serde(default)]
    pub next_time: String,
    /// The disks it covers.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub disks: Vec<String>,
    /// When it repeats.
    #[serde(default)]
    pub schedule: Schedule,
}
