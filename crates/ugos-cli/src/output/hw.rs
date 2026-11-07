//! Rows for the hardware, power, time and terminal settings.

use serde::Serialize;
use tabled::Tabled;
use ugos_client::types::hw::{
    Buzzer, Cpu, Fan, Finder, Gpu, Language, Led, LedScreen, Power, Terminal, TimeConfig, Ups, Zone,
};

use super::{FieldRow, field, format_unix, yes_no};

fn flag(value: i64) -> String {
    yes_no(value != 0)
}

/// The buzzer.
pub fn buzzer_rows(b: &Buzzer) -> Vec<FieldRow> {
    let mut rows = vec![
        field("Fan fails", flag(b.fan_fault)),
        field("Cache disk fails", flag(b.cache_exception)),
        field("NAS shuts down", flag(b.system_shutdown)),
        field("Disk too hot", flag(b.temperature_too_high)),
    ];
    if let Some(startup) = b.system_startup {
        rows.insert(2, field("NAS starts", flag(startup)));
    }
    rows
}

/// The CPU mode.
pub fn cpu_rows(c: &Cpu) -> Vec<FieldRow> {
    let mode = match c.frequency {
        0 => "high performance",
        1 => "balanced",
        2 => "power saving",
        _ => "unknown",
    };
    vec![
        field("Setting available", yes_no(c.is_support == 1)),
        field("Mode", mode),
    ]
}

