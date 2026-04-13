//! KVM-specific types for virtual machine management.

use serde::{Deserialize, Serialize};

// ── VM ──────────────────────────────────────────────────────────────

/// Summary of a VM as returned by `ShowLocalVirtualList`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VmSummary {
    /// VM UUID (used as the API identifier).
    pub vir_name: String,
    /// Numeric VM ID.
    #[serde(rename = "virID")]
    pub vir_id: i64,
    /// Human-readable display name.
    pub vir_display_name: String,
    /// Storage volume name.
    pub storage_name: String,
    /// OS type: "linux", "windows", "other".
    pub system_type: String,
    /// OS version (e.g. "win11", or empty).
    pub system_version: String,
    /// Guest CPU usage percentage.
    pub guest_cpu_percent: i64,
    /// Guest total memory in KiB.
    pub guest_total_memory: i64,
    /// Guest used memory in KiB.
    pub guest_used_memory: i64,
    /// Host CPU usage percentage.
    pub host_cpu_percent: i64,
    /// Host used memory in KiB.
    pub host_used_memory: i64,
    /// Host total memory in KiB.
    pub host_total_memory: i64,
    /// Upload bytes/s.
    pub upload: i64,
    /// Download bytes/s.
    pub download: i64,
    /// VM status: "running" or "shutoff".
    pub status: String,
    /// Process status (e.g. "createSuccess").
    pub process_status: String,
    /// Progress percentage (0-100).
    pub progress: i64,
    /// Unix timestamp of creation.
    pub create_time: i64,
}

/// Detailed VM configuration from `ShowLocalVirtualMachine`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VmDetail {
    /// VM UUID.
    pub virtual_machine_name: String,
    /// Human-readable display name.
    pub virtual_machine_display_name: String,
    /// OS type.
    pub system_type: String,
    /// OS version.
    pub system_version: String,
    /// CPU core allocation.
    pub core: VmResource,
    /// Memory allocation in KiB.
    pub memory: VmResource,
    /// Attached ISO images.
    pub images: Vec<VmImage>,
    /// Attached disks.
    pub dists: Vec<VmDisk>,
    /// Network interfaces.
    pub networks: Vec<VmNetwork>,
    /// Device configuration.
    pub device: VmDevice,
    /// Other configuration options.
    pub other_config: VmOtherConfig,
    /// Storage volume name.
    pub storage_name: String,
}

/// A simple `{value: N}` resource field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmResource {
    /// The resource value.
    pub value: i64,
}

/// An ISO image attached to a VM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmImage {
    /// File path on the NAS.
    pub path: String,
    /// Device name (e.g. "hda").
    pub dev: String,
    /// Boot order.
    pub order: i64,
}

/// A virtual disk attached to a VM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmDisk {
    /// Bus type (e.g. "virtio").
    pub bus: String,
    /// Disk size in bytes.
    pub size: i64,
    /// Device name (e.g. "vda").
    pub dev: String,
    /// File path to the qcow2 image.
    pub path: String,
    /// Boot order.
    pub order: i64,
}

/// A network interface attached to a VM.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VmNetwork {
    /// Network name (e.g. "vnet-bridge0").
    pub name: String,
    /// MAC address.
    pub mac_address: String,
    /// NIC type (e.g. "virtio").
    #[serde(rename = "type")]
    pub nic_type: String,
}

/// Device configuration for a VM.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VmDevice {
    /// Number of USB controllers.
    pub usb_controller: i64,
    /// Attached USB devices.
    pub usb_devices: Vec<serde_json::Value>,
    /// Graphics card type (e.g. "virtio").
    pub graphics_card: String,
    /// Boot type: "uefi" or "bios".
    pub boot_type: String,
}

/// Other VM configuration options.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VmOtherConfig {
    /// Whether the VM starts automatically on NAS boot.
    pub auto_matic_start_up: bool,
    /// Keyboard language (e.g. "de").
    pub keyboard_language: String,
    /// Shared directories.
    pub share_directory: Vec<serde_json::Value>,
}

// ── Snapshot ────────────────────────────────────────────────────────

/// A VM snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    /// Snapshot name (internal identifier).
    pub name: String,
    /// Display name.
    pub display_name: String,
    /// Creation timestamp.
    pub create_time: i64,
    /// Whether this is the current snapshot.
    pub is_current: bool,
}

// ── Network ─────────────────────────────────────────────────────────

/// Summary of a KVM network from `ShowNetworkList`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkSummary {
    /// Network name (e.g. "vnet-bridge0").
    pub network_name: String,
    /// Network UUID.
    #[serde(rename = "networkUUID")]
    pub network_uuid: String,
    /// Display label (e.g. "VBR-LAN1").
    pub network_label: String,
    /// Whether the network is valid/active.
    pub network_valid: bool,
    /// Network type: "bridge", "nat", "none".
    pub network_type: String,
    /// Network mode.
    pub network_mode: String,
    /// Host interface name.
    pub interface_name: String,
    /// VMs using this network.
    pub virtual_display_names: Vec<String>,
    /// Whether this is a system-managed network.
    pub system_network: bool,
}

