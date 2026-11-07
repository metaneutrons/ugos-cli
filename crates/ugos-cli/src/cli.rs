//! Command-line argument definitions.

mod net;

pub use self::net::{
    BondAction, BridgeAction, DnsArgs, FlowAction, GatewayAction, Ipv6Arg, NetAction, ProxyAction,
    SetArgs, WifiCommand,
};

use clap::{Args, Parser, Subcommand, ValueEnum};

/// UGOS NAS management CLI.
// Global switches that stand alone (`--no-cache`, `--tls-insecure`,
// `--tls-trust-new`, `--password-stdin`); an enum would not fit them.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Parser)]
#[command(name = "ugos", version, about)]
pub struct Cli {
    /// NAS hostname or IP address.
    #[arg(long, env = "UGOS_HOST", global = true)]
    pub host: Option<String>,

    /// Username. Falls back to `UGOS_USERNAME` when `UGOS_USER` is unset.
    #[arg(long, env = "UGOS_USER", global = true)]
    pub user: Option<String>,

    /// Password. An argument shows up in the process list and the shell
    /// history; prefer `UGOS_PASSWORD` or `--password-stdin`.
    #[arg(long, env = "UGOS_PASSWORD", global = true, hide_env_values = true)]
    pub password: Option<String>,

    /// Read the password from the first line of stdin. Wins over
    /// `UGOS_PASSWORD`; combining it with `--password` is an error.
    #[arg(long, global = true)]
    pub password_stdin: bool,

    /// One-time authenticator code.
    ///
    /// Spent on the first login, so it cannot renew a session. A TOTP secret
    /// belongs in the `UGOS_OTP_SECRET` environment variable: a flag would
    /// show up in `ps` and the shell history. Only authenticator apps (type 1)
    /// are supported.
    #[arg(long, env = "UGOS_OTP", global = true, hide_env_values = true)]
    pub otp: Option<String>,

    /// HTTPS port.
    #[arg(long, env = "UGOS_PORT", default_value = "9443", global = true)]
    pub port: u16,

    /// Output format.
    #[arg(long, short, default_value = "table", global = true)]
    pub output: OutputFormat,

    /// Skip session token cache.
    #[arg(long, global = true)]
    pub no_cache: bool,

    /// Do not check the NAS certificate at all.
    ///
    /// Restores the behaviour of releases before 0.9 and offers no
    /// protection against an active man-in-the-middle.
    #[arg(long, global = true)]
    pub tls_insecure: bool,

    /// Accept and record the certificate currently presented.
    ///
    /// Needed after the NAS certificate legitimately changes, for instance
    /// when it is renewed or the device is reinstalled.
    #[arg(long, global = true)]
    pub tls_trust_new: bool,

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
    /// System log across all modules.
    Log(LogArgs),
    /// User accounts.
    User {
        #[command(subcommand)]
        action: UserAction,
    },
    /// Network settings of the NAS: interfaces, DNS, gateways, bonds,
    /// bridges, Wi-Fi and traffic control. `network` is the KVM network.
    Net {
        #[command(subcommand)]
        action: NetAction,
    },
    /// Storage pools: the RAID sets that volumes live on.
    Pool {
        #[command(subcommand)]
        action: PoolAction,
    },
    /// Volumes of the NAS. `storage` is the KVM storage and has nothing to do
    /// with these.
    Volume {
        #[command(subcommand)]
        action: VolumeAction,
    },
    /// Disks: state, SMART data and test plans.
    Disk {
        #[command(subcommand)]
        action: DiskAction,
    },
    /// Groups that an administrator made.
    Group {
        #[command(subcommand)]
        action: GroupAction,
    },
    /// Show NAS host info.
    Info,
    /// Show host load and every VM at once.
    Overview,
    /// Files on the NAS.
    Fs {
        #[command(subcommand)]
        action: FsAction,
    },
    /// Filesystem snapshots of shares and home folders
    Snapshot {
        #[command(subcommand)]
        action: FsSnapshotAction,
    },
    /// Shared folders: settings and access rules.
    Share {
        #[command(subcommand)]
        action: ShareAction,
    },
    /// The SMB file service.
    Smb {
        #[command(subcommand)]
        action: SmbAction,
    },
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
    /// Install the agent skill that teaches AI tools to use ugos-cli.
    ///
    /// Needs no NAS connection. Without an option it shows where the skill is
    /// and what would happen.
    Skill(SkillArgs),
}

