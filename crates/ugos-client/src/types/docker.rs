//! Docker container and image management types.

use serde::{Deserialize, Serialize};

// ── Overview ────────────────────────────────────────────────────────

/// Docker engine overview from `ObtainOverviewInfo`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerOverview {
    /// Total container count.
    pub container_count: i64,
    /// Running container count.
    pub run_container_count: i64,
    /// Total image count.
    pub image_count: i64,
    /// Host memory used in bytes.
    pub memory_used: i64,
    /// Host total memory in bytes.
    pub total_memory: i64,
    /// Container memory used in bytes.
    pub container_memory: i64,
    /// Host CPU usage percentage.
    pub cpu_used: i64,
    /// Container CPU usage percentage.
    pub container_cpu_used: i64,
    /// Docker engine running.
    pub status: bool,
    /// Compose project count.
    #[serde(default)]
    pub project_counr: i64,
    /// Running compose project count.
    #[serde(default)]
    pub run_project_counr: i64,
}

// ── Container ───────────────────────────────────────────────────────

/// Container summary from `ContainerListV2`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Container {
    /// Container ID.
    #[serde(default)]
    pub container_id: String,
    /// Container name.
    #[serde(default)]
    pub container_name: String,
    /// Image name.
    #[serde(default)]
    pub image: String,
    /// Container status (e.g. "running", "exited").
    #[serde(default)]
    pub status: String,
    /// Container state.
    #[serde(default)]
    pub state: String,
    /// CPU usage percentage.
    #[serde(default)]
    pub cpu_percent: f64,
    /// Memory usage in bytes.
    #[serde(default)]
    pub memory_usage: i64,
    /// Memory limit in bytes.
    #[serde(default)]
    pub memory_limit: i64,
    /// Creation timestamp.
    #[serde(default)]
    pub created: i64,
    /// Port mappings.
    #[serde(default)]
    pub ports: Vec<serde_json::Value>,
    /// All other fields.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// Paginated container list response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContainerPage {
    /// Total containers (unfiltered).
    #[serde(default)]
    pub original_total: i64,
    /// Containers on this page.
    #[serde(default)]
    pub result: Option<Vec<Container>>,
    /// Total containers (filtered).
    #[serde(default)]
    pub total: i64,
}

// ── Image ───────────────────────────────────────────────────────────

/// Docker image from `ShowLocalImageV2`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerImage {
    /// Image ID.
    #[serde(default)]
    pub id: String,
    /// Repository name.
    #[serde(default)]
    pub repository: String,
    /// Tag.
    #[serde(default)]
    pub tag: String,
    /// Image size in bytes.
    #[serde(default)]
    pub size: i64,
    /// Creation timestamp.
    #[serde(default)]
    pub created: i64,
    /// All other fields.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// Paginated image list response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImagePage {
    /// Total images.
    #[serde(default)]
    pub original_total: i64,
    /// Images on this page.
    #[serde(default)]
    pub result: Option<Vec<DockerImage>>,
}

// ── Registry ────────────────────────────────────────────────────────

/// Registry mirror source from `ShowMirrorList`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mirror {
    /// Mirror ID.
    pub id: i64,
    /// Display name.
    pub alias: String,
    /// Mirror URL.
    pub address: String,
    /// Whether this is Docker Hub.
    #[serde(default)]
    pub is_dockerhub: bool,
    /// Whether this mirror is active.
    #[serde(default)]
    pub status: bool,
}
