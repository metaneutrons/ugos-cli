//! System log and user accounts, from the UGOS core API.

use crate::client::UgosClient;
use crate::error::Result;
use crate::types::syslog::{LogEntry, LogPage, User};

/// Filters accepted by the system log.
#[derive(Debug, Default, Clone)]
pub struct LogFilter<'a> {
    /// Restrict to one module, e.g. `login`.
    pub module: Option<&'a str>,
    /// Restrict to one severity, e.g. `info`.
    pub level: Option<&'a str>,
    /// Restrict to one account.
    pub operator: Option<&'a str>,
    /// Free-text search.
    pub keyword: Option<&'a str>,
    /// Page number, 1-based.
    pub page: u32,
    /// Entries per page.
    pub size: u32,
    /// `1` for the transfer log (Samba, FTP, `WebDAV`, file manager) instead
    /// of the event log.
    pub transfer: bool,
    /// Only entries from this time on, Unix seconds.
    pub from: Option<i64>,
    /// Only entries up to this time, Unix seconds.
    pub to: Option<i64>,
    /// Oldest first.
    pub ascending: bool,
}

/// System log and account listings.
#[allow(clippy::module_name_repetitions)]
pub trait SysLogApi {
    /// Query the system log.
    fn syslog(&self, filter: &LogFilter<'_>) -> impl Future<Output = Result<LogPage>> + Send;
    /// List user accounts.
    fn users(&self) -> impl Future<Output = Result<Vec<User>>> + Send;
    /// The account this session belongs to.
    fn current_user(&self) -> impl Future<Output = Result<User>> + Send;
}

/// Wrapper for the user listing, which nests under `list`.
#[derive(serde::Deserialize)]
struct UserList {
    #[serde(default)]
    list: Vec<User>,
}

impl SysLogApi for UgosClient {
    async fn syslog(&self, filter: &LogFilter<'_>) -> Result<LogPage> {
        // The page size parameter is `size`; `limit` is accepted and ignored,
        // leaving the default of 20.
        let page = filter.page.max(1).to_string();
        let size = if filter.size == 0 { 20 } else { filter.size }.to_string();
        let mut params: Vec<(&str, &str)> = vec![("page", &page), ("size", &size)];
        for (key, value) in [
            ("module", filter.module),
            ("level", filter.level),
            ("operator", filter.operator),
            ("keyword", filter.keyword),
        ] {
            if let Some(v) = value {
                params.push((key, v));
            }
        }
        let from = filter.from.map(|t| t.to_string());
        let to = filter.to.map(|t| t.to_string());
        if let Some(v) = &from {
            params.push(("from_time", v));
        }
        if let Some(v) = &to {
            params.push(("to_time", v));
        }
        params.push(("visualizer", "false"));
        params.push(("order", if filter.ascending { "up" } else { "down" }));
        if filter.transfer {
            params.push(("log_type", "1"));
        }
        self.get_with_params("log/query", &params).await
    }

    async fn users(&self) -> Result<Vec<User>> {
        let resp: UserList = self.get("user/list").await?;
        Ok(resp.list)
    }

    async fn current_user(&self) -> Result<User> {
        self.get("user/current/user").await
    }
}

impl LogPage {
    /// The entries, treating an absent list as empty.
    #[must_use]
    pub fn entries(&self) -> &[LogEntry] {
        self.log_list.as_deref().unwrap_or_default()
    }
}
