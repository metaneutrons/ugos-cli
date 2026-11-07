//! Types for the hardware, power, time and terminal settings of the NAS.
//!
//! Most of these are written back whole, so the read types keep what they do
//! not model in `extra` and a change sends it back as it was.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::common::null_vec_as_empty;

/// Which events the buzzer sounds for.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Buzzer {
    /// A fan failed.
    #[serde(default)]
    pub fan_fault: i64,
    /// A cache disk failed.
    #[serde(default)]
    pub cache_exception: i64,
    /// The NAS starts. Absent on some models.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_startup: Option<i64>,
    /// The NAS shuts down.
    #[serde(default)]
    pub system_shutdown: i64,
    /// A disk is too hot.
    #[serde(default)]
    pub temperature_too_high: i64,
}

/// The performance mode of the CPU.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cpu {
    /// `1` when the model has the setting.
    #[serde(default, rename = "isSupport")]
    pub is_support: i64,
    /// `0` high performance, `1` balanced, `2` power saving.
    #[serde(default)]
    pub frequency: i64,
}

/// The fan setting.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fan {
    /// `1` quiet, `2` automatic, `3` full speed, `4` custom curve.
    #[serde(default)]
    pub mode: i64,
    /// Whether a device fan can be set.
    #[serde(default)]
    pub has_sys_fan: bool,
    /// Whether a CPU fan can be set.
    #[serde(default)]
    pub has_cpu_fan: bool,
    /// Whether a custom curve is possible.
    #[serde(default)]
    pub support_custom: bool,
    /// Device fan curve as `[temperature, duty]` pairs.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub sys_fan_table: Vec<[i64; 2]>,
    /// CPU fan curve as `[temperature, duty]` pairs.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub cpu_fan_table: Vec<[i64; 2]>,
}

/// A dedicated GPU.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Gpu {
    /// Id, used when changing the mode.
    #[serde(default)]
    pub gpu_id: String,
    /// Name.
    #[serde(default)]
    pub gpu_name: String,
    /// `1` NVIDIA, `2` AMD, `3` Intel.
    #[serde(default)]
    pub gpu_type: i64,
    /// `1` `OCuLink`, `2` Thunderbolt 4, `3` `PCIe`.
    #[serde(default)]
    pub gpu_interface: i64,
    /// `0` no driver, `1` connected, `2` passthrough active, `3` abnormal.
    #[serde(default)]
    pub gpu_state: i64,
    /// `1` NAS mode, `2` VFIO passthrough.
    #[serde(default)]
    pub gpu_mode: i64,
}

/// `hardware/gpu/list`.
#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct GpuList {
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub result: Vec<Gpu>,
}

/// The brightness at certain hours of a group of weekdays.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedHours {
    /// Hours of the day, 0 to 23.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub time: Vec<i64>,
    /// Brightness in percent.
    #[serde(default)]
    pub bright: i64,
}

/// A group of weekdays with their brightness.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedDays {
    /// Weekdays, `1` Monday to `7` Sunday.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub days: Vec<i64>,
    /// Brightness by hour.
    #[serde(default, rename = "timeConfig", deserialize_with = "null_vec_as_empty")]
    pub time_config: Vec<LedHours>,
}

/// The status LEDs.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Led {
    /// `1` on, `2` off.
    #[serde(default)]
    pub status: i64,
    /// The model, for the web UI's choice of form.
    #[serde(default)]
    pub model: String,
    /// The brightness schedule.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub info: Vec<LedDays>,
}

/// The small display of some models, kept whole.
pub type LedScreen = Map<String, Value>;

/// Sleep and power-on settings. Kept whole besides the parts shown.
// The flags are what UGOS reports, one field each.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Power {
    /// Power on again after a power loss.
    #[serde(default)]
    pub power_boot: bool,
    /// Wake on LAN.
    #[serde(default)]
    pub wake_on: bool,
    /// Whether disks sleep.
    #[serde(default)]
    pub hard_drive_flag: bool,
    /// Minutes or hours before they sleep.
    #[serde(default)]
    pub hard_drive_time: i64,
    /// `M` or `H`.
    #[serde(default)]
    pub hard_drive_unit: String,
    /// `1` wake on login, `2` wake on read or write.
    #[serde(default)]
    pub hard_drive_strategy: i64,
    /// Whether USB disks sleep.
    #[serde(default)]
    pub usb_hard_drive_flag: bool,
    /// Minutes or hours before USB disks sleep.
    #[serde(default)]
    pub usb_hard_drive_time: i64,
    /// `M` or `H`.
    #[serde(default)]
    pub usb_hard_drive_unit: String,
    /// `1` wake on login, `2` wake on read or write.
    #[serde(default)]
    pub usb_hard_drive_strategy: i64,
    /// Log the sleeping.
    #[serde(default)]
    pub enable_hard_drive_sleep_log: bool,
    /// Whether the power schedule is on.
    #[serde(default)]
    pub enable_scheduled_power: bool,
    /// The power schedule.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub time_switches: Vec<PowerSwitch>,
    /// Whether the storage service runs; settings wait for it.
    #[serde(default)]
    pub storage_running: bool,
    /// What UGOS sent besides these, including the pools and disks that may
    /// sleep.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// One entry of the power schedule.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PowerSwitch {
    /// Weekdays, `1` Monday to `7` Sunday.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub frequency: Vec<i64>,
    /// `HH:MM`.
    #[serde(default)]
    pub time: String,
    /// `open` (power on) or `close` (power off).
    #[serde(default)]
    pub action: String,
}

