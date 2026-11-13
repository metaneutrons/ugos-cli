//! Docker views: the logic behind the tools that read what the Docker app
//! shows besides the container list.
//!
//! The same operations as `ugos-cli`. Everything here only reads.

use rmcp::schemars;
use serde::Serialize;
use ugos_client::UgosClient;
use ugos_client::error::UgosError;
use ugos_client::ops::dockernet;
use ugos_client::ops::dockerstream;
use ugos_client::ops::dockerupdates::{self, ConfigChange};
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

/// A network by name or ID.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct NetworkParam {
    /// Network name, or its ID or the first characters of it.
    pub name: String,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// The Docker networks.
pub async fn networks(client: &UgosClient) -> String {
    json(dockernet::list(client).await)
}

/// One network with the containers attached to it.
pub async fn network(client: &UgosClient, p: &NetworkParam) -> String {
    json(dockernet::show(client, &p.name).await)
}

/// What is outdated, and the update settings.
pub async fn updates(client: &UgosClient) -> String {
    json(dockerupdates::status(client).await)
}

/// A change to the image update settings.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct UpdatesSetParam {
    /// Remind when an update exists.
    #[serde(default)]
    pub reminder: Option<bool>,
    /// Look for updates on a schedule.
    #[serde(default)]
    pub plan: Option<bool>,
    /// Time of day of the schedule, `HH:MM`.
    #[serde(default)]
    pub time: Option<String>,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// Change the image update settings. The days of the schedule stay as they
/// are.
pub async fn updates_set(client: &UgosClient, p: &UpdatesSetParam) -> String {
    let change = ConfigChange {
        reminder: p.reminder,
        plan: p.plan,
        time: p.time.clone(),
    };
    json(dockerupdates::set_config(client, &change).await)
}

const fn default_lines() -> usize {
    100
}

/// The log of a compose project.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ProjectLogsParam {
    /// Project name.
    pub name: String,
    /// Number of lines, counted from the end (default: 100).
    #[serde(default = "default_lines")]
    pub lines: usize,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// The last lines of a compose project's log, every service, each line with
/// the service's name in front.
pub async fn project_logs(client: &UgosClient, p: &ProjectLogsParam) -> String {
    json(
        dockerstream::project_tail(client, &p.name, p.lines)
            .await
            .map(|log| serde_json::json!({ "log": log })),
    )
}