/// Flags for the system log.
#[derive(Debug, Default, Args)]
pub struct LogArgs {
    /// Restrict to one module, e.g. `login`.
    #[arg(long)]
    pub module: Option<String>,

    /// Restrict to one severity, e.g. `info`.
    #[arg(long)]
    pub level: Option<String>,

    /// Restrict to one account.
    #[arg(long)]
    pub operator: Option<String>,

    /// Free-text search.
    #[arg(long)]
    pub keyword: Option<String>,

    /// Page number.
    #[arg(long, default_value = "1")]
    pub page: u32,

    /// Entries per page.
    #[arg(long, default_value = "20")]
    pub size: u32,
}

/// Pool subcommands. Reading only.
#[derive(Debug, Subcommand)]
pub enum PoolAction {
    /// List the pools.
    List,
    /// Show one pool with its disks, scrub and trim.
    Show {
        /// Pool name, e.g. `pool1`, or its label.
        pool: String,
    },
}

/// Volume subcommands. Reading only.
#[derive(Debug, Subcommand)]
pub enum VolumeAction {
    /// List the volumes.
    List,
    /// Show one volume.
    Show {
        /// Volume name, e.g. `volume1`, its label or its mount path.
        volume: String,
    },
}

/// Disk subcommands. Reading only.
#[derive(Debug, Subcommand)]
pub enum DiskAction {
    /// List the disks.
    List,
    /// Show one disk.
    Show {
        /// Short name, device path, slot or label, e.g. `nvme0n1` or `nvme1`.
        disk: String,
    },
    /// Show the SMART report of a disk.
    Smart {
        /// Short name, device path, slot or label.
        disk: String,
    },
    /// Show the history of a disk's tests.
    History {
        /// Short name, device path, slot or label.
        disk: String,
    },
    /// List the scheduled test plans.
    Plans,
}

/// Group subcommands.
#[derive(Debug, Subcommand)]
pub enum GroupAction {
    /// List the groups, with their members. Built-in groups are not listed.
    List,
}

/// Role of an account.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum RoleArg {
    /// A standard user.
    User,
    /// An administrator.
    Admin,
}

/// User subcommands.
///
/// The password of a new or changed account is never a flag, since a flag
/// shows in the process list and the shell history. It is asked for on the
/// terminal, read from stdin with `--new-password-stdin`, or taken from
/// `UGOS_NEW_PASSWORD`.
#[derive(Debug, Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum UserAction {
    /// List accounts.
    List,
    /// Show the account this session belongs to.
    Me,
    /// Show one account in full.
    Show {
        /// Login name or numeric uid. Not called `user`: that is the id of
        /// the global `--user` option, and clap would fill both.
        account: String,
    },
    /// Create an account.
    Create {
        /// Login name.
        name: String,
        /// Email address.
        #[arg(long)]
        email: Option<String>,
        /// Free-text description.
        #[arg(long)]
        description: Option<String>,
        /// Role of the account.
        #[arg(long, value_enum, default_value = "user")]
        role: RoleArg,
        /// Group to join; repeat for several.
        #[arg(long = "group")]
        groups: Vec<String>,
        /// Do not give the account a home folder.
        #[arg(long)]
        no_home_dir: bool,
        /// Forbid the account to change its own password.
        #[arg(long)]
        deny_password_change: bool,
        /// Read the password from stdin.
        #[arg(long)]
        new_password_stdin: bool,
        /// Print the request without sending it. The password is not shown.
        #[arg(long)]
        dry_run: bool,
    },
    /// Change an account. Only what is given changes.
    Update {
        /// Login name or numeric uid.
        account: String,
        /// New email address.
        #[arg(long)]
        email: Option<String>,
        /// New description.
        #[arg(long)]
        description: Option<String>,
        /// New role.
        #[arg(long, value_enum)]
        role: Option<RoleArg>,
        /// Add the account to a group; repeat for several.
        #[arg(long = "add-group")]
        add_groups: Vec<String>,
        /// Remove the account from a group; repeat for several.
        #[arg(long = "remove-group")]
        remove_groups: Vec<String>,
        /// Give the account a home folder.
        #[arg(long, conflicts_with = "no_home_dir")]
        home_dir: bool,
        /// Take the home folder away.
        #[arg(long)]
        no_home_dir: bool,
        /// Forbid the account to change its own password.
        #[arg(long, conflicts_with = "allow_password_change")]
        deny_password_change: bool,
        /// Let the account change its own password.
        #[arg(long)]
        allow_password_change: bool,
        /// Disable the account.
        #[arg(long, conflicts_with_all = ["enable", "no_expiry"])]
        disable: bool,
        /// Enable a disabled account, or lift an expiry.
        #[arg(long, conflicts_with = "no_expiry")]
        enable: bool,
        /// Remove the expiry date.
        #[arg(long, conflicts_with = "expires")]
        no_expiry: bool,
        /// Let the account expire on a date, `YYYY-MM-DD`.
        #[arg(long, value_name = "DATE", conflicts_with_all = ["disable", "enable"])]
        expires: Option<String>,
        /// Set a new password, read from stdin.
        #[arg(long)]
        new_password_stdin: bool,
        /// Set a new password, asked for on the terminal or taken from
        /// `UGOS_NEW_PASSWORD`.
        #[arg(long, conflicts_with = "new_password_stdin")]
        new_password: bool,
    },
    /// Delete an account.
    Delete {
        /// Login name or numeric uid.
        account: String,
        /// Delete although the account has snapshots or files in its home
        /// folder, which go with it.
        #[arg(long)]
        force: bool,
    },
}

