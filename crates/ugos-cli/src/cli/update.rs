//! Arguments of `update`, `backup` and `index`.

use std::path::PathBuf;

use clap::{Args, Subcommand, ValueEnum};

/// How the NAS deals with updates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum AutoUpdate {
    /// Tell me and wait.
    Notify,
    /// Install important updates by itself.
    Important,
    /// Install any update by itself.
    Any,
}

impl AutoUpdate {
    /// The code UGOS uses.
    #[must_use]
    pub const fn code(self) -> i64 {
        match self {
            Self::Notify => 0,
            Self::Important => 1,
            Self::Any => 2,
        }
    }
}

/// The unit of a check interval.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum IntervalUnit {
    /// Hours.
    Hours,
    /// Days.
    Days,
    /// Months.
    Months,
}

impl IntervalUnit {
    /// The code UGOS uses.
    #[must_use]
    pub const fn code(self) -> i64 {
        match self {
            Self::Hours => 1,
            Self::Days => 2,
            Self::Months => 3,
        }
    }
}

/// Settings of the update check.
#[derive(Debug, Args)]
pub struct UpdateConfigArgs {
    /// What the NAS does with an update. `important` and `any` install by
    /// themselves, and restart: that needs `--force`.
    #[arg(long, value_enum)]
    pub auto: Option<AutoUpdate>,
    /// Check every N hours, days or months (1 to 72, with `--unit`).
    #[arg(long, requires = "unit", conflicts_with = "default_interval")]
    pub every: Option<i64>,
    /// The unit of `--every`.
    #[arg(long, value_enum)]
    pub unit: Option<IntervalUnit>,
    /// Let UGOS decide how often to check.
    #[arg(long)]
    pub default_interval: bool,
    /// Go on although the NAS then installs updates by itself.
    #[arg(long)]
    pub force: bool,
}

/// Only the confirmation.
#[derive(Debug, Args)]
pub struct ForceArgs {
    /// Go on although the web UI asks a confirmation for it.
    #[arg(long)]
    pub force: bool,
}

/// A file and the confirmation.
#[derive(Debug, Args)]
pub struct FileForceArgs {
    /// The file.
    pub file: PathBuf,
    /// Go on although the web UI asks a confirmation for it.
    #[arg(long)]
    pub force: bool,
}

/// `update` subcommands: the firmware.
#[derive(Debug, Subcommand)]
pub enum UpdateAction {
    /// The installed firmware, and the update that was found.
    Status,
    /// Show or change how the NAS looks for updates.
    Config(UpdateConfigArgs),
    /// Install the update that was found. The NAS restarts.
    Install(ForceArgs),
    /// Install a firmware image (`.img`) from a file. The NAS restarts.
    Upload(FileForceArgs),
    /// Trust a firmware signing key (`.pub`).
    Key(FileForceArgs),
    /// Clear the stored failure of the last install.
    Clean,
    /// Progress of an install from a file.
    Progress,
}

/// Restore: which services.
#[derive(Debug, Args)]
pub struct RestoreArgs {
    /// The backup file, `backup_restore.ugb`.
    pub file: PathBuf,
    /// Services to restore, as `category/item` or the item (see `services`),
    /// or `all`.
    #[arg(long = "service", required = true)]
    pub services: Vec<String>,
    /// Go on although the restore overwrites the configuration and restarts
    /// those services.
    #[arg(long)]
    pub force: bool,
}

/// `backup` subcommands: configuration backup, restore and resets.
#[derive(Debug, Subcommand)]
pub enum BackupAction {
    /// Save the configuration to a file. Name it `backup_restore.ugb`.
    Save {
        /// Where to write it.
        file: PathBuf,
    },
    /// The latest cloud backup and whether it is scheduled.
    Info,
    /// The services a restore can bring back.
    Services,
    /// Restore services from a backup file.
    Restore(RestoreArgs),
    /// Progress of a backup or a restore.
    Progress {
        /// A restore, not a backup.
        #[arg(long)]
        restore: bool,
    },
    /// Reset the network settings. The address may change.
    ResetNetwork(ForceArgs),
    /// Reset the NAS to factory state: data and configuration are deleted.
    ResetFactory(ForceArgs),
}

/// `index` subcommands: the thumbnail service.
#[derive(Debug, Subcommand)]
pub enum IndexAction {
    /// The state of the thumbnail service.
    Status,
    /// The folder ranges thumbnails are made for.
    Ranges,
    /// Pause for 1, 2 or 5 hours.
    Pause {
        /// Hours: 1, 2 or 5.
        #[arg(long, default_value = "1")]
        hours: i64,
    },
    /// Resume after a pause.
    Resume,
    /// Make thumbnails again.
    Regenerate {
        /// All ranges.
        #[arg(long, conflicts_with = "range")]
        all: bool,
        /// The ids of ranges (see `ranges`).
        #[arg(long, value_delimiter = ',')]
        range: Vec<i64>,
        /// Only the thumbnails that are missing.
        #[arg(long)]
        missing_only: bool,
    },
}
