//! Rows for pools, volumes and disks.

use serde::Serialize;
use tabled::Tabled;
use ugos_client::types::storage::{
    Disk, DiskAttribute, Pool, Smart, StorageVolume, TestPlan, TestRecord,
};

use super::{FieldRow, field, format_unix, yes_no};

/// Bytes with a unit up to TiB. The shared formatter stops at GiB.
#[allow(clippy::cast_precision_loss)]
pub fn size(bytes: i64) -> String {
    let b = bytes as f64;
    if b >= 1_099_511_627_776.0 {
        format!("{:.2} TiB", b / 1_099_511_627_776.0)
    } else if b >= 1_073_741_824.0 {
        format!("{:.1} GiB", b / 1_073_741_824.0)
    } else if b >= 1_048_576.0 {
        format!("{:.1} MiB", b / 1_048_576.0)
    } else {
        format!("{bytes} B")
    }
}

/// `used of total (NN %)`.
#[allow(clippy::cast_precision_loss)]
fn usage(used: i64, total: i64) -> String {
    if total <= 0 {
        return size(used);
    }
    let percent = used as f64 * 100.0 / total as f64;
    format!("{} of {} ({percent:.0} %)", size(used), size(total))
}

/// Table row for a pool.
#[derive(Serialize)]
pub struct PoolRow {
    pub name: String,
    pub label: String,
    pub level: String,
    pub status: String,
    pub used: String,
    pub disks: String,
}

table_row! {
  PoolRow {
    name => "Pool",
    label => "Label",
    level => "RAID",
    status => "Status",
    used => "Used",
    disks => "Disks",
  }
}

impl From<&Pool> for PoolRow {
    fn from(pool: &Pool) -> Self {
        Self {
            name: pool.name.clone(),
            label: pool.label.clone(),
            level: pool.level.clone(),
            status: pool.status.to_string(),
            used: usage(pool.used, pool.total),
            disks: pool.total_disk_num.to_string(),
        }
    }
}

/// The detail view of a pool.
pub fn pool_rows(pool: &Pool) -> Vec<FieldRow> {
    let mut rows = vec![
        field("Pool", pool.name.clone()),
        field("Label", pool.label.clone()),
        field("RAID level", pool.level.clone()),
        field("Status code", pool.status.to_string()),
        field("Used", usage(pool.used, pool.total)),
        field("Free", size(pool.free)),
        field("Disks", pool.total_disk_num.to_string()),
    ];
    for disk in &pool.disks {
        rows.push(field(
            &format!("  {}", disk.slot),
            format!(
                "{} ({}, status {})",
                disk.dev_name,
                size(disk.size),
                disk.status
            ),
        ));
    }
    if pool.progress > 0 || pool.remaining_time >= 0 {
        rows.push(field("Job progress", format!("{} %", pool.progress)));
    }
    if let Some(scrub) = &pool.data_scrub {
        rows.push(field("Scrub, last run", scrub.last_time.clone()));
        rows.push(field("Scrub, next run", scrub.next_start_time.clone()));
        rows.push(field("Scrub on a schedule", yes_no(scrub.is_cron)));
    }
    if let Some(trim) = &pool.trim {
        rows.push(field("Trim", yes_no(trim.enable)));
        rows.push(field("Trim, next run", trim.next_start_time.clone()));
    }
    rows.push(field("Can be extended", yes_no(pool.support_extend)));
    rows.push(field(
        "Can take a hot spare",
        yes_no(pool.support_hot_spare),
    ));
    rows.push(field(
        "Can take an SSD cache",
        yes_no(pool.support_ssd_cache),
    ));
    rows
}

/// Table row for a volume.
#[derive(Serialize)]
pub struct VolumeRow {
    pub name: String,
    pub label: String,
    pub pool: String,
    pub filesystem: String,
    pub used: String,
    pub path: String,
    pub status: String,
}

table_row! {
  VolumeRow {
    name => "Volume",
    label => "Label",
    pool => "Pool",
    filesystem => "Filesystem",
    used => "Used",
    path => "Mounted at",
    status => "Status",
  }
}

impl From<&StorageVolume> for VolumeRow {
    fn from(volume: &StorageVolume) -> Self {
        Self {
            name: volume.name.clone(),
            label: volume.label.clone(),
            pool: volume.poolname.clone(),
            filesystem: volume.filesystem.clone(),
            used: usage(volume.used, volume.total),
            path: volume.mntpath.clone(),
            status: volume.status.to_string(),
        }
    }
}

