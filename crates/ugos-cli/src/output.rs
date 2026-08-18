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

// ── Display row types ───────────────────────────────────────────────

/// Table row for VM list.
#[derive(Tabled, Serialize)]
pub struct VmRow {
    #[tabled(rename = "Name")]
    pub name: String,
    #[tabled(rename = "Status")]
    pub status: String,
    #[tabled(rename = "CPU%")]
    pub cpu: String,
    #[tabled(rename = "Memory")]
    pub memory: String,
    #[tabled(rename = "OS")]
    pub os: String,
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
#[derive(Tabled, Serialize)]
pub struct VmDetailRow {
    #[tabled(rename = "Field")]
    pub field: String,
    #[tabled(rename = "Value")]
    pub value: String,
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
#[derive(Tabled, Serialize)]
pub struct SnapshotRow {
    #[tabled(rename = "Name")]
    pub name: String,
    #[tabled(rename = "Created")]
    pub created: String,
    #[tabled(rename = "Description")]
    pub description: String,
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
#[derive(Tabled, Serialize)]
pub struct NetworkRow {
    #[tabled(rename = "Name")]
    pub name: String,
    #[tabled(rename = "Label")]
    pub label: String,
    #[tabled(rename = "Type")]
    pub net_type: String,
    #[tabled(rename = "Interface")]
    pub interface: String,
    #[tabled(rename = "VMs")]
    pub vms: String,
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
#[derive(Tabled, Serialize)]
pub struct NetDetailRow {
    #[tabled(rename = "Field")]
    pub field: String,
    #[tabled(rename = "Value")]
    pub value: String,
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
#[derive(Tabled, Serialize)]
pub struct StorageRow {
    #[tabled(rename = "Name")]
    pub name: String,
    #[tabled(rename = "Label")]
    pub label: String,
    #[tabled(rename = "Filesystem")]
    pub filesystem: String,
    #[tabled(rename = "Total")]
    pub total: String,
    #[tabled(rename = "Available")]
    pub available: String,
    #[tabled(rename = "VMs")]
    pub vms: String,
    #[tabled(rename = "Path")]
    pub path: String,
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
#[derive(Tabled, Serialize)]
pub struct ImageRow {
    #[tabled(rename = "Name")]
    pub name: String,
    #[tabled(rename = "File")]
    pub file: String,
    #[tabled(rename = "Type")]
    pub image_type: String,
    #[tabled(rename = "Size")]
    pub size: String,
    #[tabled(rename = "State")]
    pub state: String,
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
#[derive(Tabled, Serialize)]
pub struct HostInfoRow {
    #[tabled(rename = "Field")]
    pub field: String,
    #[tabled(rename = "Value")]
    pub value: String,
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
#[derive(Tabled, Serialize)]
pub struct UsbRow {
    #[tabled(rename = "Vendor")]
    pub vendor: String,
    #[tabled(rename = "Product")]
    pub product: String,
    #[tabled(rename = "Vendor ID")]
    pub vendor_id: String,
    #[tabled(rename = "Product ID")]
    pub product_id: String,
    #[tabled(rename = "Used By")]
    pub used_by: String,
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
#[derive(Tabled, Serialize)]
pub struct VncRow {
    #[tabled(rename = "Link")]
    pub link: String,
    #[tabled(rename = "Type")]
    pub link_type: String,
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
#[derive(Tabled, Serialize)]
pub struct LogRow {
    #[tabled(rename = "Time")]
    pub time: String,
    #[tabled(rename = "Operator")]
    pub operator: String,
    #[tabled(rename = "Content")]
    pub content: String,
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
#[derive(Tabled, Serialize)]
pub struct ProcessRow {
    #[tabled(rename = "ID")]
    pub pid: String,
    #[tabled(rename = "Name")]
    pub name: String,
    #[tabled(rename = "Status")]
    pub status: String,
    #[tabled(rename = "CPU%")]
    pub cpu: String,
    #[tabled(rename = "Memory")]
    pub memory: String,
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

/// Table row for download tasks.
#[derive(Tabled, Serialize)]
pub struct DownloadRow {
    #[tabled(rename = "File")]
    pub file: String,
    #[tabled(rename = "Size")]
    pub size: String,
    #[tabled(rename = "Progress")]
    pub progress: String,
    #[tabled(rename = "Speed")]
    pub speed: String,
    #[tabled(rename = "Target")]
    pub target: String,
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
#[derive(Tabled, Serialize)]
pub struct ContainerRow {
    #[tabled(rename = "ID")]
    pub id: String,
    #[tabled(rename = "Name")]
    pub name: String,
    #[tabled(rename = "Image")]
    pub image: String,
    #[tabled(rename = "Status")]
    pub status: String,
    #[tabled(rename = "CPU%")]
    pub cpu: String,
    #[tabled(rename = "Memory")]
    pub memory: String,
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
#[derive(Tabled, Serialize)]
pub struct DockerImageRow {
    #[tabled(rename = "ID")]
    pub id: String,
    #[tabled(rename = "Repository")]
    pub repository: String,
    #[tabled(rename = "Tag")]
    pub tag: String,
    #[tabled(rename = "Size")]
    pub size: String,
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
#[derive(Tabled, Serialize)]
pub struct MirrorRow {
    #[tabled(rename = "ID")]
    pub id: String,
    #[tabled(rename = "Name")]
    pub name: String,
    #[tabled(rename = "Address")]
    pub address: String,
    #[tabled(rename = "Active")]
    pub active: String,
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
#[derive(Tabled, Serialize)]
pub struct ComposeProjectRow {
    #[tabled(rename = "Name")]
    pub name: String,
    #[tabled(rename = "Path")]
    pub path: String,
    #[tabled(rename = "Containers")]
    pub containers: String,
    #[tabled(rename = "Status")]
    pub status: String,
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
