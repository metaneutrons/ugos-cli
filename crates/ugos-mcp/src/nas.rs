//! Administration of the NAS itself: storage, and the areas that follow.
//!
//! Each function answers a tool call with JSON, or with the error worded as
//! the other tools do. The checks that belong to a change live in
//! `ugos_client::ops`, as they do for the CLI.

use rmcp::schemars;
use serde::Serialize;
use ugos_client::UgosClient;
use ugos_client::api::storage::{StorageApi, pick_pool, pick_volume};
use ugos_client::error::UgosError;

/// A name, device path or other handle of one thing.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct NameParam {
    /// What to look at: a name, label or id, as the matching list tool shows it.
    pub name: String,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

fn json<T: Serialize>(result: Result<T, UgosError>) -> String {
    match result {
        Ok(value) => serde_json::to_string_pretty(&value).unwrap_or_default(),
        Err(e) => crate::manage::failure(&e),
    }
}

// ── Storage ─────────────────────────────────────────────────────────

pub async fn pool_list(client: &UgosClient) -> String {
    json(client.pools().await)
}

pub async fn pool_show(client: &UgosClient, name: &str) -> String {
    json(
        client
            .pools()
            .await
            .and_then(|pools| pick_pool(&pools, name)),
    )
}

pub async fn volume_list(client: &UgosClient) -> String {
    json(client.volumes().await)
}

pub async fn volume_show(client: &UgosClient, name: &str) -> String {
    json(
        client
            .volumes()
            .await
            .and_then(|volumes| pick_volume(&volumes, name)),
    )
}

pub async fn disk_list(client: &UgosClient) -> String {
    json(client.disks().await)
}

pub async fn disk_show(client: &UgosClient, name: &str) -> String {
    let both = async {
        let disk = client.disk_find(name).await?;
        let attribute = client.disk_attribute(&disk.dev_name).await?;
        Ok(serde_json::json!({ "disk": disk, "attribute": attribute }))
    };
    json(both.await)
}

pub async fn disk_smart(client: &UgosClient, name: &str) -> String {
    let smart = async {
        let disk = client.disk_find(name).await?;
        client.disk_smart(&disk.dev_name).await
    };
    json(smart.await)
}

pub async fn disk_history(client: &UgosClient, name: &str) -> String {
    let history = async {
        let disk = client.disk_find(name).await?;
        client.disk_history(&disk.dev_name).await
    };
    json(history.await)
}

pub async fn disk_plans(client: &UgosClient) -> String {
    json(client.disk_plans().await)
}
