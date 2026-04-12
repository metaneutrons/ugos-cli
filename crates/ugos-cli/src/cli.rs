//! Command-line argument definitions.

use clap::{Parser, Subcommand, ValueEnum};

/// UGOS NAS management CLI.
#[derive(Debug, Parser)]
#[command(name = "ugos", version, about)]
pub struct Cli {
    /// NAS hostname or IP address.
    #[arg(long, env = "UGOS_HOST", global = true)]
    pub host: Option<String>,

    /// Username.
    #[arg(long, env = "UGOS_USER", global = true)]
    pub user: Option<String>,

    /// Password.
    #[arg(long, env = "UGOS_PASSWORD", global = true)]
    pub password: Option<String>,

    /// HTTPS port.
    #[arg(long, env = "UGOS_PORT", default_value = "9443", global = true)]
    pub port: u16,

    /// Output format.
    #[arg(long, short, default_value = "table", global = true)]
    pub output: OutputFormat,

    /// Skip session token cache.
    #[arg(long, global = true)]
    pub no_cache: bool,

    /// Resource to manage.
    #[command(subcommand)]
    pub command: Resource,
}

/// Output format selection.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    /// Human-readable table.
    Table,
    /// JSON output.
    Json,
}

/// Top-level resource subcommands.
#[derive(Debug, Subcommand)]
pub enum Resource {
    /// Virtual machine management.
    Vm {
        #[command(subcommand)]
        action: VmAction,
    },
    /// KVM network management.
    Network {
        #[command(subcommand)]
        action: NetworkAction,
    },
    /// KVM storage management.
    Storage {
        #[command(subcommand)]
        action: StorageAction,
    },
    /// KVM image management.
    Image {
        #[command(subcommand)]
        action: ImageAction,
    },
    /// USB device management.
    Usb {
        #[command(subcommand)]
        action: UsbAction,
    },
    /// VNC link management.
    Vnc {
        #[command(subcommand)]
        action: VncAction,
    },
    /// OVA export/import.
    Ova {
        #[command(subcommand)]
        action: OvaAction,
    },
    /// KVM audit logs.
    Log {
        #[command(subcommand)]
        action: LogAction,
    },
    /// Show NAS host info.
    Info,
}

/// VM subcommands.
#[derive(Debug, Subcommand)]
pub enum VmAction {
    /// List all VMs.
    List,
    /// Show VM details.
    Show {
        /// VM name or UUID.
        name: String,
    },
    /// Power on a VM.
    Start {
        /// VM name or UUID.
        name: String,
    },
    /// Shut down a VM.
    Stop {
        /// VM name or UUID.
        name: String,
        /// Force shutdown.
        #[arg(long)]
        force: bool,
    },
    /// Reboot a VM.
    Reboot {
        /// VM name or UUID.
        name: String,
        /// Force reboot.
        #[arg(long)]
        force: bool,
    },
    /// Delete a VM.
    Delete {
        /// VM name or UUID.
        name: String,
    },
    /// Create a VM from a JSON spec file.
    Create {
        /// Path to a JSON file containing the VM spec (`VmDetail`).
        file: String,
    },
    /// Update a VM from a JSON spec file (VM must be shut off).
    Update {
        /// Path to a JSON file containing the VM spec (`VmDetail`).
        file: String,
    },
    /// Snapshot management.
    Snapshot {
        #[command(subcommand)]
        action: SnapshotAction,
    },
}

/// Snapshot subcommands.
#[derive(Debug, Subcommand)]
pub enum SnapshotAction {
    /// List snapshots for a VM.
    List {
        /// VM name or UUID.
        vm: String,
    },
    /// Create a snapshot.
    Create {
        /// VM name or UUID.
        vm: String,
        /// Snapshot name.
        name: String,
    },
    /// Delete a snapshot.
    Delete {
        /// VM name or UUID.
        vm: String,
        /// Snapshot name.
        name: String,
    },
    /// Revert to a snapshot.
    Revert {
        /// VM name or UUID.
        vm: String,
        /// Snapshot name.
        name: String,
    },
    /// Rename a snapshot.
    Rename {
        /// VM name or UUID.
        vm: String,
        /// Current snapshot name.
        old_name: String,
        /// New snapshot name.
        new_name: String,
    },
}

/// Network subcommands.
#[derive(Debug, Subcommand)]
pub enum NetworkAction {
    /// List KVM networks.
    List,
    /// Show network details.
    Show {
        /// Network name.
        name: String,
    },
    /// Delete a KVM network.
    Delete {
        /// Network name.
        name: String,
    },
}

/// Storage subcommands.
#[derive(Debug, Subcommand)]
pub enum StorageAction {
    /// List storage volumes.
    List,
    /// Check which VMs use a storage volume.
    Usage {
        /// Volume name.
        name: String,
        /// Volume UUID.
        uuid: String,
    },
    /// Add a storage volume to KVM.
    Add {
        /// Volume name.
        name: String,
        /// Volume UUID.
        uuid: String,
    },
    /// Remove a storage volume from KVM.
    Delete {
        /// Volume name.
        name: String,
        /// Volume UUID.
        uuid: String,
    },
}

/// Image subcommands.
#[derive(Debug, Subcommand)]
pub enum ImageAction {
    /// List ISO/disk images.
    List,
    /// Delete an image.
    Delete {
        /// Image file name (e.g. `CachyOS.iso`).
        file_name: String,
        /// Image display name (e.g. `CachyOS`).
        image_name: String,
    },
    /// Check which VMs use an image.
    Usage {
        /// Image name.
        name: String,
    },
}

/// USB subcommands.
#[derive(Debug, Subcommand)]
pub enum UsbAction {
    /// List USB devices for a VM.
    List {
        /// VM name or UUID.
        vm: String,
    },
}

/// VNC subcommands.
#[derive(Debug, Subcommand)]
pub enum VncAction {
    /// List VNC links for a VM.
    List {
        /// VM name or UUID.
        vm: String,
    },
    /// Generate a noVNC link for a VM.
    Generate {
        /// VM name or UUID.
        vm: String,
        /// Base URL for the noVNC link.
        #[arg(long, default_value = "")]
        source_url: String,
    },
}

/// Log subcommands.
#[derive(Debug, Subcommand)]
pub enum LogAction {
    /// Search KVM audit logs.
    List {
        /// Page number.
        #[arg(long, default_value = "1")]
        page: u32,
        /// Page size.
        #[arg(long, default_value = "20")]
        page_size: u32,
    },
    /// List all operator usernames.
    Operators,
}

/// OVA subcommands.
#[derive(Debug, Subcommand)]
pub enum OvaAction {
    /// Export a VM as an OVA file.
    Export {
        /// VM name or UUID.
        vm: String,
        /// Storage volume name.
        storage_name: String,
        /// Storage volume UUID.
        storage_uuid: String,
        /// Output OVA file path on the NAS.
        ova_path: String,
    },
    /// Parse an OVA file and show the VM configuration it contains.
    Parse {
        /// OVA file path on the NAS.
        ova_path: String,
    },
}
