//! Types for the UGOS core API (system info, monitoring).
//!
//! Unlike the KVM and Docker apps, the core API uses `snake_case` field names,
//! so these types need no rename attribute. Every field defaults: this is
//! telemetry, and a missing one must not fail a whole listing.

use serde::{Deserialize, Serialize};

// ── Machine info ────────────────────────────────────────────────────

/// Machine identity and hardware from `sysinfo/machine/common`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MachineInfo {
    /// Identity and uptime.
    #[serde(default)]
    pub common: MachineCommon,
    /// Installed hardware.
    #[serde(default)]
    pub hardware: MachineHardware,
}

/// Identity of the NAS itself.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MachineCommon {
    /// Host name shown in the UI.
    #[serde(default)]
    pub nas_name: String,
    /// Model, e.g. `DXP480T Plus`.
    #[serde(default)]
    pub model: String,
    /// Model family, e.g. `dxp`.
    #[serde(default)]
    pub model_series: String,
    /// Product line, e.g. `nasync`.
    #[serde(default)]
    pub product_series: String,
    /// Serial number.
    #[serde(default)]
    pub serial: String,
    /// UGOS version, e.g. `1.18.1.0098`.
    #[serde(default)]
    pub system_version: String,
    /// Whether this is a beta build.
    #[serde(default)]
    pub beta: bool,
    /// Last boot as `YYYY-MM-DD HH:MM:SS`.
    #[serde(default)]
    pub last_turn_on_time: String,
    /// Seconds since boot.
    #[serde(default)]
    pub run_time: i64,
    /// Owner account, "-" when unset.
    #[serde(default)]
    pub nas_owner: String,
}

/// Installed hardware. Absent parts arrive as `null`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MachineHardware {
    /// CPUs.
    #[serde(default)]
    pub cpu: Vec<CpuInfo>,
    /// Memory modules.
    #[serde(default)]
    pub mem: Vec<MemInfo>,
    /// Network interfaces.
    #[serde(default)]
    pub net: Vec<NetInfo>,
    /// Graphics cards, if any.
    #[serde(default)]
    pub gpu: Option<Vec<serde_json::Value>>,
    /// Attached UPS, if any.
    #[serde(default)]
    pub ups: Option<Vec<serde_json::Value>>,
    /// USB devices, if any.
    #[serde(default)]
    pub usb: Option<Vec<serde_json::Value>>,
}

/// One CPU.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CpuInfo {
    /// Model name.
    #[serde(default)]
    pub model: String,
    /// Physical cores.
    #[serde(default)]
    pub core: i64,
    /// Hardware threads.
    #[serde(default)]
    pub thread: i64,
    /// Maximum clock in MHz, despite the field name.
    #[serde(default)]
    pub ghz: i64,
    /// Current temperature in °C.
    #[serde(default)]
    pub temperature: i64,
}

/// One memory module.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemInfo {
    /// Manufacturer.
    #[serde(default)]
    pub manufacturer: String,
    /// Part number.
    #[serde(default)]
    pub model: String,
    /// Size in bytes.
    #[serde(default)]
    pub size: i64,
    /// Clock, e.g. `5600 MHz`.
    #[serde(default)]
    pub mhz: String,
    /// Whether the module is ECC.
    #[serde(default)]
    pub is_ecc: bool,
}

/// One network interface.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetInfo {
    /// Interface name, e.g. `bridge0`.
    #[serde(default)]
    pub model: String,
    /// IPv4 address.
    #[serde(default)]
    pub ip: String,
    /// MAC address.
    #[serde(default)]
    pub mac: String,
    /// Netmask.
    #[serde(default)]
    pub mask: String,
    /// MTU.
    #[serde(default)]
    pub mtu: i64,
    /// Link speed in Mbit/s.
    #[serde(default)]
    pub speed: i64,
    /// Duplex mode, often empty.
    #[serde(default)]
    pub duplex: String,
}

// ── Live statistics ─────────────────────────────────────────────────

/// Current load from `taskmgr/stat/overview`.
///
/// Every series arrives as a list; UGOS sends one sample per call.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SystemStats {
    /// CPU utilisation and temperature.
    #[serde(default)]
    pub cpu: Vec<CpuStat>,
    /// Memory utilisation.
    #[serde(default)]
    pub mem: Vec<MemStat>,
    /// Physical disk throughput.
    #[serde(default)]
    pub disk: Vec<IoStat>,
    /// Volume throughput.
    #[serde(default)]
    pub volume: Vec<IoStat>,
    /// Network throughput.
    #[serde(default)]
    pub net: Vec<NetStat>,
    /// CPU fans.
    #[serde(default)]
    pub cpu_fan: Vec<FanStat>,
    /// Chassis fans.
    #[serde(default)]
    pub device_fan: Vec<FanStat>,
    /// GPUs.
    #[serde(default)]
    pub gpu: Vec<GpuStat>,
}

