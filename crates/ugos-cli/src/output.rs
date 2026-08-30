//! Output formatting for CLI results.

use std::io::Write;

use anyhow::Result;
use serde::Serialize;
use tabled::{Table, Tabled};
use ugos_client::types::docker::{ComposeProject, Container, DockerImage, Mirror};
use ugos_client::types::kvm::{
    HostInfo, ImageInfo, LogEntry, NetworkDetail, NetworkSummary, Snapshot, StorageInfo, UsbDevice,
    VmDetail, VmSummary, VncLink,
};

use crate::cli::OutputFormat;

/// Implement [`Tabled`] for a row type without the derive macro.
///
/// `tabled`'s derive lives in `tabled_derive`, which depends on the
/// unmaintained `proc-macro-error2`. That crate re-exports `proc_macro` in a
/// way Rust is phasing out (E0365), so the derive was dropped in favour of
/// this. Rendering is untouched — the same crate still draws the table, and
/// this produces the headers and cells the derive produced.
///
/// Header and field stay on one line, which the two separate lists a manual
/// impl would need cannot guarantee.
macro_rules! table_row {
    ($ty:ident { $($field:ident => $header:literal),+ $(,)? }) => {
        impl Tabled for $ty {
            const LENGTH: usize = <[()]>::len(&[$(table_row!(@unit $field)),+]);

            fn fields(&self) -> Vec<std::borrow::Cow<'_, str>> {
                vec![$(std::borrow::Cow::Owned(self.$field.to_string())),+]
            }

            fn headers() -> Vec<std::borrow::Cow<'static, str>> {
                vec![$(std::borrow::Cow::Borrowed($header)),+]
            }
        }
    };
    (@unit $field:ident) => { () };
}

// ── Display row types ───────────────────────────────────────────────

/// Table row for VM list.
#[derive(Serialize)]
pub struct VmRow {
    pub name: String,
    pub status: String,
    pub cpu: String,
    pub memory: String,
    pub os: String,
}

table_row! {
  VmRow {
    name => "Name",
    status => "Status",
    cpu => "CPU%",
    memory => "Memory",
    os => "OS",
  }
}

impl From<&VmSummary> for VmRow {
    fn from(v: &VmSummary) -> Self {
        Self {
            name: v.vir_display_name.clone(),
            status: v.status.clone(),
            cpu: format!("{}%", v.guest_cpu_percent),
            memory: format_mib(v.guest_used_memory),
            os: v.system_type.clone(),
        }
    }
}

/// Table row for VM detail.
#[derive(Serialize)]
pub struct VmDetailRow {
    pub field: String,
    pub value: String,
}

table_row! {
  VmDetailRow {
    field => "Field",
    value => "Value",
  }
}

/// Convert a `VmDetail` into key-value rows.
pub fn vm_detail_rows(d: &VmDetail) -> Vec<VmDetailRow> {
    vec![
        VmDetailRow {
            field: "Name".into(),
            value: d.virtual_machine_display_name.clone(),
        },
        VmDetailRow {
            field: "UUID".into(),
            value: d.virtual_machine_name.clone(),
        },
        VmDetailRow {
            field: "OS".into(),
            value: format!("{} {}", d.system_type, d.system_version),
        },
        VmDetailRow {
            field: "CPUs".into(),
            value: d.core.value.to_string(),
        },
        VmDetailRow {
            field: "Memory".into(),
            value: format_mib(d.memory.value),
        },
        VmDetailRow {
            field: "Storage".into(),
            value: d.storage_name.clone(),
        },
        VmDetailRow {
            field: "Boot".into(),
            value: d.device.boot_type.clone(),
        },
        VmDetailRow {
            field: "Graphics".into(),
            value: d.device.graphics_card.clone(),
        },
        VmDetailRow {
            field: "Auto-start".into(),
            value: d.other_config.auto_matic_start_up.to_string(),
        },
    ]
    .into_iter()
    .chain(d.dists.iter().map(|disk| VmDetailRow {
        field: format!("Disk {}", disk.dev),
        value: format!("{} ({})", format_mib(disk.size), disk.bus),
    }))
    .chain(d.images.iter().map(|image| VmDetailRow {
        field: format!("ISO {}", image.dev),
        value: image.path.clone(),
    }))
    .chain(d.networks.iter().map(|net| VmDetailRow {
        field: format!("NIC {}", net.name),
        value: format!("{} {}", net.nic_type, net.mac_address),
    }))
    .collect()
}