/// Detailed network configuration from `GetNetworkByName`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::struct_excessive_bools)]
pub struct NetworkDetail {
    /// Network UUID.
    #[serde(rename = "networkUUID")]
    pub network_uuid: String,
    /// Network name.
    pub network_name: String,
    /// Network type.
    pub network_type: String,
    /// Network mode.
    pub network_mode: String,
    /// Mapped host network interface.
    pub mapping_network: String,
    /// IPv4 enabled.
    pub enable_ipv4: bool,
    /// IPv4 DHCP allocation enabled.
    pub allocate_ipv4: bool,
    /// IPv4 subnet.
    pub ipv4_subnet: String,
    /// IPv4 gateway.
    pub ipv4_gateway: String,
    /// IPv4 DHCP range start.
    #[serde(rename = "ipv4DHCPStartIp")]
    pub ipv4_dhcp_start_ip: String,
    /// IPv4 DHCP range end.
    #[serde(rename = "ipv4DHCPEndIp")]
    pub ipv4_dhcp_end_ip: String,
    /// IPv6 enabled.
    pub enable_ipv6: bool,
    /// IPv6 subnet.
    pub ipv6_subnet: String,
    /// IPv6 gateway.
    pub ipv6_gateway: String,
    /// IPv6 DHCP range start.
    #[serde(rename = "ipv6DHCPStartIp")]
    pub ipv6_dhcp_start_ip: String,
    /// IPv6 DHCP range end.
    #[serde(rename = "ipv6DHCPEndIp")]
    pub ipv6_dhcp_end_ip: String,
    /// IPv6 DHCP allocation enabled.
    pub allocate_ipv6: bool,
}

// ── Storage ─────────────────────────────────────────────────────────

/// Storage volume info from `ShowStorageList`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageInfo {
    /// Volume name (e.g. "volume1").
    pub name: String,
    /// Display label (e.g. "Volume 1").
    pub label: String,
    /// Health status code.
    pub health: i64,
    /// Status code.
    pub status: i64,
    /// Total capacity in bytes.
    #[serde(rename = "totalCapacity")]
    pub total_capacity: i64,
    /// Available capacity in bytes.
    #[serde(rename = "availableCapacity")]
    pub available_capacity: i64,
    /// Volume UUID.
    pub uuid: String,
    /// Mount path.
    pub path: String,
    /// Filesystem type (e.g. "btrfs").
    pub filesystem: String,
}

// ── Image ───────────────────────────────────────────────────────────

/// ISO/disk image info from `ShowImageList`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageInfo {
    /// Image ID.
    pub id: i64,
    /// File name on disk.
    pub file_name: String,
    /// Display name.
    pub image_name: String,
    /// File size in bytes.
    pub file_size: i64,
    /// Upload/processing progress.
    pub progress: i64,
    /// State (e.g. "completed").
    pub state: String,
    /// Image type (e.g. "iso").
    pub image_type: String,
    /// Full path on the NAS.
    pub path: String,
}

// ── USB ─────────────────────────────────────────────────────────────

/// USB device info from `USBList`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsbDevice {
    /// Vendor ID (e.g. "0x8087").
    #[serde(rename = "vendorID")]
    pub vendor_id: String,
    /// Vendor name.
    pub vendor_name: String,
    /// Product ID (e.g. "0x0033").
    #[serde(rename = "productID")]
    pub product_id: String,
    /// Product name.
    pub product_name: String,
    /// USB bus ID.
    #[serde(rename = "busID")]
    pub bus_id: i64,
    /// USB device ID.
    #[serde(rename = "deviceID")]
    pub device_id: i64,
    /// VM currently using this device (empty if unused).
    pub used_by: String,
}

// ── Host ────────────────────────────────────────────────────────────

/// Host hardware info from `ShowNativeInfo`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostInfo {
    /// Number of CPU cores.
    pub cores: i64,
    /// Total memory in bytes.
    pub memory: i64,
}

// ── VNC ─────────────────────────────────────────────────────────────

/// VNC link info from `ListAllLink`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VncLink {
    /// Link URL or identifier.
    #[serde(default)]
    pub link: String,
    /// Link type.
    #[serde(default, rename = "type")]
    pub link_type: i64,
    /// Password (if set).
    #[serde(default)]
    pub password: String,
    /// API key.
    #[serde(default)]
    pub api_key: String,
}

// ── Logs ────────────────────────────────────────────────────────────

/// A KVM log entry from `PageSearchLogs`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    /// Log ID.
    #[serde(default)]
    pub id: i64,
    /// Operator username.
    #[serde(default)]
    pub operator: String,
    /// Operation description.
    #[serde(default)]
    pub content: String,
    /// Creation timestamp.
    #[serde(default)]
    pub create_time: String,
}

/// Paginated log search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogPage {
    /// Log entries.
    #[serde(default)]
    pub list: Vec<LogEntry>,
    /// Total number of entries.
    #[serde(default)]
    pub total: i64,
}
