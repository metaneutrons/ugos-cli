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
}

/// Storage subcommands.
#[derive(Debug, Subcommand)]
pub enum StorageAction {
    /// List storage volumes.
    List,
}

/// Image subcommands.
#[derive(Debug, Subcommand)]
pub enum ImageAction {
    /// List ISO/disk images.
    List,
}