/// Table row for snapshots.
#[derive(Serialize)]
pub struct SnapshotRow {
    pub name: String,
    pub created: String,
    pub description: String,
}

table_row! {
  SnapshotRow {
    name => "Name",
    created => "Created",
    description => "Description",
  }
}

impl From<&Snapshot> for SnapshotRow {
    fn from(s: &Snapshot) -> Self {
        Self {
            name: s.name.clone(),
            created: s.create_time.clone(),
            description: s.description.clone(),
        }
    }
}

/// Table row for networks.
#[derive(Serialize)]
pub struct NetworkRow {
    pub name: String,
    pub label: String,
    pub net_type: String,
    pub interface: String,
    pub vms: String,
}

table_row! {
  NetworkRow {
    name => "Name",
    label => "Label",
    net_type => "Type",
    interface => "Interface",
    vms => "VMs",
  }
}

impl From<&NetworkSummary> for NetworkRow {
    fn from(n: &NetworkSummary) -> Self {
        Self {
            name: n.network_name.clone(),
            label: n.network_label.clone(),
            net_type: n.network_type.clone(),
            interface: n.interface_name.clone(),
            vms: n.virtual_display_names.join(", "),
        }
    }
}

/// Table row for network detail.
#[derive(Serialize)]
pub struct NetDetailRow {
    pub field: String,
    pub value: String,
}

table_row! {
  NetDetailRow {
    field => "Field",
    value => "Value",
  }
}

/// Convert a `NetworkDetail` into key-value rows.
pub fn net_detail_rows(d: &NetworkDetail) -> Vec<NetDetailRow> {
    vec![
        NetDetailRow {
            field: "Name".into(),
            value: d.network_name.clone(),
        },
        NetDetailRow {
            field: "UUID".into(),
            value: d.network_uuid.clone(),
        },
        NetDetailRow {
            field: "Type".into(),
            value: d.network_type.clone(),
        },
        NetDetailRow {
            field: "Mode".into(),
            value: d.network_mode.clone(),
        },
        NetDetailRow {
            field: "Interface".into(),
            value: d.mapping_network.clone(),
        },
        NetDetailRow {
            field: "IPv4".into(),
            value: d.enable_ipv4.to_string(),
        },
        NetDetailRow {
            field: "IPv4 Subnet".into(),
            value: d.ipv4_subnet.clone(),
        },
        NetDetailRow {
            field: "IPv6".into(),
            value: d.enable_ipv6.to_string(),
        },
    ]
}

/// Table row for storage.
#[derive(Serialize)]
pub struct StorageRow {
    pub name: String,
    pub label: String,
    pub filesystem: String,
    pub total: String,
    pub available: String,
    pub vms: String,
    pub path: String,
}

table_row! {
  StorageRow {
    name => "Name",
    label => "Label",
    filesystem => "Filesystem",
    total => "Total",
    available => "Available",
    vms => "VMs",
    path => "Path",
  }
}

impl From<&StorageInfo> for StorageRow {
    fn from(s: &StorageInfo) -> Self {
        Self {
            name: s.name.clone(),
            label: s.label.clone(),
            filesystem: s.filesystem.clone(),
            total: format_gib(s.total_capacity),
            available: format_gib(s.available_capacity),
            vms: s.vir_count.to_string(),
            path: s.path.clone(),
        }
    }
}

/// Table row for images.
#[derive(Serialize)]
pub struct ImageRow {
    pub name: String,
    pub file: String,
    pub image_type: String,
    pub size: String,
    pub state: String,
}

table_row! {
  ImageRow {
    name => "Name",
    file => "File",
    image_type => "Type",
    size => "Size",
    state => "State",
  }
}

impl From<&ImageInfo> for ImageRow {
    fn from(i: &ImageInfo) -> Self {
        Self {
            name: i.image_name.clone(),
            file: i.file_name.clone(),
            image_type: i.image_type.clone(),
            size: format_gib(i.file_size),
            state: i.state.clone(),
        }
    }
}

/// Table row for host info.
#[derive(Serialize)]
pub struct HostInfoRow {
    pub field: String,
    pub value: String,
}

table_row! {
  HostInfoRow {
    field => "Field",
    value => "Value",
  }
}

/// Convert `HostInfo` into key-value rows.
pub fn host_info_rows(h: &HostInfo) -> Vec<HostInfoRow> {
    vec![
        HostInfoRow {
            field: "CPU Cores".into(),
            value: h.cores.to_string(),
        },
        HostInfoRow {
            field: "Memory".into(),
            value: format_gib(h.memory),
        },
    ]
}

