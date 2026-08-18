//! Types for the Download Center.
//!
//! Like the rest of the core API these are `snake_case`, and every field
//! defaults: a task in flight reports different keys than a finished one.

use serde::{Deserialize, Serialize};

/// A download task, in progress or finished.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DownloadTask {
    /// Numeric id within the list.
    #[serde(default)]
    pub id: i64,
    /// Stable task identifier, used to pause, resume or delete it.
    #[serde(default)]
    pub task_id: String,
    /// File name on disk.
    #[serde(default)]
    pub download_file_name: String,
    /// Source URL.
    #[serde(default)]
    pub download_url: String,
    /// Target directory.
    #[serde(default)]
    pub save_dir: String,
    /// Total size in bytes.
    #[serde(default)]
    pub total_size: i64,
    /// Bytes fetched so far. Running tasks call this `downloaded_size`;
    /// finished ones omit it.
    #[serde(default)]
    pub downloaded_size: i64,
    /// Current rate in bytes/s.
    #[serde(default)]
    pub download_speed: i64,
    /// Task state as UGOS reports it.
    #[serde(default)]
    pub task_status: i64,
    /// Non-zero when the task failed; 9 appears for an unreachable URL.
    #[serde(default)]
    pub error_code: i64,
    /// Progress in percent.
    #[serde(default)]
    pub plan: i64,
    /// Estimated seconds remaining.
    #[serde(default)]
    pub remaining_time: i64,
    /// Creation time, ISO 8601, on running tasks.
    #[serde(default)]
    pub created_at: String,
    /// Completion time as a unix timestamp.
    #[serde(default)]
    pub download_completed_time: i64,
    /// Owner.
    #[serde(default)]
    pub uname: String,
}

/// Aggregate rates and counts from `globalSpeed`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DownloadSpeed {
    /// Combined download rate in bytes/s.
    #[serde(default)]
    pub download_speed: i64,
    /// Combined upload rate in bytes/s.
    #[serde(default)]
    pub upload_speed: i64,
    /// Tasks in progress.
    #[serde(default)]
    pub downloading_num: i64,
    /// Tasks finished.
    #[serde(default)]
    pub completed_num: i64,
}

/// The configured target directory.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DownloadPath {
    /// Absolute path on the NAS.
    #[serde(default)]
    pub path: String,
    /// The same path as shown in the web UI.
    #[serde(default)]
    pub path_display: String,
    /// Whether the path currently exists.
    #[serde(default)]
    pub path_is_validity: bool,
    /// Free space in bytes.
    #[serde(default)]
    pub available_size: i64,
}
