//! Command-line argument definitions.

use clap::{Args, Parser, Subcommand, ValueEnum};

/// UGOS NAS management CLI.
#[derive(Debug, Parser)]
#[command(name = "ugos", version, about)]
pub struct Cli {
    /// NAS hostname or IP address.
    #[arg(long, env = "UGOS_HOST", global = true)]
    pub host: Option<String>,

    /// Username. Falls back to `UGOS_USERNAME` when `UGOS_USER` is unset.
    #[arg(long, env = "UGOS_USER", global = true)]
    pub user: Option<String>,

    /// Password.
    #[arg(long, env = "UGOS_PASSWORD", global = true, hide_env_values = true)]
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
    /// Docker container management.
    Docker {
        #[command(subcommand)]
        action: DockerAction,
    },
    /// KVM audit logs.
    Log {
        #[command(subcommand)]
        action: LogAction,
    },
    /// Show NAS host info.
    Info,
    /// Show host load and every VM at once.
    Overview,
    /// Download Center: fetch files straight to the NAS.
    Download {
        #[command(subcommand)]
        action: DownloadAction,
    },
    /// NAS hardware and monitoring.
    System {
        #[command(subcommand)]
        action: SystemAction,
    },
    /// PCI passthrough devices.
    Passthrough {
        #[command(subcommand)]
        action: PassthroughAction,
    },
}

/// Download Center subcommands.
#[derive(Debug, Subcommand)]
pub enum DownloadAction {
    /// Show running and finished downloads.
    List,
    /// Queue a URL for the NAS to fetch.
    Add {
        /// The URL to download.
        url: String,
        /// Target directory [default: the configured one].
        #[arg(long)]
        dir: Option<String>,
    },
    /// Check whether a URL can be downloaded, without queueing it.
    Check {
        /// The URL to test.
        url: String,
    },
    /// Show target directory, free space and current rates.
    Status,
    /// Remove a task from the list.
    Rm {
        /// Numeric task id from `download list -o json` — the `id` field,
        /// not `task_id`, which the endpoint rejects.
        id: String,
        /// Also delete what was already fetched.
        #[arg(long)]
        delete_file: bool,
        /// Mark the task as still running rather than finished.
        #[arg(long)]
        running: bool,
    },
}

/// System subcommands.
#[derive(Debug, Subcommand)]
pub enum SystemAction {
    /// Model, serial, firmware and installed hardware.
    Info,
    /// Current CPU, memory, disk, network and fan readings.
    Stat,
    /// Running processes, busiest first.
    Processes {
        /// How many to show.
        #[arg(long, default_value = "20")]
        limit: usize,
    },
    /// Installed services and their resource use.
    Services,
}

/// PCI passthrough subcommands.
#[derive(Debug, Subcommand)]
pub enum PassthroughAction {
    /// List PCI devices available for passthrough.
    List,
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
    /// Create a VM.
    Create(Box<VmCreateArgs>),
    /// Update a VM (must be shut off). Only specified flags are changed.
    Update(Box<VmUpdateArgs>),
    /// Snapshot management.
    Snapshot {
        #[command(subcommand)]
        action: SnapshotAction,
    },
}

/// Flags for `vm create`.
///
/// Every device flag is repeatable and accepts either a short form or a
/// `key=value,...` list. Sizes take a unit suffix (`k`, `m`, `g`, `t`); a bare
/// number means MiB.
#[derive(Debug, Default, Args)]
pub struct VmCreateArgs {
    /// Display name for the VM.
    pub name: String,

    /// Use this JSON file as the base spec (`-` reads stdin); flags override it.
    #[arg(long, value_name = "PATH")]
    pub spec_file: Option<String>,

    /// Print the spec that would be sent instead of creating the VM.
    #[arg(long)]
    pub dry_run: bool,

    #[command(flatten)]
    pub spec: VmSpecFlags,
}

/// Flags for `vm update`.
///
/// The VM's current configuration is the base; only what the flags name is
/// changed. Device flags come in three forms: `--disk` and friends replace the
/// whole list, `--set-*` edits one existing entry selected with `match=`,
/// `--add-*` appends and `--rm-*` removes.
#[derive(Debug, Default, Args)]
pub struct VmUpdateArgs {
    /// VM name or UUID.
    pub name: String,