// ── USB ─────────────────────────────────────────────────────────────

/// Table row for USB devices.
#[derive(Serialize)]
pub struct UsbRow {
    pub vendor: String,
    pub product: String,
    pub vendor_id: String,
    pub product_id: String,
    pub used_by: String,
}

table_row! {
  UsbRow {
    vendor => "Vendor",
    product => "Product",
    vendor_id => "Vendor ID",
    product_id => "Product ID",
    used_by => "Used By",
  }
}

impl From<&UsbDevice> for UsbRow {
    fn from(u: &UsbDevice) -> Self {
        Self {
            vendor: u.vendor_name.clone(),
            product: u.product_name.clone(),
            vendor_id: u.vendor_id.clone(),
            product_id: u.product_id.clone(),
            used_by: if u.used_by.is_empty() {
                "-".into()
            } else {
                u.used_by.clone()
            },
        }
    }
}

// ── VNC ─────────────────────────────────────────────────────────────

/// Table row for VNC links.
#[derive(Serialize)]
pub struct VncRow {
    pub link: String,
    pub link_type: String,
}

table_row! {
  VncRow {
    link => "Link",
    link_type => "Type",
  }
}

impl From<&VncLink> for VncRow {
    fn from(v: &VncLink) -> Self {
        Self {
            link: v.link.clone(),
            link_type: v.link_type.to_string(),
        }
    }
}

// ── Logs ────────────────────────────────────────────────────────────

/// Table row for log entries.
#[derive(Serialize)]
pub struct LogRow {
    pub time: String,
    pub operator: String,
    pub content: String,
}

table_row! {
  LogRow {
    time => "Time",
    operator => "Operator",
    content => "Content",
  }
}

impl From<&LogEntry> for LogRow {
    fn from(l: &LogEntry) -> Self {
        Self {
            time: l.create_time.clone(),
            operator: l.operator.clone(),
            content: l.content.clone(),
        }
    }
}

// ── Formatting helpers ──────────────────────────────────────────────

/// Format KiB as human-readable MiB.
pub fn format_mib(kib: i64) -> String {
    format!("{} MiB", kib / 1024)
}

/// Rows for `overview`: host load first, then one line per VM.
#[allow(clippy::cast_precision_loss)]
pub fn overview_rows(ov: &ugos_client::types::kvm::Overview) -> Vec<VmDetailRow> {
    let h = &ov.host_stats;
    let mut rows = vec![
        VmDetailRow {
            field: "Host CPU".into(),
            value: format!("{:.1}%", h.cpu_util),
        },
        VmDetailRow {
            field: "Host memory".into(),
            value: format!(
                "{} of {} used",
                format_gib(h.total_used_mem),
                format_gib(h.host_total_mem)
            ),
        },
        VmDetailRow {
            field: "Memory by VMs".into(),
            value: format_gib(h.vm_used_mem),
        },
        VmDetailRow {
            field: "VMs".into(),
            value: format!(
                "{} total, {} running",
                ov.vm_list.len(),
                ov.vm_list.iter().filter(|v| v.status == "running").count()
            ),
        },
    ];
    rows.extend(ov.vm_list.iter().map(|vm| VmDetailRow {
        field: format!("  {}", vm.vir_display_name),
        value: format!(
            "{}, {}% CPU, {}",
            vm.status,
            vm.guest_cpu_percent,
            format_mib(vm.guest_used_memory)
        ),
    }));
    rows
}

/// Rows for `storage df`: one line per volume, then its VMs.
#[allow(clippy::cast_precision_loss)]
pub fn storage_usage_rows(usage: &[ugos_client::types::kvm::StorageUsage]) -> Vec<VmDetailRow> {
    let mut rows = Vec::new();
    for vol in usage {
        rows.push(VmDetailRow {
            field: vol.name.clone(),
            value: format!(
                "{} used by KVM ({:.1}%), {} free of {}",
                format_gib(vol.used_capacity),
                vol.used_percent,
                format_gib(vol.available_capacity),
                format_gib(vol.total_capacity)
            ),
        });
        rows.extend(vol.vm_usages.iter().map(|vm| VmDetailRow {
            field: format!("  {}", vm.vir_display_name),
            value: format!("{} ({:.2}%)", format_gib(vm.used_capacity), vm.used_percent),
        }));
    }
    rows
}

