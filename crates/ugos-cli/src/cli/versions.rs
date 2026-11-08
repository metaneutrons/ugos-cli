//! Arguments of the `version` commands: the File Version Explorer.

use clap::{Args, Subcommand, ValueEnum};

use super::OnOff;

/// Whose folders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum FolderKindArg {
    /// Shared folders.
    Share,
    /// Home folders.
    User,
    /// Home folders of domain users.
    Domain,
}

/// What to do when the destination has a file of that name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ClashArg {
    /// Leave the old file.
    Skip,
    /// Replace the old file.
    Overwrite,
    /// Keep both under different names.
    KeepBoth,
}

/// The time to look at.
#[derive(Debug, Args)]
pub struct AtArgs {
    /// The time to look at, `YYYY-MM-DD HH:MM:SS` in UTC. Now when not given.
    #[arg(long)]
    pub at: Option<String>,
}

/// `version` subcommands.
#[derive(Debug, Subcommand)]
pub enum VersionAction {
    /// The folders and whether they keep versions.
    Folders {
        /// Whose folders.
        #[arg(long, value_enum, default_value = "share")]
        kind: FolderKindArg,
    },
    /// Switch versions on or off, or change how many are kept.
    Set {
        /// The share or user, or its folder.
        name: String,
        /// Keep versions or not.
        #[arg(long, value_enum)]
        enable: Option<OnOff>,
        /// How many versions of a file to keep, 1 to 32.
        #[arg(long)]
        keep: Option<i64>,
        /// Scan the folder again.
        #[arg(long)]
        rescan: bool,
        /// Go on although UGOS deletes versions for it.
        #[arg(long)]
        force: bool,
    },
    /// How far UGOS is with migrating old histories.
    Status,
    /// List a folder as it was at a time.
    Ls {
        /// The folder.
        path: String,
        #[command(flatten)]
        at: AtArgs,
        /// Include what was deleted since.
        #[arg(long)]
        deleted: bool,
    },
    /// Every version of one file.
    History {
        /// The file, deleted or not.
        path: String,
    },
    /// Copy an older version into a folder on the NAS.
    Copy {
        /// The file or folder.
        path: String,
        /// The version, from `history`.
        #[arg(long)]
        version: String,
        /// The destination folder.
        #[arg(long)]
        to: String,
        /// What to do when the name is taken there.
        #[arg(long, value_enum, default_value = "keep-both")]
        clash: ClashArg,
        /// Go on although this overwrites a file.
        #[arg(long)]
        force: bool,
    },
    /// Put deleted files or folders back where they were.
    RestoreDeleted {
        /// Deleted items.
        #[arg(required = true)]
        paths: Vec<String>,
    },
    /// Drop all versions of deleted files or folders for good.
    ClearDeleted {
        /// Deleted items.
        #[arg(required = true)]
        paths: Vec<String>,
        /// Go on: the versions are gone for good.
        #[arg(long)]
        force: bool,
    },
    /// The days of a month with something made, changed or deleted.
    Calendar {
        /// The year.
        year: i64,
        /// The month, 1 to 12.
        month: i64,
    },
}
