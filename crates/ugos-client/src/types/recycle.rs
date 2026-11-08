//! Types for the recycle bin.

use serde::{Deserialize, Serialize};

use super::common::null_vec_as_empty;

/// The two switches of the recycle bin page.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecycleConf {
    /// A shortcut on the desktop.
    #[serde(default)]
    pub desktop_recycle: bool,
    /// The recycle folder shown inside folders.
    #[serde(default)]
    pub dir_recycle: bool,
}

/// How long a bin keeps its files before the automatic cleanup.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CleaningDay {
    /// The number of days.
    #[serde(default)]
    pub interval: i64,
}

/// The recycle bin of a shared folder or of a user's folder.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecycleBin {
    /// Name of the share or the user.
    #[serde(default)]
    pub name: String,
    /// The folder the bin belongs to.
    #[serde(default)]
    pub path: String,
    /// Whether the bin is switched on.
    #[serde(default)]
    pub status: bool,
    /// Where the `#recycle` folder is.
    #[serde(default)]
    pub recycle_path: String,
    /// Share id, `0` for a user.
    #[serde(default)]
    pub share_id: i64,
    /// User id, `0` for a share.
    #[serde(default)]
    pub user_id: i64,
    /// Only administrators see what was deleted.
    #[serde(default)]
    pub allow_only_admin_recycle: bool,
    /// Whether the bin holds files.
    #[serde(default)]
    pub recycle_not_empty: bool,
    /// Not empty while a task empties it.
    #[serde(default)]
    pub empty_recycle_task_id: String,
    /// The retention of the automatic cleanup.
    #[serde(default)]
    pub dynamic_cleaning_day: CleaningDay,
    /// The next cleanup, as a date.
    #[serde(default)]
    pub next_cleaning_time: String,
}

/// `getRecycleList`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecycleList {
    /// The bins of the shared folders.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub recycle_shares: Vec<RecycleBin>,
    /// The bins of the user folders.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub recycle_users: Vec<RecycleBin>,
}

/// A file in a recycle bin.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecycleFile {
    /// The full path, inside `#recycle`.
    #[serde(default)]
    pub path: String,
    /// The name.
    #[serde(default)]
    pub name: String,
    /// Size in bytes.
    #[serde(default)]
    pub size: i64,
    /// `0` file, `1` folder.
    #[serde(default)]
    pub file_type: i64,
    /// When it was deleted, Unix seconds.
    #[serde(default)]
    pub dtime: i64,
    /// Seconds until the automatic cleanup removes it.
    #[serde(default)]
    pub remaining_del_time: i64,
    /// Who deleted it, when listing the users' bins.
    #[serde(default)]
    pub uname: String,
}

/// A share or user in a cleanup plan.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanTarget {
    /// Id.
    #[serde(default)]
    pub id: i64,
    /// Name.
    #[serde(default)]
    pub name: String,
}

/// A scheduled cleanup plan.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plan {
    /// Id.
    #[serde(default)]
    pub id: i64,
    /// A label, empty for the generated one.
    #[serde(default)]
    pub task_name: String,
    /// The generated label.
    #[serde(default)]
    pub default_task_name: String,
    /// `1` automatic: delete what is older than the retention; `2` timed.
    #[serde(default)]
    pub cleaning_type: i64,
    /// Whether it covers every bin.
    #[serde(default)]
    pub is_select_all: bool,
    /// The shares it covers.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub shares: Vec<PlanTarget>,
    /// The users it covers.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub users: Vec<PlanTarget>,
    /// The first run, Unix seconds.
    #[serde(default)]
    pub first_run_time: i64,
    /// The interval or retention.
    #[serde(default)]
    pub interval: i64,
    /// `4` days, `5` months.
    #[serde(default)]
    pub interval_unit: i64,
    /// Time of day of an automatic cleanup.
    #[serde(default)]
    pub dynamic_task_time: String,
    /// The next run, Unix seconds.
    #[serde(default)]
    pub next_time: i64,
}

/// Bins a plan can still take.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlanChoices {
    /// Shares.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub share: Vec<PlanTarget>,
    /// Users.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub user: Vec<PlanTarget>,
}