/// Filesystem snapshot actions.
///
/// These are btrfs snapshots of shares and home folders, unrelated to the
/// KVM snapshots under `vm snapshot`.
#[derive(Debug, Subcommand)]
pub enum FsSnapshotAction {
    /// List folders that can hold snapshots.
    Folders,
    /// List the snapshots of a folder.
    List {
        /// Folder name, as shown by `snapshot folders`.
        folder: String,
    },
    /// Take a snapshot of a folder. UGOS names it after the current time.
    Create {
        /// Folder name to snapshot.
        folder: String,
        /// Description stored with the snapshot.
        #[arg(long, default_value = "")]
        desc: String,
        /// Mark as locked. The web UI then refuses to delete it, but the
        /// API does not enforce this — `snapshot rm` removes it anyway.
        #[arg(long)]
        lock: bool,
    },
    /// Change a snapshot's description or lock state.
    Edit {
        /// Folder the snapshot belongs to.
        folder: String,
        /// Snapshot id, as shown by `snapshot list`.
        id: i64,
        /// New description.
        #[arg(long, default_value = "")]
        desc: String,
        /// Lock the snapshot against deletion.
        #[arg(long)]
        lock: bool,
    },
    /// Delete snapshots, locked ones included.
    Rm {
        /// Folder the snapshots belong to.
        folder: String,
        /// Snapshot ids to delete (repeatable).
        ids: Vec<i64>,
    },
    /// Copy a snapshot into a new folder of its own.
    ///
    /// The non-destructive way to reach a snapshot's contents: it appears
    /// beside the original instead of replacing it.
    Clone {
        /// Folder the snapshot belongs to.
        folder: String,
        /// Snapshot id to clone.
        id: i64,
        /// Name for the new folder.
        new_name: String,
    },
}

