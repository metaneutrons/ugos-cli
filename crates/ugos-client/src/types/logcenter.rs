//! Types for the Log Center: archiving, notifications and the archive viewer.

use serde::{Deserialize, Serialize};

use super::common::null_vec_as_empty;

/// The archive policy.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveConfig {
    /// Whether archiving runs.
    #[serde(default)]
    pub status: bool,
    /// Archive when the log data reaches this many MB; `0` is no trigger.
    #[serde(default)]
    pub data_bytes: i64,
    /// Archive when there are this many entries; `0` is no trigger.
    #[serde(default)]
    pub num: i64,
    /// Archive entries older than this many months; `0` is no trigger.
    #[serde(default)]
    pub remain_time: i64,
    /// Where the archives go.
    #[serde(default)]
    pub path: String,
    /// Whether that folder exists.
    #[serde(default)]
    pub path_exist: bool,
}

/// An archive that was made.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveRecord {
    /// The file.
    #[serde(default)]
    pub output_path: String,
    /// When it was made, Unix seconds.
    #[serde(default)]
    pub archive_time: i64,
    /// The first entry in it, Unix seconds.
    #[serde(default)]
    pub start_time: i64,
    /// The last entry in it, Unix seconds.
    #[serde(default)]
    pub end_time: i64,
}

/// A page of archives or notifications.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(bound(deserialize = "T: Deserialize<'de>"))]
pub struct RecordPage<T> {
    /// The items.
    #[serde(default = "Vec::new", deserialize_with = "null_vec_as_empty")]
    pub list: Vec<T>,
    /// The page, from 0.
    #[serde(default)]
    pub cur_page: i64,
    /// How many there are in all.
    #[serde(default)]
    pub total: i64,
}

/// An archive that is open in the viewer, or can be.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveFile {
    /// Id.
    #[serde(default)]
    pub archive_id: i64,
    /// File name.
    #[serde(default)]
    pub file_name: String,
    /// First entry, Unix seconds.
    #[serde(default)]
    pub start_time: i64,
    /// Last entry, Unix seconds.
    #[serde(default)]
    pub end_time: i64,
    /// Whether it is the active log source.
    #[serde(default)]
    pub in_use: bool,
}

/// The notification policy.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotifyConfig {
    /// Whether it runs.
    #[serde(default)]
    pub status: bool,
    /// A keyword or expression; the first one counts.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub keywords: Vec<String>,
    /// `info`, `warning`, `critical`.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub levels: Vec<String>,
    /// Module ids.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub modules: Vec<String>,
    /// User names.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub operators: Vec<String>,
}

/// A notification that was sent.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotifyRecord {
    /// When, Unix seconds.
    #[serde(default)]
    pub notify_time: i64,
    /// The log entry.
    #[serde(default)]
    pub log_content: String,
    /// Its id.
    #[serde(default)]
    pub log_id: String,
    /// `0` event log, `1` transfer log.
    #[serde(default)]
    pub log_type: i64,
}

/// Users and modules that appear in the log.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsersModules {
    /// User names.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub users: Vec<String>,
    /// Module ids.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub modules: Vec<String>,
}
