//! Arguments of the `logcenter` commands.

use std::path::PathBuf;

use clap::{Args, Subcommand};

use super::OnOff;

/// `logcenter archive set`.
#[derive(Debug, Args)]
pub struct ArchiveSetArgs {
    /// Run archiving or not.
    #[arg(long, value_enum)]
    pub enable: Option<OnOff>,
    /// Archive when the log reaches this many MB: 0, 500 or 1024.
    #[arg(long)]
    pub mb: Option<i64>,
    /// Archive at this many entries: 0, 1000000, 5000000 or 10000000.
    #[arg(long)]
    pub entries: Option<i64>,
    /// Archive entries older than this many months: 0, 3, 6 or 12.
    #[arg(long)]
    pub months: Option<i64>,
    /// The folder for the archives.
    #[arg(long)]
    pub path: Option<String>,
    /// Go on although archiving moves entries out of the log.
    #[arg(long)]
    pub force: bool,
}

/// `logcenter archive` subcommands.
#[derive(Debug, Subcommand)]
pub enum ArchiveAction {
    /// The archive policy.
    Show,
    /// Change it.
    Set(ArchiveSetArgs),
    /// The archives that were made.
    Records {
        /// The page, from 0.
        #[arg(long, default_value = "0")]
        page: u32,
    },
    /// The archives the viewer knows.
    Files,
    /// Add a `.ulog` archive to the viewer.
    Add {
        /// Path of the `.ulog` file on the NAS.
        path: String,
    },
    /// Make an archive the active log source.
    Open {
        /// The archive id, from `files`.
        id: i64,
    },
    /// Remove an archive from the viewer. The file stays.
    Remove {
        /// The archive id, from `files`.
        id: i64,
    },
}

/// `logcenter notify set`.
#[derive(Debug, Args)]
pub struct NotifySetArgs {
    /// Notify or not.
    #[arg(long, value_enum)]
    pub enable: Option<OnOff>,
    /// A keyword or expression. Empty for none.
    #[arg(long)]
    pub keyword: Option<String>,
    /// Levels: `info`, `warning`, `critical`. Replaces the list.
    #[arg(long = "level")]
    pub levels: Vec<String>,
    /// Modules, like `login`. Replaces the list.
    #[arg(long = "module")]
    pub modules: Vec<String>,
    /// User names. Replaces the list.
    #[arg(long = "operator")]
    pub operators: Vec<String>,
}

/// `logcenter notify` subcommands.
#[derive(Debug, Subcommand)]
pub enum NotifyAction {
    /// The notification policy.
    Show,
    /// Change it.
    Set(NotifySetArgs),
    /// The notifications that were sent.
    Records {
        /// The page, from 0.
        #[arg(long, default_value = "0")]
        page: u32,
    },
}

/// `logcenter filemgr` subcommands.
#[derive(Debug, Subcommand)]
pub enum FilemgrAction {
    /// Whether file manager operations are logged.
    Show,
    /// Switch that logging on or off.
    Set {
        /// On or off.
        #[arg(value_enum)]
        state: OnOff,
    },
}

/// `logcenter clear`.
#[derive(Debug, Args)]
pub struct ClearArgs {
    /// Clear the transfer log instead of the event log.
    #[arg(long)]
    pub transfer: bool,
    /// Clear entries older than this many days.
    #[arg(long, conflicts_with_all = ["before", "all"])]
    pub older_than_days: Option<i64>,
    /// Clear entries older than this time, `YYYY-MM-DD HH:MM:SS` in UTC.
    #[arg(long, conflicts_with = "all")]
    pub before: Option<String>,
    /// Clear everything.
    #[arg(long)]
    pub all: bool,
    /// Go on: cleared entries cannot be retrieved.
    #[arg(long)]
    pub force: bool,
}

/// `logcenter export`.
#[derive(Debug, Args)]
pub struct ExportArgs {
    /// Where to write the file.
    pub file: PathBuf,
    /// HTML instead of CSV.
    #[arg(long)]
    pub html: bool,
    /// The transfer log instead of the event log.
    #[arg(long)]
    pub transfer: bool,
    /// From this time on, `YYYY-MM-DD HH:MM:SS` in UTC.
    #[arg(long)]
    pub from: Option<String>,
    /// Up to this time, `YYYY-MM-DD HH:MM:SS` in UTC.
    #[arg(long)]
    pub to: Option<String>,
}

/// `logcenter` subcommands.
#[derive(Debug, Subcommand)]
pub enum LogCenterAction {
    /// The users and modules that appear in the log.
    Modules {
        /// The transfer log instead of the event log.
        #[arg(long)]
        transfer: bool,
    },
    /// Archiving and the archive viewer.
    Archive {
        #[command(subcommand)]
        action: ArchiveAction,
    },
    /// Notifications on matching entries.
    Notify {
        #[command(subcommand)]
        action: NotifyAction,
    },
    /// The logging of file manager operations.
    Filemgr {
        #[command(subcommand)]
        action: FilemgrAction,
    },
    /// Clear the log for good.
    Clear(ClearArgs),
    /// Save the log to a file.
    Export(ExportArgs),
}