/// CPU sample.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CpuStat {
    /// Utilisation in percent.
    #[serde(default)]
    pub used_percent: f64,
    /// Temperature in °C.
    #[serde(default)]
    pub temp: f64,
    /// Unix timestamp of the sample.
    #[serde(default)]
    pub time: i64,
}

/// Memory sample.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemStat {
    /// Utilisation in percent.
    #[serde(default)]
    pub used_percent: f64,
    /// Unix timestamp of the sample.
    #[serde(default)]
    pub time: i64,
}

/// Disk or volume sample.
///
/// Careful with the units: for `volume` these two fields are **totals since
/// boot** in bytes despite their names — measured five seconds apart, the
/// write figure grew by exactly the bytes written. For `disk` they read as
/// rates, but that could not be confirmed because the NAS was idle.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IoStat {
    /// Bytes read: a rate for `disk`, a running total for `volume`.
    #[serde(default)]
    pub read_rate: f64,
    /// Bytes written: a rate for `disk`, a running total for `volume`.
    #[serde(default)]
    pub write_rate: f64,
    /// Utilisation in percent.
    #[serde(default)]
    pub used_percent: f64,
    /// Unix timestamp of the sample.
    #[serde(default)]
    pub time: i64,
}

/// Network sample.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetStat {
    /// Receive rate in bytes/s.
    #[serde(default)]
    pub recv_rate: f64,
    /// Send rate in bytes/s.
    #[serde(default)]
    pub send_rate: f64,
    /// Unix timestamp of the sample.
    #[serde(default)]
    pub time: i64,
}

/// Fan sample.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FanStat {
    /// Speed in RPM.
    #[serde(default)]
    pub speed: i64,
    /// Status flag; 1 appears to mean running.
    #[serde(default)]
    pub status: i64,
}

/// GPU sample. Present with empty values when no GPU is installed.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GpuStat {
    /// GPU name.
    #[serde(default)]
    pub gpu_name: String,
    /// Utilisation in percent.
    #[serde(default)]
    pub used_percent: f64,
    /// Temperature in °C.
    #[serde(default)]
    pub temp: f64,
    /// Used memory in bytes.
    #[serde(default)]
    pub mem_used: i64,
    /// Free memory in bytes.
    #[serde(default)]
    pub mem_free: i64,
    /// Driver version.
    #[serde(default)]
    pub driver_version: String,
}

// ── Processes and services ──────────────────────────────────────────

/// A process listing with the totals UGOS reports alongside it.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProcessList {
    /// The processes.
    #[serde(default)]
    pub list: Vec<Process>,
    /// Totals across all processes.
    #[serde(default)]
    pub total_consume: Consumption,
}

/// One process.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Process {
    /// Process id.
    #[serde(default)]
    pub pid: i64,
    /// Executable name.
    #[serde(default)]
    pub name: String,
    /// Description, often empty.
    #[serde(default)]
    pub desc: String,
    /// Human-readable state, e.g. `Sleeping`.
    #[serde(default)]
    pub status: String,
    /// Machine-readable state, e.g. `sleep`.
    #[serde(default)]
    pub process_status: String,
    /// Whether UGOS allows stopping it.
    #[serde(default)]
    pub can_be_operated: bool,
    /// This process's resource use.
    #[serde(default)]
    pub consume: Consumption,
}

/// A service listing with totals.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServiceList {
    /// The services.
    #[serde(default)]
    pub list: Vec<Service>,
    /// Totals across all services.
    #[serde(default)]
    pub total_consume: Consumption,
}

/// One service, which is an installed app.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Service {
    /// Service id, e.g. `snapshot_serv`.
    #[serde(default)]
    pub id: String,
    /// App id, e.g. `com.ugreen.snapshot`.
    #[serde(default)]
    pub appid: String,
    /// Display name.
    #[serde(default)]
    pub name: String,
    /// Whether UGOS allows stopping it.
    #[serde(default)]
    pub can_be_operated: bool,
    /// This service's resource use.
    #[serde(default)]
    pub consume: Consumption,
}

/// Resource use, reported per entry and as a total.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Consumption {
    /// CPU in percent.
    #[serde(default)]
    pub cpu_used_percent: f64,
    /// Memory in bytes.
    #[serde(default)]
    pub mem_used: i64,
    /// Memory in percent.
    #[serde(default)]
    pub mem_used_percent: f64,
    /// Disk read in bytes/s.
    #[serde(default)]
    pub disk_read_speed: f64,
    /// Disk write in bytes/s.
    #[serde(default)]
    pub disk_write_speed: f64,
    /// Network receive in bytes/s.
    #[serde(default)]
    pub net_recv_speed: f64,
    /// Network send in bytes/s.
    #[serde(default)]
    pub net_send_speed: f64,
    /// GPU in percent.
    #[serde(default)]
    pub gpu_used_percent: f64,
}
