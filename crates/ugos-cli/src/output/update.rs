//! Rows for firmware, backup, the about pages and the thumbnail service.

use serde::Serialize;
use serde_json::Value;
use tabled::Tabled;
use ugos_client::ops::update::backup::RestoreService;
use ugos_client::types::update::{
    BackupInfo, FirmwareConfig, FirmwareStatus, ManualStatus, PregenRange, ServicePorts,
    TaskProgress, ThumbStatus, VersionFlag,
};

use super::{FieldRow, field, format_unix, yes_no};

const fn stat(code: i64) -> &'static str {
    match code {
        0 => "check failed",
        1 => "up to date",
        2 => "update available",
        3 => "checking",
        4 => "manual upload running",
        _ => "unknown",
    }
}

/// The firmware state.
pub fn firmware_rows(s: &FirmwareStatus, flag: &VersionFlag) -> Vec<FieldRow> {
    let mut rows = vec![
        field("Installed", s.current_version.clone()),
        field("State", stat(s.current_stat)),
        field("Newer firmware exists", yes_no(flag.has_version)),
        field("Last check", format_unix(s.prev_check_timestamp)),
    ];
    if let Some(r) = &s.remote_base {
        rows.push(field("Found", r.version.clone()));
        rows.push(field("Important", yes_no(r.version_type == 1)));
        rows.push(field("Beta", yes_no(r.beta)));
        rows.push(field("Size", format!("{} MB", r.bytes / 1_048_576)));
        rows.push(field("Released", format_unix(r.pubtime / 1000)));
        rows.push(field("Changes", r.desc.replace('\n', " ")));
    }
    if s.running || s.process > 0 {
        rows.push(field(
            "Stage",
            format!("{} {}%", s.current_stage, s.process),
        ));
    }
    if s.error_code != 0 {
        rows.push(field("Error", format!("{} {}", s.error_code, s.error_msg)));
    }
    rows
}

/// How the NAS looks for updates.
pub fn firmware_config_rows(c: &FirmwareConfig) -> Vec<FieldRow> {
    let auto = match c.auto_update {
        0 => "tell me",
        1 => "install important updates",
        _ => "install any update",
    };
    let unit = match c.unit {
        2 => "days",
        3 => "months",
        _ => "hours",
    };
    vec![
        field("When found", auto),
        field(
            "Check every",
            if c.custom_interval_time {
                format!("{} {unit}", c.interval_time)
            } else {
                "UGOS decides".to_owned()
            },
        ),
        field("Last check", format_unix(c.pre_check_time)),
    ]
}

/// Progress of an install from a file.
pub fn manual_rows(s: &ManualStatus) -> Vec<FieldRow> {
    vec![
        field("Stage", s.current_stage.clone()),
        field("Progress", format!("{}%", s.process.max(0))),
        field("Error code", s.error_code.to_string()),
    ]
}

/// A cloud backup.
pub fn backup_rows(info: &BackupInfo, scheduled: bool) -> Vec<FieldRow> {
    let mut rows = vec![field("Scheduled", yes_no(scheduled))];
    if info.last_backup_time == 0 {
        rows.push(field("Latest backup", "none"));
        return rows;
    }
    rows.push(field("Latest backup", format_unix(info.last_backup_time)));
    rows.push(field("Account", info.account.clone()));
    rows.push(field("NAS", info.server_name.clone()));
    rows.push(field("Version", info.system_version.clone()));
    rows
}

/// Progress of a task.
pub fn progress_rows(p: Option<&TaskProgress>) -> Vec<FieldRow> {
    p.map_or_else(
        || vec![field("Task", "none is running")],
        |p| {
            vec![
                field("State", p.status.clone()),
                field("Done", format!("{} of {}", p.completed, p.total)),
            ]
        },
    )
}

/// A restorable service.
#[derive(Serialize)]
pub struct RestoreRow {
    pub path: String,
    pub names: String,
}

table_row! {
  RestoreRow {
    path => "Service",
    names => "Names",
  }
}

impl From<&RestoreService> for RestoreRow {
    fn from(s: &RestoreService) -> Self {
        Self {
            path: s.path.clone(),
            names: s.names.join(", "),
        }
    }
}

/// The about page, from what UGOS answers.
pub fn about_rows(machine: &Value, repair_end: i64) -> Vec<FieldRow> {
    let common = machine.get("common").unwrap_or(&Value::Null);
    let text = |key: &str| match common.get(key) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Null) | None => "-".into(),
        Some(v) => v.to_string(),
    };
    let mut rows = vec![
        field("Name", text("nas_name")),
        field("Model", text("model")),
        field("Serial", text("serial")),
        field("Version", text("system_version")),
        field("Beta", text("beta")),
        field("Last started", text("last_turn_on_time")),
        field("Run time (s)", text("run_time")),
        field("Warranty ends", format_unix(repair_end / 1000)),
    ];
    if let Some(cpu) = machine["hardware"]["cpu"].get(0) {
        rows.push(field(
            "CPU",
            format!(
                "{} ({} cores, {} threads)",
                cpu["model"].as_str().unwrap_or("-"),
                cpu["core"],
                cpu["thread"]
            ),
        ));
    }
    rows
}

/// Ports of a service.
#[derive(Serialize)]
pub struct PortRow {
    pub name: String,
    pub kind: String,
    pub tcp: String,
    pub udp: String,
}

table_row! {
  PortRow {
    name => "Service",
    kind => "Kind",
    tcp => "TCP",
    udp => "UDP",
  }
}

impl From<&ServicePorts> for PortRow {
    fn from(s: &ServicePorts) -> Self {
        let dash = |t: &str| {
            if t.is_empty() {
                "-".to_owned()
            } else {
                t.to_owned()
            }
        };
        Self {
            name: s.name.clone(),
            kind: if s.kind == 2 { "application" } else { "system" }.into(),
            tcp: dash(&s.ports),
            udp: dash(&s.udp_ports),
        }
    }
}

/// The thumbnail service.
pub fn thumb_rows(t: &ThumbStatus) -> Vec<FieldRow> {
    let state = match t.status {
        1 => "generating",
        2 => "complete",
        3 => "paused",
        4 => "failed",
        5 => "scanning",
        _ => "idle",
    };
    let mut rows = vec![
        field("State", state),
        field(
            "Progress",
            format!("{}% ({} of {})", t.progress, t.done, t.total),
        ),
    ];
    if t.status == 3 {
        rows.push(field("Resumes in", format!("{} s", t.count_down_time)));
    }
    if !t.error_string.is_empty() {
        rows.push(field("Error", t.error_string.clone()));
    }
    rows
}

/// A folder range.
#[derive(Serialize)]
pub struct RangeRow {
    pub id: i64,
    pub title: String,
    pub choosable: String,
    pub dirs: String,
}

table_row! {
  RangeRow {
    id => "Id",
    title => "Range",
    choosable => "Can be chosen",
    dirs => "Folders",
  }
}

impl From<&PregenRange> for RangeRow {
    fn from(r: &PregenRange) -> Self {
        Self {
            id: r.id,
            title: r.title.clone(),
            choosable: yes_no(r.status == 1),
            dirs: r.dirs.join(", "),
        }
    }
}
