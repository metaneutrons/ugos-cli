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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use serde_json::json;
    use ugos_client::ops::update::backup::RestoreService;
    use ugos_client::types::update::{
        BackupInfo, FirmwareConfig, FirmwareStatus, ManualStatus, PregenRange, RemoteFirmware,
        ServicePorts, TaskProgress, ThumbStatus, VersionFlag,
    };

    use super::{
        PortRow, RangeRow, RestoreRow, about_rows, backup_rows, firmware_config_rows,
        firmware_rows, manual_rows, progress_rows, thumb_rows,
    };

    fn value(rows: &[super::FieldRow], name: &str) -> String {
        rows.iter()
            .find(|r| r.field == name)
            .map(|r| r.value.clone())
            .unwrap_or_default()
    }

    #[test]
    fn firmware_is_shown_with_the_update_that_was_found() {
        let status = FirmwareStatus {
            current_version: "1.20".into(),
            current_stat: 2,
            current_stage: "download".into(),
            process: 100,
            error_code: 1500,
            error_msg: "bad".into(),
            remote_base: Some(RemoteFirmware {
                version: "1.21".into(),
                version_type: 1,
                bytes: 3 << 20,
                desc: "a\nb".into(),
                ..RemoteFirmware::default()
            }),
            ..FirmwareStatus::default()
        };
        let rows = firmware_rows(
            &status,
            &VersionFlag {
                has_version: true,
                ..VersionFlag::default()
            },
        );
        assert_eq!(value(&rows, "State"), "update available");
        assert_eq!(value(&rows, "Found"), "1.21");
        assert_eq!(value(&rows, "Important"), "yes");
        assert_eq!(value(&rows, "Size"), "3 MB");
        assert_eq!(value(&rows, "Changes"), "a b");
        assert_eq!(value(&rows, "Error"), "1500 bad");
        let plain = firmware_rows(&FirmwareStatus::default(), &VersionFlag::default());
        assert_eq!(value(&plain, "State"), "check failed");
    }

    #[test]
    fn the_update_settings_are_shown() {
        let auto = FirmwareConfig {
            auto_update: 1,
            custom_interval_time: true,
            interval_time: 6,
            unit: 2,
            ..FirmwareConfig::default()
        };
        let rows = firmware_config_rows(&auto);
        assert_eq!(value(&rows, "When found"), "install important updates");
        assert_eq!(value(&rows, "Check every"), "6 days");
        let default = firmware_config_rows(&FirmwareConfig::default());
        assert_eq!(value(&default, "Check every"), "UGOS decides");
    }

    #[test]
    fn backups_progress_and_manual_installs_are_shown() {
        let none = backup_rows(&BackupInfo::default(), false);
        assert_eq!(value(&none, "Latest backup"), "none");
        let info = BackupInfo {
            last_backup_time: 86_400,
            account: "me".into(),
            ..BackupInfo::default()
        };
        assert_eq!(value(&backup_rows(&info, true), "Account"), "me");
        assert_eq!(value(&progress_rows(None), "Task"), "none is running");
        let p = TaskProgress {
            status: "proccessing".into(),
            total: 4,
            completed: 1,
        };
        assert_eq!(value(&progress_rows(Some(&p)), "Done"), "1 of 4");
        let manual = manual_rows(&ManualStatus {
            current_stage: "install".into(),
            process: -1,
            error_code: 0,
        });
        assert_eq!(value(&manual, "Progress"), "0%");
    }

    #[test]
    fn services_ports_and_ranges_are_shown() {
        let service = RestoreService {
            path: "a/b".into(),
            names: vec!["x".into(), "y".into()],
        };
        assert_eq!(RestoreRow::from(&service).names, "x, y");
        let port = ServicePorts {
            name: "SMB".into(),
            ports: "445".into(),
            kind: 1,
            ..ServicePorts::default()
        };
        let row = PortRow::from(&port);
        assert_eq!(
            (row.tcp.as_str(), row.udp.as_str(), row.kind.as_str()),
            ("445", "-", "system")
        );
        let range = PregenRange {
            id: 2,
            status: 1,
            dirs: vec!["/a".into()],
            ..PregenRange::default()
        };
        assert_eq!(RangeRow::from(&range).choosable, "yes");
    }

    #[test]
    fn the_thumbnail_service_and_the_machine_are_shown() {
        let paused = ThumbStatus {
            status: 3,
            count_down_time: 60,
            error_string: "e".into(),
            ..ThumbStatus::default()
        };
        let rows = thumb_rows(&paused);
        assert_eq!(value(&rows, "State"), "paused");
        assert_eq!(value(&rows, "Resumes in"), "60 s");
        assert_eq!(value(&rows, "Error"), "e");
        let machine = json!({"common": {"nas_name": "n", "beta": false, "run_time": 5},
            "hardware": {"cpu": [{"model": "M", "core": 2, "thread": 4}]}});
        let about = about_rows(&machine, 0);
        assert_eq!(value(&about, "Name"), "n");
        assert_eq!(value(&about, "Beta"), "false");
        assert_eq!(value(&about, "CPU"), "M (2 cores, 4 threads)");
    }
}