/// Rows for `system info`: identity first, then hardware.
#[allow(clippy::cast_precision_loss)]
pub fn machine_info_rows(m: &ugos_client::types::system::MachineInfo) -> Vec<VmDetailRow> {
    let c = &m.common;
    let mut rows = vec![
        VmDetailRow {
            field: "Name".into(),
            value: c.nas_name.clone(),
        },
        VmDetailRow {
            field: "Model".into(),
            value: format!("{} ({})", c.model, c.product_series),
        },
        VmDetailRow {
            field: "Serial".into(),
            value: c.serial.clone(),
        },
        VmDetailRow {
            field: "UGOS".into(),
            value: format!(
                "{}{}",
                c.system_version,
                if c.beta { " (beta)" } else { "" }
            ),
        },
        VmDetailRow {
            field: "Booted".into(),
            value: format!("{} (up {})", c.last_turn_on_time, format_uptime(c.run_time)),
        },
    ];
    rows.extend(m.hardware.cpu.iter().map(|cpu| VmDetailRow {
        field: "CPU".into(),
        value: format!(
            "{} — {} cores / {} threads, {} °C",
            cpu.model, cpu.core, cpu.thread, cpu.temperature
        ),
    }));
    rows.extend(m.hardware.mem.iter().map(|mem| VmDetailRow {
        field: "Memory".into(),
        value: format!(
            "{} {} ({}, {}{})",
            format_gib(mem.size),
            mem.mhz,
            mem.manufacturer,
            mem.model,
            if mem.is_ecc { ", ECC" } else { "" }
        ),
    }));
    rows.extend(m.hardware.net.iter().map(|net| VmDetailRow {
        field: format!("Net {}", net.model),
        value: format!("{} / {} — {} Mbit/s", net.ip, net.mac, net.speed),
    }));
    rows
}

/// Rows for `system stat`: one line per subsystem.
#[allow(clippy::cast_precision_loss)]
pub fn system_stat_rows(s: &ugos_client::types::system::SystemStats) -> Vec<VmDetailRow> {
    let mut rows = Vec::new();
    if let Some(cpu) = s.cpu.first() {
        rows.push(VmDetailRow {
            field: "CPU".into(),
            value: format!("{:.1}% at {:.0} °C", cpu.used_percent, cpu.temp),
        });
    }
    if let Some(mem) = s.mem.first() {
        rows.push(VmDetailRow {
            field: "Memory".into(),
            value: format!("{:.1}%", mem.used_percent),
        });
    }
    if let Some(disk) = s.disk.first() {
        rows.push(VmDetailRow {
            field: "Disk".into(),
            value: format!(
                "read {}/s, write {}/s",
                format_rate(disk.read_rate),
                format_rate(disk.write_rate)
            ),
        });
    }
    if let Some(vol) = s.volume.first() {
        // These are totals since boot, not rates — see IoStat.
        rows.push(VmDetailRow {
            field: "Volume total".into(),
            value: format!(
                "{} read, {} written",
                format_gib(bytes_to_i64(vol.read_rate)),
                format_gib(bytes_to_i64(vol.write_rate))
            ),
        });
    }
    if let Some(net) = s.net.first() {
        rows.push(VmDetailRow {
            field: "Network".into(),
            value: format!(
                "in {}/s, out {}/s",
                format_rate(net.recv_rate),
                format_rate(net.send_rate)
            ),
        });
    }
    for (i, fan) in s.cpu_fan.iter().enumerate() {
        rows.push(VmDetailRow {
            field: format!("CPU fan {}", i + 1),
            value: format!("{} rpm", fan.speed),
        });
    }
    for (i, fan) in s.device_fan.iter().enumerate() {
        rows.push(VmDetailRow {
            field: format!("Case fan {}", i + 1),
            value: format!("{} rpm", fan.speed),
        });
    }
    rows.extend(
        s.gpu
            .iter()
            .filter(|g| !g.gpu_name.is_empty())
            .map(|g| VmDetailRow {
                field: "GPU".into(),
                value: format!(
                    "{} — {:.1}% at {:.0} °C",
                    g.gpu_name, g.used_percent, g.temp
                ),
            }),
    );
    rows
}

/// Table row for processes and services.
#[derive(Serialize)]
pub struct ProcessRow {
    pub pid: String,
    pub name: String,
    pub status: String,
    pub cpu: String,
    pub memory: String,
}