/// The UPS setting. Kept whole besides the parts shown.
// The flags are what UGOS reports, one field each.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ups {
    /// `0` USB, `1` SNMP, `2` network UPS (NUT client), `-1` none.
    #[serde(default, rename = "type")]
    pub kind: i64,
    /// Whether the monitoring runs.
    #[serde(default)]
    pub status: bool,
    /// `0` standby, `1` shut down.
    #[serde(default)]
    pub protect_type: i64,
    /// Shut down at once on battery.
    #[serde(default)]
    pub right_now: bool,
    /// Minutes or hours on battery before acting, `-1` until the battery is low.
    #[serde(default)]
    pub standby_time: i64,
    /// `1` minutes, `2` hours.
    #[serde(default)]
    pub standby_time_unit: i64,
    /// Serve the UPS to other NAS as a NUT server (USB UPS).
    #[serde(default)]
    pub nut_server_enable: bool,
    /// Address of the NUT master, for a network UPS.
    #[serde(default)]
    pub ups_master_ip: String,
    /// SNMP address.
    #[serde(default)]
    pub snmp_ip: String,
    /// SNMP MIB.
    #[serde(default)]
    pub snmp_mib: String,
    /// SNMP version, `1` or `2`.
    #[serde(default)]
    pub snmp_ver: i64,
    /// SNMP community.
    #[serde(default)]
    pub snmp_community: String,
    /// A USB UPS is plugged in.
    #[serde(default)]
    pub usb_ups_connected: bool,
    /// The UPS as it reports itself.
    #[serde(default)]
    pub ups_info: Value,
    /// What UGOS sent besides these.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// A time zone.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Zone {
    /// Name in the time zone database, e.g. `Europe/Berlin`.
    #[serde(default, rename = "nameInTZDB")]
    pub name: String,
    /// The label the web UI shows.
    #[serde(default)]
    pub uistring: String,
}

/// Date, time zone and how the clock is kept.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeConfig {
    /// Time zone name.
    #[serde(default)]
    pub timezone: String,
    /// Unix time.
    #[serde(default)]
    pub time: i64,
    /// The time as shown, `YYYY-MM-DD HH:MM:SS`.
    #[serde(default)]
    pub time_show: String,
    /// `0` from a time server, `1` set by hand.
    #[serde(default)]
    pub time_set_type: i64,
    /// The time server.
    #[serde(default)]
    pub time_server: String,
    /// `24` or `12`.
    #[serde(default)]
    pub time_format: String,
    /// Date format, e.g. `YYYY-MM-DD`.
    #[serde(default)]
    pub date_format: String,
    /// The time zones.
    #[serde(default, rename = "zoneList", deserialize_with = "null_vec_as_empty")]
    pub zones: Vec<Zone>,
    /// What UGOS sent besides these.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// `time/server`.
#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct ServerList {
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub result: Vec<String>,
}

/// A language the NAS offers.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Language {
    /// Key, e.g. `de-DE`.
    #[serde(default)]
    pub key: String,
    /// Name in that language.
    #[serde(default)]
    pub name: String,
}

/// `language/list`.
#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct LanguageList {
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub result: Vec<Language>,
}

/// The SSH service. UGOS reads it as `SshConfig` and takes it as `sshConfig`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ssh {
    /// `1` on, `2` off.
    #[serde(default)]
    pub enable: i64,
    /// Port.
    #[serde(default)]
    pub port: i64,
    /// `1` the local network only, `2` any.
    #[serde(default)]
    pub permit_lan_access_only: i64,
    /// `1` SFTP on, `2` off.
    #[serde(default)]
    pub use_sftp: i64,
    /// Encryption level as UGOS reports it.
    #[serde(default)]
    pub encrypt_level: i64,
    /// How long SSH stays on: `1` 10 min, `2` 30 min, `3` 1 h, `4` 2 h,
    /// `5` 3 h, `6` 6 h, `7` until switched off.
    #[serde(default)]
    pub ex_time_duration: i64,
    /// When SSH closes by itself, Unix time, `0` for never.
    #[serde(default, skip_serializing)]
    pub ex_time_stamp: i64,
    /// Whether a session was cut at the deadline.
    #[serde(default, skip_serializing)]
    pub is_interrupted: bool,
}

/// The Telnet service.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Telnet {
    /// `1` on, `2` off.
    #[serde(default)]
    pub enable: i64,
    /// Port.
    #[serde(default)]
    pub port: i64,
    /// `1` the local network only, `2` any.
    #[serde(default)]
    pub permit_lan_access_only: i64,
}

/// `terminal/get`, with the capitals UGOS uses on the way out.
#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct TerminalGet {
    #[serde(default, rename = "SshConfig")]
    pub ssh: Ssh,
    #[serde(default, rename = "TelnetConfig")]
    pub telnet: Telnet,
}

/// SSH and Telnet together.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Terminal {
    /// SSH.
    pub ssh: Ssh,
    /// Telnet.
    pub telnet: Telnet,
}

/// The finder and improvement-plan switches.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finder {
    /// Join the improvement plan.
    #[serde(default)]
    pub improvement_enable: i64,
    /// Let the NAS be found from the web.
    #[serde(default)]
    pub web_find: i64,
}
