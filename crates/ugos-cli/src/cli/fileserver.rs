//! Arguments of the file service commands: `smb`, `ftp`, `nfs`, `rsync`,
//! `webdav`.

use clap::{Args, Subcommand};

use super::OnOff;

/// Settings to change, and the confirmation for the risky ones.
#[derive(Debug, Args)]
pub struct ApplyArgs {
    /// Settings as `alias=value`, like `port=2121` or `encryption=force`. See
    /// `settings` for the aliases.
    #[arg(value_name = "ALIAS=VALUE")]
    pub settings: Vec<String>,
    /// Go on although the web UI asks for a confirmation: plain FTP, SMB1,
    /// `NTLMv1`, NFS, or discovery services while SMB is off.
    #[arg(long)]
    pub force: bool,
}

/// Switch off. SMB asks first when Bonjour or WS-Discovery announce it.
#[derive(Debug, Args)]
pub struct StopArgs {
    /// Go on although discovery services announce the SMB shares.
    #[arg(long)]
    pub force: bool,
}

/// Actions of a file service.
#[derive(Debug, Subcommand)]
pub enum ServiceAction {
    /// Show the service and its settings.
    Status,
    /// List the settings and what they take.
    Settings,
    /// Switch the service on, with these settings.
    Start(ApplyArgs),
    /// Change the settings of a running service.
    Set(ApplyArgs),
    /// Switch the service off.
    Stop(StopArgs),
}

/// rsync takes a password besides the settings.
#[derive(Debug, Args)]
pub struct RsyncArgs {
    #[command(flatten)]
    pub apply: ApplyArgs,
    /// Read the rsync password from stdin. Else `UGOS_RSYNC_PASSWORD`, else a
    /// prompt. Only needed for a new account.
    #[arg(long)]
    pub password_stdin: bool,
}

/// `rsync` subcommands.
#[derive(Debug, Subcommand)]
pub enum RsyncAction {
    /// Show the service and its settings.
    Status,
    /// List the settings and what they take.
    Settings,
    /// Switch the service on, with these settings.
    Start(RsyncArgs),
    /// Change the settings of a running service.
    Set(RsyncArgs),
    /// Switch the service off.
    Stop(StopArgs),
}

/// Discovery services and Time Machine.
#[derive(Debug, Args)]
pub struct DiscoveryArgs {
    /// WS-Discovery, which Windows uses to find the NAS.
    #[arg(long, value_enum)]
    pub wsdd2: Option<OnOff>,
    /// Bonjour, which Macs use to find the NAS.
    #[arg(long, value_enum)]
    pub bonjour: Option<OnOff>,
    /// `UPnP` announcements.
    #[arg(long, value_enum)]
    pub upnp: Option<OnOff>,
    /// Shares that are Time Machine targets, comma separated. `none` switches
    /// Time Machine off.
    #[arg(long, value_delimiter = ',')]
    pub time_machine: Option<Vec<String>>,
    /// Go on although UGOS advises against it: discovery services while SMB
    /// is off, or more than five Time Machine targets.
    #[arg(long)]
    pub force: bool,
}

/// `smb` subcommands.
#[derive(Debug, Subcommand)]
pub enum SmbAction {
    /// Show the service, its settings and the discovery services.
    Status,
    /// List the settings and what they take.
    Settings,
    /// Switch the service on, with these settings.
    Start(ApplyArgs),
    /// Change the settings of a running service.
    Set(ApplyArgs),
    /// Switch the service off.
    Stop(StopArgs),
    /// Show or change the discovery services and Time Machine targets.
    Discovery(DiscoveryArgs),
    /// Clear the SMB cache. The network service restarts after it.
    ClearCache(StopArgs),
}