table_row! {
  ProcessRow {
    pid => "ID",
    name => "Name",
    status => "Status",
    cpu => "CPU%",
    memory => "Memory",
  }
}

/// Rows for `system processes`, heaviest first.
pub fn process_rows(p: &ugos_client::types::system::ProcessList, limit: usize) -> Vec<ProcessRow> {
    let mut procs: Vec<_> = p.list.iter().collect();
    procs.sort_by(|a, b| {
        b.consume
            .cpu_used_percent
            .total_cmp(&a.consume.cpu_used_percent)
            .then(b.consume.mem_used.cmp(&a.consume.mem_used))
    });
    procs
        .into_iter()
        .take(limit)
        .map(|proc| ProcessRow {
            pid: proc.pid.to_string(),
            name: proc.name.clone(),
            status: proc.status.clone(),
            cpu: format!("{:.1}%", proc.consume.cpu_used_percent),
            memory: format_gib(proc.consume.mem_used),
        })
        .collect()
}

/// Rows for `system services`.
pub fn service_rows(s: &ugos_client::types::system::ServiceList) -> Vec<ProcessRow> {
    s.list
        .iter()
        .map(|svc| ProcessRow {
            pid: svc.id.clone(),
            name: format!("{} ({})", svc.name, svc.appid),
            status: if svc.can_be_operated {
                "controllable".into()
            } else {
                String::new()
            },
            cpu: format!("{:.1}%", svc.consume.cpu_used_percent),
            memory: format_gib(svc.consume.mem_used),
        })
        .collect()
}

/// Clamp a byte count that arrives as a float into an integer.
///
/// The casts are deliberate: these are byte counters, where a rounded value
/// and a saturating upper bound are exactly what the display needs.
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn bytes_to_i64(bytes: f64) -> i64 {
    if bytes.is_finite() && bytes > 0.0 {
        bytes.min(i64::MAX as f64).round() as i64
    } else {
        0
    }
}

/// Format seconds as `Nd Nh Nm`.
fn format_uptime(seconds: i64) -> String {
    let (d, h, m) = (
        seconds / 86400,
        (seconds % 86400) / 3600,
        (seconds % 3600) / 60,
    );
    if d > 0 {
        format!("{d}d {h}h {m}m")
    } else {
        format!("{h}h {m}m")
    }
}

/// Format a byte rate.
#[allow(clippy::cast_precision_loss)]
fn format_rate(bytes_per_second: f64) -> String {
    if bytes_per_second >= 1_048_576.0 {
        format!("{:.1} MiB", bytes_per_second / 1_048_576.0)
    } else if bytes_per_second >= 1024.0 {
        format!("{:.0} KiB", bytes_per_second / 1024.0)
    } else {
        format!("{bytes_per_second:.0} B")
    }
}

/// Table row for the system log.
#[derive(Serialize)]
pub struct SysLogRow {
    pub time: String,
    pub level: String,
    pub module: String,
    pub operator: String,
    pub content: String,
}

table_row! {
  SysLogRow {
    time => "Time",
    level => "Level",
    module => "Module",
    operator => "Operator",
    content => "Message",
  }
}

impl From<&ugos_client::types::syslog::LogEntry> for SysLogRow {
    fn from(e: &ugos_client::types::syslog::LogEntry) -> Self {
        Self {
            time: format_unix(e.create_time),
            level: e.level.clone(),
            module: e.module.clone(),
            operator: e.operator.clone(),
            content: e.content.clone(),
        }
    }
}

/// Table row for user accounts.
#[derive(Serialize)]
pub struct UserRow {
    pub username: String,
    pub account_type: String,
    pub email: String,
    pub description: String,
}

table_row! {
  UserRow {
    username => "User",
    account_type => "Type",
    email => "Email",
    description => "Description",
  }
}

impl From<&ugos_client::types::syslog::User> for UserRow {
    fn from(u: &ugos_client::types::syslog::User) -> Self {
        Self {
            username: u.username.clone(),
            account_type: if u.account_type == 0 {
                "standard".into()
            } else {
                format!("type {}", u.account_type)
            },
            email: u.email.clone(),
            description: u.description.clone(),
        }
    }
}

/// Table row for a directory listing.
#[derive(Serialize)]
pub struct FileRow {
    pub name: String,
    pub kind: String,
    pub size: String,
    pub modified: String,
}

table_row! {
  FileRow {
    name => "Name",
    kind => "Type",
    size => "Size",
    modified => "Modified",
  }
}