fn curve(table: &[[i64; 2]]) -> String {
    table
        .iter()
        .map(|p| format!("{}°C:{}%", p[0], p[1]))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The fans.
pub fn fan_rows(f: &Fan) -> Vec<FieldRow> {
    let mode = match f.mode {
        1 => "quiet",
        2 => "automatic",
        3 => "full speed",
        4 => "custom curve",
        _ => "unknown",
    };
    vec![
        field("Mode", mode),
        field("Device fan", yes_no(f.has_sys_fan)),
        field("CPU fan", yes_no(f.has_cpu_fan)),
        field("Custom curve possible", yes_no(f.support_custom)),
        field("Device fan curve", curve(&f.sys_fan_table)),
        field("CPU fan curve", curve(&f.cpu_fan_table)),
    ]
}

/// The LEDs.
pub fn led_rows(l: &Led) -> Vec<FieldRow> {
    let mut rows = vec![
        field("LEDs", if l.status == 1 { "on" } else { "off" }),
        field("Model", l.model.clone()),
    ];
    for group in &l.info {
        let days = group
            .days
            .iter()
            .map(i64::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let bright = group.time_config.first().map_or(0, |t| t.bright);
        rows.push(field(&format!("  days {days}"), format!("{bright} %")));
    }
    rows
}

/// The small display.
pub fn screen_rows(s: &LedScreen) -> Vec<FieldRow> {
    s.iter()
        .map(|(key, value)| {
            field(
                key,
                value
                    .as_str()
                    .map_or_else(|| value.to_string(), str::to_owned),
            )
        })
        .collect()
}

fn sleep(flag_on: bool, time: i64, unit: &str) -> String {
    if !flag_on {
        return "never".into();
    }
    format!("after {time} {}", if unit == "H" { "h" } else { "min" })
}

fn days(list: &[i64]) -> String {
    const NAMES: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    list.iter()
        .filter_map(|d| usize::try_from(d - 1).ok().and_then(|i| NAMES.get(i)))
        .copied()
        .collect::<Vec<_>>()
        .join(",")
}

/// Power settings and schedule.
pub fn power_rows(p: &Power) -> Vec<FieldRow> {
    let mut rows = vec![
        field("Power on after a power loss", yes_no(p.power_boot)),
        field("Wake on LAN", yes_no(p.wake_on)),
        field(
            "Disks sleep",
            sleep(p.hard_drive_flag, p.hard_drive_time, &p.hard_drive_unit),
        ),
        field(
            "USB disks sleep",
            sleep(
                p.usb_hard_drive_flag,
                p.usb_hard_drive_time,
                &p.usb_hard_drive_unit,
            ),
        ),
        field(
            "Woken by",
            if p.hard_drive_strategy == 2 {
                "read or write"
            } else {
                "login"
            },
        ),
        field("Log the sleeping", yes_no(p.enable_hard_drive_sleep_log)),
        field("Power schedule", yes_no(p.enable_scheduled_power)),
    ];
    for (i, entry) in p.time_switches.iter().enumerate() {
        let what = if entry.action == "open" {
            "power on"
        } else {
            "power off"
        };
        rows.push(field(
            &format!("  entry {}", i + 1),
            format!("{} {} {what}", days(&entry.frequency), entry.time),
        ));
    }
    rows
}

/// The UPS.
pub fn ups_rows(u: &Ups) -> Vec<FieldRow> {
    let kind = match u.kind {
        0 => "USB",
        1 => "SNMP",
        2 => "network UPS (NUT)",
        _ => "none",
    };
    let wait = match (u.right_now, u.standby_time) {
        (true, _) => "at once".to_owned(),
        (false, t) if t > 0 => format!(
            "after {t} {}",
            if u.standby_time_unit == 2 { "h" } else { "min" }
        ),
        _ => "until the battery is low".to_owned(),
    };
    let info = |key: &str| {
        u.ups_info
            .get(key)
            .map(|v| v.as_str().map_or_else(|| v.to_string(), str::to_owned))
            .unwrap_or_default()
    };
    vec![
        field("Kind", kind),
        field("Monitoring", yes_no(u.status)),
        field(
            "On battery",
            if u.protect_type == 1 {
                "shut down"
            } else {
                "standby"
            },
        ),
        field("When", wait),
        field("NUT server address", u.ups_master_ip.clone()),
        field("SNMP address", u.snmp_ip.clone()),
        field("Model", info("product_mode")),
        field("Battery", info("battery_capacity")),
    ]
}

/// Table row for a GPU.
#[derive(Serialize)]
pub struct GpuRow {
    pub id: String,
    pub name: String,
    pub link: String,
    pub state: String,
    pub mode: String,
}

table_row! {
  GpuRow {
    id => "ID",
    name => "GPU",
    link => "Link",
    state => "State",
    mode => "Mode",
  }
}

impl From<&Gpu> for GpuRow {
    fn from(g: &Gpu) -> Self {
        Self {
            id: g.gpu_id.clone(),
            name: g.gpu_name.clone(),
            link: match g.gpu_interface {
                1 => "OCuLink",
                2 => "Thunderbolt 4",
                3 => "PCIe",
                _ => "-",
            }
            .into(),
            state: match g.gpu_state {
                0 => "no driver",
                1 => "connected",
                2 => "passthrough",
                _ => "abnormal",
            }
            .into(),
            mode: if g.gpu_mode == 2 {
                "passthrough"
            } else {
                "NAS"
            }
            .into(),
        }
    }
}

/// Time and how the clock is kept.
pub fn time_rows(t: &TimeConfig) -> Vec<FieldRow> {
    vec![
        field("Time", t.time_show.clone()),
        field("Time (UTC)", format_unix(t.time)),
        field("Time zone", t.timezone.clone()),
        field(
            "Clock",
            if t.time_set_type == 0 {
                format!("from {}", t.time_server)
            } else {
                "set by hand".into()
            },
        ),
        field("Time format", format!("{} h", t.time_format)),
        field("Date format", t.date_format.clone()),
    ]
}

/// Table row for a time zone.
#[derive(Serialize)]
pub struct ZoneRow {
    pub name: String,
    pub label: String,
}

table_row! {
  ZoneRow {
    name => "Zone",
    label => "Label",
  }
}

impl From<&Zone> for ZoneRow {
    fn from(z: &Zone) -> Self {
        Self {
            name: z.name.clone(),
            label: z.uistring.clone(),
        }
    }
}

/// Table row for a language.
#[derive(Serialize)]
pub struct LanguageRow {
    pub key: String,
    pub name: String,
}

table_row! {
  LanguageRow {
    key => "Key",
    name => "Name",
  }
}

impl From<&Language> for LanguageRow {
    fn from(l: &Language) -> Self {
        Self {
            key: l.key.clone(),
            name: l.name.clone(),
        }
    }
}

/// SSH and Telnet.
pub fn terminal_rows(t: &Terminal) -> Vec<FieldRow> {
    let on = |v: i64| if v == 1 { "on" } else { "off" };
    let lan = |v: i64| {
        if v == 1 {
            "local network only"
        } else {
            "any network"
        }
    };
    let duration = match t.ssh.ex_time_duration {
        1 => "10 minutes",
        2 => "30 minutes",
        3 => "1 hour",
        4 => "2 hours",
        5 => "3 hours",
        6 => "6 hours",
        _ => "until switched off",
    };
    let mut rows = vec![
        field("SSH", on(t.ssh.enable)),
        field("SSH port", t.ssh.port.to_string()),
        field("SSH reachable from", lan(t.ssh.permit_lan_access_only)),
        field("SFTP", on(t.ssh.use_sftp)),
        field(
            "SSH encryption",
            if t.ssh.encrypt_level >= 3 {
                "high"
            } else {
                "low"
            },
        ),
        field("SSH stays on", duration),
        field("Telnet", on(t.telnet.enable)),
        field("Telnet port", t.telnet.port.to_string()),
        field(
            "Telnet reachable from",
            lan(t.telnet.permit_lan_access_only),
        ),
    ];
    if t.ssh.ex_time_stamp > 0 {
        rows.push(field("SSH closes at", format_unix(t.ssh.ex_time_stamp)));
    }
    rows
}

/// The finder switches.
pub fn finder_rows(f: &Finder) -> Vec<FieldRow> {
    vec![
        field("Improvement plan", flag(f.improvement_enable)),
        field("Found from the web", flag(f.web_find)),
    ]
}