    /// New display name for the VM. Not verified against a live NAS.
    #[arg(long, value_name = "NAME")]
    pub rename: Option<String>,

    /// Use this JSON file as the base spec instead of the VM's current one
    /// (`-` reads stdin); the VM's UUID is always kept.
    #[arg(long, value_name = "PATH")]
    pub spec_file: Option<String>,

    /// Print the spec that would be sent instead of updating the VM.
    #[arg(long)]
    pub dry_run: bool,

    #[command(flatten)]
    pub spec: VmSpecFlags,

    #[command(flatten)]
    pub edits: VmEditFlags,
}

/// Device and resource flags shared by `vm create` and `vm update`.
///
/// Repeatable device flags replace the corresponding list wholesale.
#[derive(Debug, Default, Args)]
pub struct VmSpecFlags {
    /// OS type: linux, windows, other [create default: linux].
    #[arg(long)]
    pub os: Option<String>,

    /// OS version tag (e.g. `win11`).
    #[arg(long)]
    pub os_version: Option<String>,

    /// Number of CPU cores.
    #[arg(long)]
    pub cores: Option<i64>,

    /// Memory, e.g. `8192` (MiB) or `8g`.
    #[arg(long)]
    pub memory: Option<String>,

    /// Disk (repeatable): `40g` or `size=40g,bus=virtio,dev=vda,order=1,path=...`.
    #[arg(long, value_name = "SPEC")]
    pub disk: Vec<String>,

    /// ISO image (repeatable): `/path.iso` or `path=/path.iso,dev=hda,order=2`.
    #[arg(long, value_name = "SPEC")]
    pub iso: Vec<String>,

    /// Network interface (repeatable): `vnet-bridge0` or `name=...,type=virtio,mac=...`.
    #[arg(long, value_name = "SPEC")]
    pub nic: Vec<String>,

    /// KVM network name; shorthand for a single `--nic` [create default: vnet-bridge0].
    #[arg(long)]
    pub network: Option<String>,

    /// USB passthrough device (repeatable): `vendor-id=0x8087,product-id=0x0033,bus-id=1,device-id=4`
    /// or a raw JSON object. Schema not verified against a live NAS.
    #[arg(long, value_name = "SPEC")]
    pub usb: Vec<String>,

    /// Number of USB controllers [create default: 2].
    #[arg(long)]
    pub usb_controller: Option<i64>,

    /// Graphics card type [create default: virtio].
    #[arg(long)]
    pub graphics: Option<String>,

    /// Keyboard language [create default: en-us].
    #[arg(long)]
    pub keyboard: Option<String>,

    /// Shared directory (repeatable): `key=value,...` or a raw JSON object.
    /// Schema not verified against a live NAS.
    #[arg(long, value_name = "SPEC")]
    pub share: Vec<String>,

    /// Boot type: uefi or bios [create default: uefi].
    #[arg(long)]
    pub boot_type: Option<String>,

    /// Storage volume name [create default: volume1].
    #[arg(long)]
    pub storage: Option<String>,

    /// Auto-start on NAS boot (`--autostart` or `--autostart false`).
    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    pub autostart: Option<bool>,
}

/// Incremental device edits, `vm update` only.
///
/// `--set-*` takes a `match=<selector>` key naming the entry to edit; the
/// remaining keys are the same as on the corresponding `--disk`, `--iso` or
/// `--nic` flag. Edits are applied after any replacing flag, in the order
/// remove, set, add.
#[derive(Debug, Default, Args)]
pub struct VmEditFlags {
    /// Append a disk (repeatable), same syntax as `--disk`.
    #[arg(long, value_name = "SPEC")]
    pub add_disk: Vec<String>,

    /// Append an ISO image (repeatable), same syntax as `--iso`.
    #[arg(long, value_name = "SPEC")]
    pub add_iso: Vec<String>,

    /// Append a network interface (repeatable), same syntax as `--nic`.
    #[arg(long, value_name = "SPEC")]
    pub add_nic: Vec<String>,

