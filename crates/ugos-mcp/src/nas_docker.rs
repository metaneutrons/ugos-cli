//! Docker views: the logic behind the tools that read what the Docker app
//! shows besides the container list.
//!
//! The same operations as `ugos-cli`. Everything here only reads.

use rmcp::schemars;
use serde::Serialize;
use ugos_client::UgosClient;
use ugos_client::error::UgosError;
use ugos_client::ops::dockerview::{self, OpLogQuery};

use crate::manage::failure;

fn json<T: Serialize>(result: Result<T, UgosError>) -> String {
    match result {
        Ok(value) => serde_json::to_string_pretty(&value).unwrap_or_default(),
        Err(e) => failure(&e),
    }
}

/// The containers with CPU, memory, traffic, ports and whether an update
/// exists.
pub async fn containers(client: &UgosClient) -> String {
    let views = dockerview::containers(client, 1, 100).await;
    json(views)
}

/// The last lines of a container's log.
pub async fn log(client: &UgosClient, id: &str, lines: usize) -> String {
    json(
        dockerview::container_log(client, id, lines)
            .await
            .map(|(log, cut_by_ugos)| serde_json::json!({"log": log, "cut_by_ugos": cut_by_ugos})),
    )
}

/// What to look for in the log of the Docker app.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct OplogParam {
    /// Only this user (`system` is the NAS itself).
    #[serde(default)]
    pub user: Option<String>,
    /// Only failures.
    #[serde(default)]
    pub failed: bool,
    /// Only entries that contain this text.
    #[serde(default)]
    pub contains: Option<String>,
    /// Oldest first (default: newest first).
    #[serde(default)]
    pub oldest_first: bool,
    /// Entries per page (default: 50).
    #[serde(default)]
    pub limit: Option<u32>,
    /// Page, from 1 (default: 1).
    #[serde(default)]
    pub page: Option<u32>,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// A page of the log of the Docker app.
pub async fn oplog(client: &UgosClient, p: &OplogParam) -> String {
    let query = OpLogQuery {
        operator: p.user.clone(),
        failures_only: p.failed,
        contains: p.contains.clone(),
        oldest_first: p.oldest_first,
    };
    json(
        dockerview::oplog(client, &query, p.page.unwrap_or(1), p.limit.unwrap_or(50))
            .await
            .map(|(entries, total)| serde_json::json!({"total": total, "entries": entries})),
    )
}

/// The users that appear in the log of the Docker app.
pub async fn oplog_users(client: &UgosClient) -> String {
    json(dockerview::oplog_users(client).await)
}