impl From<&ugos_client::types::files::FileEntry> for FileRow {
    fn from(e: &ugos_client::types::files::FileEntry) -> Self {
        Self {
            name: e.name.clone(),
            kind: if e.is_dir() {
                "dir".into()
            } else {
                e.ext.clone()
            },
            size: if e.is_dir() {
                String::new()
            } else {
                format_bytes(e.size)
            },
            modified: format_unix(e.mtime),
        }
    }
}

/// Table row for volumes as the file manager reports them.
#[derive(Serialize)]
pub struct VolumeRow {
    pub name: String,
    pub path: String,
    pub fs_type: String,
    pub used: String,
    pub free: String,
}

table_row! {
  VolumeRow {
    name => "Name",
    path => "Path",
    fs_type => "Filesystem",
    used => "Used",
    free => "Free",
  }
}

impl From<&ugos_client::types::files::Volume> for VolumeRow {
    fn from(v: &ugos_client::types::files::Volume) -> Self {
        Self {
            name: v.name.clone(),
            path: v.path.clone(),
            fs_type: v.fs_type.clone(),
            used: format_gib(v.used),
            free: format_gib(v.free),
        }
    }
}

/// Table row for a folder that can hold filesystem snapshots.
#[derive(Serialize)]
pub struct SnapshotFolderRow {
    pub folder: String,
    pub id: i64,
    pub snapshots: i64,
    pub latest: String,
    pub writable: String,
}

table_row! {
  SnapshotFolderRow {
    folder => "Folder",
    id => "ID",
    snapshots => "Snapshots",
    latest => "Latest",
    writable => "Writable",
  }
}

impl From<&ugos_client::types::snapshot::SnapshotFolder> for SnapshotFolderRow {
    fn from(f: &ugos_client::types::snapshot::SnapshotFolder) -> Self {
        Self {
            folder: f.folder_name.clone(),
            id: f.id,
            snapshots: f.snapshot_number,
            latest: format_unix(f.latest_snapshot_timestamp),
            writable: if f.allow_operations { "yes" } else { "no" }.to_owned(),
        }
    }
}

/// Table row for a filesystem snapshot.
#[derive(Serialize)]
pub struct FsSnapshotRow {
    pub id: i64,
    pub created: String,
    pub name: String,
    pub desc: String,
    pub locked: String,
}

table_row! {
  FsSnapshotRow {
    id => "ID",
    created => "Created",
    name => "Name",
    desc => "Description",
    locked => "Locked",
  }
}

impl From<&ugos_client::types::snapshot::Snapshot> for FsSnapshotRow {
    fn from(s: &ugos_client::types::snapshot::Snapshot) -> Self {
        Self {
            id: s.id,
            created: format_unix(s.create_timestamp),
            name: s.name.clone(),
            desc: s.desc.clone(),
            locked: if s.is_locked { "yes" } else { "no" }.to_owned(),
        }
    }
}

/// Format a unix timestamp as `YYYY-MM-DD HH:MM` in UTC.
fn format_unix(ts: i64) -> String {
    if ts <= 0 {
        return String::new();
    }
    // Days since the epoch, converted with the civil-from-days algorithm.
    let (days, secs) = (ts.div_euclid(86400), ts.rem_euclid(86400));
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}",
        secs / 3600,
        (secs % 3600) / 60
    )
}

/// Table row for download tasks.
#[derive(Serialize)]
pub struct DownloadRow {
    pub file: String,
    pub size: String,
    pub progress: String,
    pub speed: String,
    pub target: String,
}

table_row! {
  DownloadRow {
    file => "File",
    size => "Size",
    progress => "Progress",
    speed => "Speed",
    target => "Target",
  }
}

impl From<&ugos_client::types::download::DownloadTask> for DownloadRow {
    fn from(t: &ugos_client::types::download::DownloadTask) -> Self {
        let progress = if t.error_code != 0 {
            format!("failed ({})", t.error_code)
        } else if t.download_completed_time > 0 {
            "done".into()
        } else if t.plan > 0 {
            format!("{}%", t.plan)
        } else if t.total_size > 0 && t.downloaded_size > 0 {
            format!("{}%", t.downloaded_size * 100 / t.total_size)
        } else {
            "waiting".into()
        };
        Self {
            file: t.download_file_name.clone(),
            size: format_bytes(t.total_size),
            progress,
            speed: if t.download_speed > 0 {
                format!("{}/s", format_bytes(t.download_speed))
            } else {
                String::new()
            },
            target: t.save_dir.clone(),
        }
    }
}

