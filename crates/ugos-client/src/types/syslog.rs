//! Types for the system log and user accounts.

use serde::{Deserialize, Serialize};

use super::common::null_vec_as_empty;

/// One page of system log entries.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LogPage {
    /// The entries on this page.
    #[serde(default)]
    pub log_list: Option<Vec<LogEntry>>,
    /// Total entries matching the filter, across all pages.
    #[serde(default)]
    pub total: i64,
    /// The page this response covers.
    #[serde(default)]
    pub cur_page: i64,
}

/// A system log entry, from any module.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LogEntry {
    /// Message text.
    #[serde(default)]
    pub content: String,
    /// Severity, e.g. `info`.
    #[serde(default)]
    pub level: String,
    /// Originating module, e.g. `login`.
    #[serde(default)]
    pub module: String,
    /// Account that triggered it.
    #[serde(default)]
    pub operator: String,
    /// Unix timestamp.
    #[serde(default)]
    pub create_time: i64,
    /// Stable identifier.
    #[serde(default)]
    pub log_id: String,
}

/// A user account.
///
/// The listing carries far more than this — quota, cloud binding, permission
/// lists — so this models what identifies an account.
// The flags mirror what UGOS reports, one field each.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct User {
    /// Login name.
    #[serde(default)]
    pub username: String,
    /// Numeric user id, e.g. 1000.
    #[serde(default)]
    pub uid: i64,
    /// Free-text description.
    #[serde(default)]
    pub description: String,
    /// Masked email address, as UGOS returns it.
    #[serde(default)]
    pub email: String,
    /// The real address, obscured. `ugos_client::crypto::decrypt_email`
    /// reads it back.
    #[serde(default, skip_serializing)]
    pub email_encrypted: String,
    /// Account type; 0 is a standard user.
    #[serde(default)]
    pub account_type: i64,
    /// Whether the account is disabled.
    #[serde(default)]
    pub disabled: bool,
    /// `admin` or `users`. The account type does not tell them apart.
    #[serde(default)]
    pub role: String,
    /// The groups the account belongs to.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub groups: Vec<GroupRef>,
    /// When the account expires: `-1` never, `1` it has been disabled, else
    /// a Unix time in seconds.
    #[serde(default)]
    pub expire_time: i64,
    /// The expiry date as the web UI shows it, empty when there is none.
    #[serde(default)]
    pub expire_date: String,
    /// Whether the account has expired.
    #[serde(default)]
    pub is_expired: bool,
    /// Whether the account has a home folder.
    #[serde(default)]
    pub enable_home_dir: bool,
    /// Whether the account may not change its own password.
    #[serde(default)]
    pub deny_change_pwd: bool,
    /// Quota of the home folder in `unit`s, 0 for none.
    #[serde(default)]
    pub dir_quota: i64,
    /// Unit of `dir_quota`: 0 MB, 1 GB, 2 TB.
    #[serde(default)]
    pub unit: i64,
}

/// A group an account belongs to.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupRef {
    /// Numeric group id.
    #[serde(default)]
    pub gid: i64,
    /// Group name.
    #[serde(default)]
    pub name: String,
}