/// Shared folder actions.
///
/// A share is a top-level folder on a volume that UGOS offers over SMB. The
/// name is the folder's base name; a number is read as the share id when no
/// share has that name.
#[derive(Debug, Subcommand)]
pub enum ShareAction {
    /// List the shared folders.
    List,
    /// Show the settings of a share.
    Show {
        /// Share name or id, as shown by `share list`.
        share: String,
    },
    /// Create a share.
    ///
    /// The defaults are the web UI's: a recycle bin that only administrators
    /// may empty, hidden from users without access, visible in the network,
    /// no checksums, no encryption, no space limit. Only administrators
    /// have access at first; grant the rest with `share acl set`.
    Create {
        /// Name of the new folder.
        name: String,
        /// Volume to create it on, such as `/volume1`. Optional when the NAS
        /// has a single volume.
        #[arg(long)]
        volume: Option<String>,
        /// Space limit, a number and a unit: `500m`, `100g` or `2t`.
        #[arg(long)]
        quota: Option<String>,
        /// Do not announce the share when browsing the network. It stays
        /// reachable by name.
        #[arg(long)]
        hide_in_network: bool,
        /// Show the share to users who have no access to it.
        #[arg(long)]
        show_unreadable: bool,
        /// Do not keep a recycle bin.
        #[arg(long)]
        no_recycle: bool,
        /// Let any user with access empty the recycle bin, not only
        /// administrators.
        #[arg(long)]
        recycle_for_everyone: bool,
        /// Turn on data checksums (btrfs).
        #[arg(long)]
        data_check: bool,
        /// Print the request instead of sending it.
        #[arg(long)]
        dry_run: bool,
    },
    /// Change the settings of a share.
    ///
    /// The name and the volume stay as they are.
    Update {
        /// Share name or id.
        share: String,
        /// New space limit: `500m`, `100g` or `2t`.
        #[arg(long, conflicts_with = "no_quota")]
        quota: Option<String>,
        /// Remove the space limit.
        #[arg(long)]
        no_quota: bool,
        /// Do not announce the share when browsing the network.
        #[arg(long, conflicts_with = "show_in_network")]
        hide_in_network: bool,
        /// Announce the share when browsing the network.
        #[arg(long)]
        show_in_network: bool,
        /// Hide the share from users who have no access to it.
        #[arg(long, conflicts_with = "show_unreadable")]
        hide_unreadable: bool,
        /// Show the share to users who have no access to it.
        #[arg(long)]
        show_unreadable: bool,
    },
    /// Delete a share and everything in its folder.
    ///
    /// The folder's files are deleted with it. UGOS asks for the account's
    /// password for this, and `ugos-cli` sends the one it logged in with.
    /// Refuses when the folder holds files, unless `--force` is given.
    Delete {
        /// Share name or id.
        share: String,
        /// Delete the share although its folder is not empty.
        #[arg(long)]
        force: bool,
    },
    /// Access rules of a share.
    Acl {
        #[command(subcommand)]
        action: ShareAclAction,
    },
}

/// Access rule actions of a share.
#[derive(Debug, Subcommand)]
pub enum ShareAclAction {
    /// Show who has which access.
    Show {
        /// Share name or id.
        share: String,
    },
    /// Set access for users and groups.
    ///
    /// Each rule is `user:NAME=ACCESS` or `group:NAME=ACCESS`, where ACCESS is
    /// `none`, `read`, `rw` or `inherit` (nothing set for this user or group,
    /// so the other rules decide). Entries that are not named keep their
    /// access. UGOS keeps administrators at read and write.
    Set {
        /// Share name or id.
        share: String,
        /// Rules, such as `user:anna=rw` or `group:guests=read`.
        #[arg(required = true)]
        rules: Vec<String>,
    },
}

/// SMB service actions.
#[derive(Debug, Subcommand)]
pub enum SmbAction {
    /// Show the SMB service: state, workgroup, protocol settings and the
    /// discovery services.
    Status,
}