    /// Edit a disk (repeatable): `match=vda,size=80g`.
    #[arg(long, value_name = "SPEC")]
    pub set_disk: Vec<String>,

    /// Edit an ISO image (repeatable): `match=hda,path=/volume1/iso/other.iso`.
    #[arg(long, value_name = "SPEC")]
    pub set_iso: Vec<String>,

    /// Edit a network interface (repeatable): `match=vnet-bridge0,type=e1000`.
    #[arg(long, value_name = "SPEC")]
    pub set_nic: Vec<String>,

    /// Remove a disk by device name (repeatable), e.g. `vdb`.
    #[arg(long, value_name = "DEV")]
    pub rm_disk: Vec<String>,

    /// Remove an ISO image by device name or path (repeatable).
    #[arg(long, value_name = "DEV|PATH")]
    pub rm_iso: Vec<String>,

    /// Remove a network interface by name or MAC address (repeatable).
    #[arg(long, value_name = "NAME|MAC")]
    pub rm_nic: Vec<String>,
}

/// Snapshot subcommands.
#[derive(Debug, Subcommand)]
pub enum SnapshotAction {
    /// List snapshots for a VM.
    List {
        /// VM name or UUID.
        vm: String,
    },
    /// Create a snapshot. UGOS picks the name and reports it back.
    Create {
        /// VM name or UUID.
        vm: String,
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
        /// Snapshot the current state before reverting.
        #[arg(long)]
        snapshot_first: bool,
    },
    /// Set a snapshot's description.
    Describe {
        /// VM name or UUID.
        vm: String,
        /// Snapshot name.
        name: String,
        /// Description text.
        description: String,
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
    /// Create a KVM network.
    Create {
        /// Network name.
        name: String,
        /// Network type: bridge, nat, none.
        #[arg(long, default_value = "bridge")]
        net_type: String,
        /// Mapping network interface (e.g. `bridge0`).
        #[arg(long)]
        interface: String,
    },
    /// Update a KVM network.
    Update {
        /// Network name.
        name: String,
        /// Mapping network interface.
        #[arg(long)]
        interface: Option<String>,
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
    /// Show how much space KVM uses per volume and per VM.
    Df,
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
    /// Register an image that already sits on the NAS, without uploading.
    /// Body taken from the web UI; not verified against a live NAS.
    Register {
        /// Full path on the NAS, e.g. `/volume1/isos/debian.iso`.
        path: String,
        /// Display name for the image [default: the file name without its extension].
        #[arg(long)]
        name: Option<String>,
    },
    /// Upload an ISO from a local file or an http(s) URL.
    Upload {
        /// Local path, or a URL starting with `http://` or `https://`.
        source: String,
        /// Display name for the image [default: the file name without its extension].
        #[arg(long)]
        name: Option<String>,
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

/// Docker subcommands.
#[derive(Debug, Subcommand)]
pub enum DockerAction {
    /// Show Docker engine overview.
    Overview,
    /// Show Docker engine status.
    Status,
    /// List containers.
    Ps {
        /// Page number.
        #[arg(long, default_value = "1")]
        page: u32,
        /// Page size.
        #[arg(long, default_value = "50")]
        page_size: u32,
    },
    /// Start a container.
    Start {
        /// Container ID.
        id: String,
    },
    /// Show container details.
    Show {
        /// Container ID.
        id: String,
    },
    /// Create a container.
    Create {
        /// Container name.
        name: String,
        /// Image (e.g. `nginx:latest`).
        #[arg(long)]
        image: String,
        /// Port mapping (repeatable, `host:container` or `host:container/udp`).
        #[arg(id = "publish", long = "publish", short = 'p')]
        port: Vec<String>,
        /// Environment variable (repeatable, `KEY=VALUE`).
        #[arg(long, short)]
        env: Vec<String>,
        /// Volume mount (repeatable, `host_path:container_path`).
        #[arg(long, short)]
        volume: Vec<String>,
        /// Restart policy: no, always, unless-stopped.
        #[arg(long, default_value = "no")]
        restart: String,
        /// Network mode: bridge, host.
        #[arg(long, default_value = "bridge")]
        network: String,
        /// Run in privileged mode.
        #[arg(long)]
        privileged: bool,
        /// Memory limit (e.g. `512m`, `2g`). 0 = unlimited.
        #[arg(long)]
        memory: Option<String>,
        /// CPU limit (number of cores). 0 = unlimited.
        #[arg(long)]
        cpus: Option<f64>,
    },
    /// Stop a container.
    Stop {
        /// Container ID.
        id: String,
    },
    /// Restart a container.
    Restart {
        /// Container ID.
        id: String,
    },
    /// Kill a container.
    Kill {
        /// Container ID.
        id: String,
    },
    /// Remove a container.
    Rm {
        /// Container ID.
        id: String,
    },
    /// List local images.
    Images {
        /// Page number.
        #[arg(long, default_value = "1")]
        page: u32,
        /// Page size.
        #[arg(long, default_value = "50")]
        page_size: u32,
    },
    /// Search Docker Hub for images.
    Search {
        /// Image name to search for.
        name: String,
    },
    /// Pull an image.
    Pull {
        /// Image name (e.g. `nginx`).
        image: String,
        /// Image tag (default: latest).
        #[arg(long, default_value = "latest")]
        tag: String,
    },
    /// Delete an image.
    Rmi {
        /// Image ID.
        id: String,
    },
    /// Export an image to a NAS path.
    Export {
        /// Image ID.
        id: String,
        /// Destination path on the NAS.
        path: String,
    },
    /// Load an image from a URL.
    LoadUrl {
        /// URL to load image from.
        url: String,
    },
    /// Load an image from a NAS path.
    LoadPath {
        /// Path to image file on the NAS.
        path: String,
    },
    /// List registry mirror sources.
    Mirrors,
    /// Add a registry mirror source.
    MirrorAdd {
        /// Display name.
        alias: String,
        /// Mirror URL.
        address: String,
    },
    /// Delete a registry mirror source.
    MirrorDelete {
        /// Mirror ID.
        id: i64,
    },
    /// Switch active registry mirror source.
    MirrorSwitch {
        /// Mirror ID.
        id: i64,
    },
    /// Show container logs.
    Logs {
        /// Container ID.
        id: String,
        /// Number of log lines.
        #[arg(long, default_value = "100")]
        lines: u32,
    },
    /// Clone a container.
    Clone {
        /// Source container ID.
        id: String,
        /// New container name.
        name: String,
    },
    /// Batch operate on containers.
    Batch {
        /// Operation: start, stop, restart, remove.
        action: String,
        /// Container IDs.
        ids: Vec<String>,
    },
    /// Show compose project containers.
    Compose {
        /// Project name.
        project: String,
    },
    /// List compose projects.
    ProjectLs,
    /// Show compose project details.
    ProjectShow {
        /// Project name.
        name: String,
    },
    /// Create a compose project from a `docker-compose.yml` file.
    ProjectCreate {
        /// Project name.
        name: String,
        /// Path to a local `docker-compose.yml` file to upload.
        #[arg(long)]
        file: String,
        /// NAS storage path for the project (default: `<shared-folder>/<name>`).
        #[arg(long)]
        path: Option<String>,
        /// Start the project immediately after creation.
        #[arg(long)]
        run: bool,
    },
    /// Start a compose project.
    ProjectStart {
        /// Project name.
        name: String,
    },
    /// Stop a compose project.
    ProjectStop {
        /// Project name.
        name: String,
    },
    /// Restart a compose project.
    ProjectRestart {
        /// Project name.
        name: String,
    },
    /// Remove a compose project (`docker compose down`).
    ProjectRm {
        /// Project name.
        name: String,
        /// Also remove images pulled for the project.
        #[arg(long)]
        del_images: bool,
    },
    /// Show Docker HTTP proxy configuration.
    ProxyGet,
    /// Set Docker HTTP proxy configuration.
    ProxySet {
        /// Proxy JSON (e.g. `{"http":"http://proxy:8080"}`).
        json: String,
    },
}