/// Rows for `download status`.
pub fn download_status_rows(
    path: &ugos_client::types::download::DownloadPath,
    speed: &ugos_client::types::download::DownloadSpeed,
) -> Vec<VmDetailRow> {
    vec![
        VmDetailRow {
            field: "Target".into(),
            value: format!(
                "{} ({}){}",
                path.path,
                path.path_display,
                if path.path_is_validity {
                    ""
                } else {
                    " — missing"
                }
            ),
        },
        VmDetailRow {
            field: "Free".into(),
            value: format_gib(path.available_size),
        },
        VmDetailRow {
            field: "Tasks".into(),
            value: format!(
                "{} running, {} finished",
                speed.downloading_num, speed.completed_num
            ),
        },
        VmDetailRow {
            field: "Rate".into(),
            value: format!(
                "down {}/s, up {}/s",
                format_bytes(speed.download_speed),
                format_bytes(speed.upload_speed)
            ),
        },
    ]
}

/// Format a byte count with a binary unit.
#[allow(clippy::cast_precision_loss)]
fn format_bytes(bytes: i64) -> String {
    let b = bytes as f64;
    if b >= 1_073_741_824.0 {
        format!("{:.1} GiB", b / 1_073_741_824.0)
    } else if b >= 1_048_576.0 {
        format!("{:.1} MiB", b / 1_048_576.0)
    } else if b >= 1024.0 {
        format!("{:.0} KiB", b / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

/// Format bytes as human-readable GiB.
#[allow(clippy::cast_precision_loss)]
pub fn format_gib(bytes: i64) -> String {
    let gib = bytes as f64 / 1_073_741_824.0;
    format!("{gib:.1} GiB")
}

// ── Generic printers ────────────────────────────────────────────────

/// Print a list of items as a table or JSON.
///
/// # Errors
///
/// Returns an error if writing or JSON serialization fails.
pub fn print_list<T: Tabled + Serialize>(
    w: &mut impl Write,
    items: &[T],
    format: OutputFormat,
) -> Result<()> {
    match format {
        OutputFormat::Table => {
            if items.is_empty() {
                writeln!(w, "No results.")?;
            } else {
                writeln!(w, "{}", Table::new(items))?;
            }
        }
        OutputFormat::Json => {
            writeln!(w, "{}", serde_json::to_string_pretty(items)?)?;
        }
    }
    Ok(())
}

/// Print a success message (for mutating operations).
///
/// # Errors
///
/// Returns an error if writing fails.
pub fn print_success(w: &mut impl Write, msg: &str, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Table => writeln!(w, "{msg}")?,
        OutputFormat::Json => {
            writeln!(w, "{}", serde_json::json!({"status": "ok", "message": msg}))?;
        }
    }
    Ok(())
}

/// Print a raw JSON value.
///
/// # Errors
///
/// Returns an error if writing or serialization fails.
pub fn print_json(w: &mut impl Write, value: &impl Serialize) -> Result<()> {
    writeln!(w, "{}", serde_json::to_string_pretty(value)?)?;
    Ok(())
}

// ── Docker ──────────────────────────────────────────────────────────

/// Table row for Docker containers.
#[derive(Serialize)]
pub struct ContainerRow {
    pub id: String,
    pub name: String,
    pub image: String,
    pub status: String,
    pub cpu: String,
    pub memory: String,
}

table_row! {
  ContainerRow {
    id => "ID",
    name => "Name",
    image => "Image",
    status => "Status",
    cpu => "CPU%",
    memory => "Memory",
  }
}

impl From<&Container> for ContainerRow {
    fn from(c: &Container) -> Self {
        Self {
            id: c.container_id.chars().take(12).collect(),
            name: c.name.clone(),
            image: format!("{}:{}", c.image_name, c.version),
            status: c.status.clone(),
            cpu: String::new(),
            memory: String::new(),
        }
    }
}

/// Table row for Docker images.
#[derive(Serialize)]
pub struct DockerImageRow {
    pub id: String,
    pub repository: String,
    pub tag: String,
    pub size: String,
}

table_row! {
  DockerImageRow {
    id => "ID",
    repository => "Repository",
    tag => "Tag",
    size => "Size",
  }
}

impl From<&DockerImage> for DockerImageRow {
    fn from(i: &DockerImage) -> Self {
        Self {
            id: i.image_id.chars().take(19).collect(),
            repository: i.image_name.clone(),
            tag: i.image_version.clone(),
            size: format_mib(i.image_size / 1024),
        }
    }
}

/// Table row for registry mirrors.
#[derive(Serialize)]
pub struct MirrorRow {
    pub id: String,
    pub name: String,
    pub address: String,
    pub active: String,
}

table_row! {
  MirrorRow {
    id => "ID",
    name => "Name",
    address => "Address",
    active => "Active",
  }
}

impl From<&Mirror> for MirrorRow {
    fn from(m: &Mirror) -> Self {
        Self {
            id: m.id.to_string(),
            name: m.alias.clone(),
            address: m.address.clone(),
            active: if m.status { "✓" } else { "✗" }.into(),
        }
    }
}

/// Table row for compose projects.
#[derive(Serialize)]
pub struct ComposeProjectRow {
    pub name: String,
    pub path: String,
    pub containers: String,
    pub status: String,
}

table_row! {
  ComposeProjectRow {
    name => "Name",
    path => "Path",
    containers => "Containers",
    status => "Status",
  }
}

impl From<&ComposeProject> for ComposeProjectRow {
    fn from(p: &ComposeProject) -> Self {
        Self {
            name: p.name.clone(),
            path: p.path.clone(),
            containers: format!("{}/{}", p.run_container_sum, p.container_sum),
            status: if p.status == 1 { "up" } else { "down" }.into(),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod table_rendering_tests {
    use super::{FsSnapshotRow, VmRow, print_list};
    use crate::cli::OutputFormat;

    /// Renders a table with fixed values.
    ///
    /// The `Tabled` implementations come from the `table_row!` macro rather
    /// than a derive, so this pins the result the derive used to produce:
    /// column order, the renamed headers, and the borders tabled draws.
    #[test]
    fn a_row_renders_exactly_as_before() {
        let rows = vec![VmRow {
            name: "alpha".to_owned(),
            status: "running".to_owned(),
            cpu: "5%".to_owned(),
            memory: "512 MiB".to_owned(),
            os: "linux".to_owned(),
        }];
        let mut out = Vec::new();
        print_list(&mut out, &rows, OutputFormat::Table).unwrap();
        assert_eq!(
            String::from_utf8(out).unwrap(),
            "+-------+---------+------+---------+-------+\n\
             | Name  | Status  | CPU% | Memory  | OS    |\n\
             +-------+---------+------+---------+-------+\n\
             | alpha | running | 5%   | 512 MiB | linux |\n\
             +-------+---------+------+---------+-------+\n"
        );
    }

    #[test]
    fn columns_widen_to_the_longest_value() {
        // What made the live comparison differ: a longer cell widens its
        // column. Pinned here so it is understood as data, not formatting.
        let rows = vec![
            VmRow {
                name: "a".to_owned(),
                status: "up".to_owned(),
                cpu: "1%".to_owned(),
                memory: "7894 MiB".to_owned(),
                os: "linux".to_owned(),
            },
            VmRow {
                name: "b".to_owned(),
                status: "up".to_owned(),
                cpu: "2%".to_owned(),
                memory: "15853 MiB".to_owned(),
                os: "linux".to_owned(),
            },
        ];
        let mut out = Vec::new();
        print_list(&mut out, &rows, OutputFormat::Table).unwrap();
        let rendered = String::from_utf8(out).unwrap();
        assert!(rendered.contains("| 7894 MiB  |"), "{rendered}");
        assert!(rendered.contains("| 15853 MiB |"), "{rendered}");
    }

    #[test]
    fn numeric_fields_render_without_quotes() {
        // i64 columns go through Display, as the derive did.
        let rows = vec![FsSnapshotRow {
            id: 42,
            created: "2026-03-01 08:40".to_owned(),
            name: "snap".to_owned(),
            desc: String::new(),
            locked: "no".to_owned(),
        }];
        let mut out = Vec::new();
        print_list(&mut out, &rows, OutputFormat::Table).unwrap();
        let rendered = String::from_utf8(out).unwrap();
        assert!(rendered.contains("| 42 "), "{rendered}");
        assert!(rendered.contains("| ID "), "{rendered}");
    }

    #[test]
    fn an_empty_list_says_so() {
        let mut out = Vec::new();
        print_list(&mut out, &[] as &[VmRow], OutputFormat::Table).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "No results.\n");
    }
}