/// File manager subcommands.
#[derive(Debug, Subcommand)]
pub enum FsAction {
    /// List a directory.
    Ls {
        /// Absolute path on the NAS, e.g. `/volume1/download`.
        path: String,
    },
    /// List the storage volumes.
    Volumes,
    /// Create a directory.
    Mkdir {
        /// Absolute path of the directory to create.
        path: String,
    },
    /// Delete files or directories.
    Rm {
        /// Absolute paths to delete (repeatable).
        paths: Vec<String>,
        /// Bypass the recycle bin.
        #[arg(long)]
        forever: bool,
    },
    /// Download a file from the NAS.
    Get {
        /// Absolute path on the NAS.
        remote: String,
        /// Local destination [default: the file name in the current directory].
        local: Option<String>,
    },
    /// Upload a local file to a directory on the NAS.
    Put {
        /// Local file to upload.
        local: String,
        /// Absolute target directory on the NAS.
        remote_dir: String,
    },
    /// Rename a file or directory.
    Mv {
        /// Absolute path of the entry.
        path: String,
        /// New base name (not a path).
        new_name: String,
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
    /// KVM audit log.
    Log {
        #[command(subcommand)]
        action: LogAction,
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
    ///
    /// For a private registry such as ghcr.io, pass `--registry-user` and the
    /// password through `UGOS_REGISTRY_PASSWORD` or `--registry-password-stdin`.
    /// There is deliberately no flag for the password itself: arguments show up
    /// in the process list.
    MirrorAdd {
        /// Display name.
        alias: String,
        /// Mirror URL.
        address: String,
        /// Registry user name, stored with the source.
        #[arg(long)]
        registry_user: Option<String>,
        /// Read the registry password from stdin instead of
        /// `UGOS_REGISTRY_PASSWORD`.
        #[arg(long, requires = "registry_user")]
        registry_password_stdin: bool,
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
    /// Recreate a compose project from a `docker-compose.yml` file
    /// (`docker compose up -d` with the new content).
    ProjectUpdate {
        /// Project name.
        name: String,
        /// Path to a local `docker-compose.yml` file; its content replaces the
        /// project's compose file.
        #[arg(long)]
        file: String,
        /// Pull the images again before recreating.
        #[arg(long)]
        pull: bool,
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

/// Options of `ugos-cli skill`.
// Four switches on a command line, at most one action plus `--yes`; clap
// enforces the exclusions, so an enum would only restate them.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Args)]
pub struct SkillArgs {
    /// Install where it is missing, after asking.
    #[arg(long, conflicts_with_all = ["decline", "remove"])]
    pub install: bool,

    /// Stop offering it where it is missing.
    #[arg(long, conflicts_with_all = ["install", "remove"])]
    pub decline: bool,

    /// Remove the copies ugos-cli installed and nobody changed.
    #[arg(long, conflicts_with_all = ["install", "decline"])]
    pub remove: bool,

    /// Install without asking. Needed when there is no terminal to ask on.
    #[arg(long, short = 'y', requires = "install")]
    pub yes: bool,
}

impl Cli {
    /// `true` when `--password-stdin` and another flag that reads a password
    /// from stdin would both read the one stream.
    pub const fn reads_stdin_twice(&self) -> bool {
        self.password_stdin
            && match &self.command {
                Resource::Docker {
                    action:
                        DockerAction::MirrorAdd {
                            registry_password_stdin,
                            ..
                        },
                } => *registry_password_stdin,
                Resource::User {
                    action:
                        UserAction::Create {
                            new_password_stdin, ..
                        }
                        | UserAction::Update {
                            new_password_stdin, ..
                        },
                } => *new_password_stdin,
                _ => false,
            }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::{Cli, Resource, ShareAclAction, ShareAction, SmbAction, UserAction};
    use clap::parser::ValueSource;
    use clap::{CommandFactory as _, FromArgMatches as _};

    fn parse(args: &[&str]) -> Result<(clap::ArgMatches, Cli), clap::Error> {
        let matches = Cli::command().try_get_matches_from(args)?;
        let cli = Cli::from_arg_matches(&matches)?;
        Ok((matches, cli))
    }

    #[test]
    fn the_account_argument_does_not_fill_the_global_user_option() {
        // The positional once shared the id of `--user`, so the account to
        // show became the login name and the login failed.
        for args in [
            &["ugos-cli", "user", "show", "anna"][..],
            &["ugos-cli", "user", "update", "anna", "--description", "x"][..],
            &["ugos-cli", "user", "delete", "anna"][..],
        ] {
            let (matches, cli) = parse(args).unwrap();
            assert_eq!(cli.user, None, "{args:?}");
            assert_eq!(matches.value_source("user"), None, "{args:?}");
        }
        let (_, cli) = parse(&["ugos-cli", "--user", "admin", "user", "show", "anna"]).unwrap();
        assert_eq!(cli.user.as_deref(), Some("admin"));
        assert!(matches!(
            cli.command,
            Resource::User {
                action: UserAction::Show { ref account }
            } if account == "anna"
        ));
    }

    #[test]
    fn two_stdin_password_flags_are_caught() {
        let (_, cli) = parse(&[
            "ugos-cli",
            "--password-stdin",
            "user",
            "create",
            "anna",
            "--new-password-stdin",
        ])
        .unwrap();
        assert!(cli.reads_stdin_twice());
        let (_, cli) = parse(&["ugos-cli", "--password-stdin", "user", "create", "anna"]).unwrap();
        assert!(!cli.reads_stdin_twice());
    }

    #[test]
    fn password_stdin_is_a_global_flag() {
        let (_, cli) = parse(&["ugos-cli", "vm", "list", "--password-stdin"]).unwrap();
        assert!(cli.password_stdin);
    }

    #[test]
    fn a_password_argument_is_seen_as_a_command_line_value() {
        let (matches, _) = parse(&["ugos-cli", "--password", "x", "vm", "list"]).unwrap();
        assert_eq!(
            matches.value_source("password"),
            Some(ValueSource::CommandLine)
        );
        let (matches, _) = parse(&["ugos-cli", "vm", "list", "--password", "x"]).unwrap();
        assert_eq!(
            matches.value_source("password"),
            Some(ValueSource::CommandLine)
        );
    }

    #[test]
    fn no_password_argument_means_no_command_line_source() {
        let (matches, _) = parse(&["ugos-cli", "--password-stdin", "vm", "list"]).unwrap();
        assert_ne!(
            matches.value_source("password"),
            Some(ValueSource::CommandLine)
        );
    }

    #[test]
    fn both_stdin_flags_are_caught() {
        let (_, cli) = parse(&[
            "ugos-cli",
            "--password-stdin",
            "docker",
            "mirror-add",
            "alias",
            "https://registry.example",
            "--registry-user",
            "me",
            "--registry-password-stdin",
        ])
        .unwrap();
        assert!(cli.reads_stdin_twice());
        let (_, cli) = parse(&["ugos-cli", "--password-stdin", "vm", "list"]).unwrap();
        assert!(!cli.reads_stdin_twice());
    }

    #[test]
    fn share_create_takes_its_options() {
        let (_, cli) = parse(&[
            "ugos-cli",
            "share",
            "create",
            "media",
            "--volume",
            "/volume1",
            "--quota",
            "500g",
            "--hide-in-network",
            "--dry-run",
        ])
        .unwrap();
        let Resource::Share {
            action:
                ShareAction::Create {
                    name,
                    volume,
                    quota,
                    hide_in_network,
                    dry_run,
                    show_unreadable,
                    ..
                },
        } = cli.command
        else {
            panic!("not a share create");
        };
        assert_eq!(name, "media");
        assert_eq!(volume.as_deref(), Some("/volume1"));
        assert_eq!(quota.as_deref(), Some("500g"));
        assert!(hide_in_network && dry_run);
        assert!(!show_unreadable);
    }

    #[test]
    fn share_update_refuses_contradicting_flags() {
        for args in [
            &["share", "update", "x", "--quota", "5g", "--no-quota"][..],
            &[
                "share",
                "update",
                "x",
                "--hide-in-network",
                "--show-in-network",
            ][..],
            &[
                "share",
                "update",
                "x",
                "--hide-unreadable",
                "--show-unreadable",
            ][..],
        ] {
            let mut argv = vec!["ugos-cli"];
            argv.extend_from_slice(args);
            let err = parse(&argv).unwrap_err();
            assert_eq!(
                err.kind(),
                clap::error::ErrorKind::ArgumentConflict,
                "{args:?}"
            );
        }
    }

    #[test]
    fn share_acl_set_needs_at_least_one_rule() {
        let err = parse(&["ugos-cli", "share", "acl", "set", "media"]).unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::MissingRequiredArgument);
        let (_, cli) = parse(&[
            "ugos-cli",
            "share",
            "acl",
            "set",
            "media",
            "user:anna=rw",
            "group:family=read",
        ])
        .unwrap();
        let Resource::Share {
            action:
                ShareAction::Acl {
                    action: ShareAclAction::Set { share, rules },
                },
        } = cli.command
        else {
            panic!("not a share acl set");
        };
        assert_eq!(share, "media");
        assert_eq!(rules, vec!["user:anna=rw", "group:family=read"]);
    }

    #[test]
    fn share_delete_and_smb_status_parse() {
        let (_, cli) = parse(&["ugos-cli", "share", "delete", "media", "--force"]).unwrap();
        assert!(matches!(
            cli.command,
            Resource::Share {
                action: ShareAction::Delete { force: true, .. }
            }
        ));
        let (_, cli) = parse(&["ugos-cli", "smb", "status"]).unwrap();
        assert!(matches!(
            cli.command,
            Resource::Smb {
                action: SmbAction::Status
            }
        ));
    }

    #[test]
    fn the_totp_secret_is_not_a_flag() {
        let mut cmd = Cli::command();
        let help = cmd.render_long_help().to_string();
        assert!(help.contains("--otp"), "the one-shot code flag is missing");
        assert!(
            !help.contains("otp-secret"),
            "UGOS_OTP_SECRET must not be a flag: {help}"
        );
    }
}
