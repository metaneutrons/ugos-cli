//! Output formatting for CLI results.

use std::io::Write;

use anyhow::Result;
use serde::Serialize;
use tabled::{Table, Tabled};
use ugos_client::types::docker::{Container, DockerImage, Mirror};
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
}

/// Table row for snapshots.
#[derive(Tabled, Serialize)]
pub struct SnapshotRow {
    #[tabled(rename = "Name")]
    pub name: String,
    #[tabled(rename = "Display Name")]
    pub display_name: String,
    #[tabled(rename = "Current")]
    pub current: String,
}

impl From<&Snapshot> for SnapshotRow {
    fn from(s: &Snapshot) -> Self {
        Self {
            name: s.name.clone(),
            display_name: s.display_name.clone(),
            current: if s.is_current {
                "✓".into()
            } else {
                String::new()
            },
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
