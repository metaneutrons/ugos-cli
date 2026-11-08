//! Arguments of the `recycle` commands.

use clap::{Args, Subcommand, ValueEnum};

use super::OnOff;

/// One of the three places a recycle bin can be listed from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum RecycleRoot {
    /// The bin of the account itself.
    Personal,
    /// The bins of the shared folders.
    Common,
    /// The bins of the user folders.
    Users,
}

/// The unit of a timed plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum PlanUnit {
    /// Days.
    Days,
    /// Months.
    Months,
}

/// What a cleanup plan does and when.
#[derive(Debug, Args)]
pub struct PlanArgs {
    /// A label.
    #[arg(long, default_value = "")]
    pub name: String,
    /// Delete what is older than this many days (1 to 365), every day.
    #[arg(long, conflicts_with_all = ["every", "first_run"])]
    pub retention_days: Option<i64>,
    /// Empty the bins every this many days or months (1 to 365).
    #[arg(long, requires_all = ["unit", "first_run"])]
    pub every: Option<i64>,
    /// The unit of `--every`.
    #[arg(long, value_enum)]
    pub unit: Option<PlanUnit>,
    /// The first run of a timed plan, `YYYY-MM-DD HH:MM:SS` in UTC.
    #[arg(long)]
    pub first_run: Option<String>,
    /// Time of day of a retention plan.
    #[arg(long, default_value = "12:00:00")]
    pub time: String,
    /// Cover every bin. Only one plan may.
    #[arg(long, conflicts_with_all = ["share", "user"])]
    pub all: bool,
    /// A share the plan covers, by name.
    #[arg(long = "share")]
    pub share: Vec<String>,
    /// A user the plan covers, by name.
    #[arg(long = "user")]
    pub user: Vec<String>,
}

/// `recycle plan` subcommands.
#[derive(Debug, Subcommand)]
pub enum PlanAction {
    /// The cleanup plans.
    List,
    /// The bins a plan can still take.
    Choices,
    /// Make a plan.
    Add(PlanArgs),
    /// Replace a plan.
    Edit {
        /// The plan id.
        id: i64,
        #[command(flatten)]
        plan: PlanArgs,
    },
    /// Delete a plan. The bins and their files stay.
    Remove {
        /// The plan id.
        id: i64,
    },
}

/// `recycle` subcommands.
#[derive(Debug, Subcommand)]
pub enum RecycleAction {
    /// The two switches of the recycle bin page.
    Status,
    /// Change them.
    Config {
        /// A shortcut on the desktop.
        #[arg(long, value_enum)]
        desktop: Option<OnOff>,
        /// The recycle folder shown inside folders.
        #[arg(long, value_enum)]
        folder: Option<OnOff>,
    },
    /// The bins of the shared folders and the user folders. Administrators
    /// only.
    List,
    /// Switch a bin on.
    Enable {
        /// The share or user, or its folder.
        name: String,
        /// Only administrators see what was deleted (shares).
        #[arg(long, value_enum)]
        admin_only: Option<OnOff>,
    },
    /// Switch a bin off.
    Disable {
        /// The share or user, or its folder.
        name: String,
        /// Go on although the bin holds files.
        #[arg(long)]
        force: bool,
    },
    /// List what is in a bin.
    Ls {
        /// A folder inside a bin, like `/volume1/docs/#recycle`.
        path: Option<String>,
        /// Or one of the three roots.
        #[arg(long, value_enum, conflicts_with = "path")]
        root: Option<RecycleRoot>,
    },
    /// Put files back where they were deleted from.
    Restore {
        /// Paths inside a bin.
        #[arg(required = true)]
        paths: Vec<String>,
    },
    /// Empty bins for good.
    Empty {
        /// The `#recycle` folders to empty.
        paths: Vec<String>,
        /// Or every bin of a root.
        #[arg(long, value_enum, conflicts_with = "paths")]
        root: Option<RecycleRoot>,
        /// Go on: the files are deleted for good.
        #[arg(long)]
        force: bool,
    },
    /// Delete files in a bin for good.
    Delete {
        /// Paths inside a bin.
        #[arg(required = true)]
        paths: Vec<String>,
        /// Go on: this cannot be undone.
        #[arg(long)]
        force: bool,
    },
    /// Scheduled cleanup.
    Plan {
        #[command(subcommand)]
        action: PlanAction,
    },
}