/// The detail view of a volume.
pub fn volume_rows(volume: &StorageVolume) -> Vec<FieldRow> {
    vec![
        field("Volume", volume.name.clone()),
        field("Label", volume.label.clone()),
        field("Pool", volume.poolname.clone()),
        field("Filesystem", volume.filesystem.clone()),
        field("Used", usage(volume.used, volume.total)),
        field("Available", size(volume.available)),
        field("Mounted at", volume.mntpaths.join(", ")),
        field("Device", volume.device.clone()),
        field("Status code", volume.status.to_string()),
        field("Health code", volume.health.to_string()),
        field("Holds the home folders", yes_no(volume.is_home)),
        field("Ready to use", yes_no(volume.ready_to_use)),
        field(
            "Warns below",
            format!("{} % free", volume.space_alert_percent),
        ),
    ]
}

/// Table row for a disk.
#[derive(Serialize)]
pub struct DiskRow {
    pub name: String,
    pub slot: String,
    pub model: String,
    pub size: String,
    pub temperature: String,
    pub hours: String,
    pub used_for: String,
}

table_row! {
  DiskRow {
    name => "Disk",
    slot => "Slot",
    model => "Model",
    size => "Size",
    temperature => "Temp",
    hours => "Hours on",
    used_for => "Used for",
  }
}

impl From<&Disk> for DiskRow {
    fn from(disk: &Disk) -> Self {
        Self {
            name: disk.name.clone(),
            slot: disk.slot.clone(),
            model: disk.model.clone(),
            size: size(disk.size),
            temperature: format!("{} °C", disk.temperature),
            hours: disk.power_on_hours.to_string(),
            used_for: disk.used_for.clone(),
        }
    }
}

/// The detail view of a disk: the list entry and its fixed facts.
pub fn disk_rows(disk: &Disk, attribute: &DiskAttribute) -> Vec<FieldRow> {
    vec![
        field("Disk", disk.dev_name.clone()),
        field("Label", disk.label.clone()),
        field("Slot", disk.slot.clone()),
        field("Brand", attribute.brand.clone()),
        field("Model", attribute.model.clone()),
        field("Serial", attribute.serial_number.clone()),
        field("Firmware", attribute.firmware.clone()),
        field(
            "Interface",
            format!("{} {}", attribute.interface, attribute.standard_agreement),
        ),
        field("Size", size(attribute.size)),
        field("Temperature", format!("{} °C", disk.temperature)),
        field("Hours on", attribute.power_on_hours.to_string()),
        field("Power cycles", attribute.power_on_count.to_string()),
        field("Used for", disk.used_for.clone()),
        field("Active", yes_no(disk.activate)),
        field("Standby", yes_no(disk.is_standby)),
        field("Reports SMART", yes_no(disk.is_support_smart)),
    ]
}

/// The SMART state and report of a disk.
pub fn smart_rows(smart: &Smart) -> Vec<FieldRow> {
    let mut rows = vec![
        field("Test running", yes_no(smart.is_testing)),
        field("Last test", format_unix(smart.last.last_time)),
        field("Last result code", smart.last.status.to_string()),
        field("Next test", format_unix(smart.last.next_time)),
        field("Short test takes", format!("{} min", smart.short_test_cost)),
        field("Long test takes", format!("{} min", smart.long_test_cost)),
    ];
    if smart.is_testing {
        rows.push(field("Progress", format!("{} %", smart.progress)));
    }
    for item in &smart.report {
        rows.push(field(&item.label, item.value.to_string()));
    }
    rows
}

/// Table row for one run of a disk test.
#[derive(Serialize)]
pub struct HistoryRow {
    pub time: String,
    pub kind: String,
    pub result: String,
    pub error: String,
}

table_row! {
  HistoryRow {
    time => "Time",
    kind => "Test",
    result => "Result code",
    error => "Error",
  }
}

impl From<&TestRecord> for HistoryRow {
    fn from(record: &TestRecord) -> Self {
        Self {
            time: format_unix(record.time),
            kind: match record.smart_type {
                0 => "short".into(),
                1 => "long".into(),
                other => format!("type {other}"),
            },
            result: record.status.to_string(),
            error: record.err_code.clone(),
        }
    }
}

/// Table row for a test plan.
#[derive(Serialize)]
pub struct PlanRow {
    pub id: String,
    pub name: String,
    pub next: String,
    pub repeats: String,
    pub disks: String,
}

table_row! {
  PlanRow {
    id => "ID",
    name => "Plan",
    next => "Next run",
    repeats => "Repeats",
    disks => "Disks",
  }
}

impl From<&TestPlan> for PlanRow {
    fn from(plan: &TestPlan) -> Self {
        Self {
            id: plan.cron_id.to_string(),
            name: plan.name.clone(),
            next: plan.next_time.clone(),
            repeats: format!(
                "every {} (unit code {})",
                plan.schedule.interval, plan.schedule.interval_unit
            ),
            disks: plan.disks.join(", "),
        }
    }
}
