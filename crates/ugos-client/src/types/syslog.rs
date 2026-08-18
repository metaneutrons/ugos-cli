//! Types for the system log and user accounts.

use serde::{Deserialize, Serialize};

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
    /// Account type; 0 is a standard user.
    #[serde(default)]
    pub account_type: i64,
    /// Whether the account is disabled.
    #[serde(default)]
    pub disabled: bool,
}
