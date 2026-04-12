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
    /// Container name.
    #[serde(default)]
    pub name: String,
    /// Container ID.
    #[serde(default)]
    pub container_id: String,
    /// Image ID.
    #[serde(default)]
    pub image_id: String,
    /// Image name.
    #[serde(default)]
    pub image_name: String,
    /// Image version/tag.
    #[serde(default)]
    pub version: String,
    /// Container status (e.g. "running", "exited").
    #[serde(default)]
    pub status: String,
    /// Compose project name (empty if standalone).
    #[serde(default)]
    pub project_name: String,
    /// Creation timestamp.
    #[serde(default)]
    pub create_at: i64,
    /// Application label.
    #[serde(default)]
    pub application: String,
}

/// Detailed container configuration from `GetContainerById`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::struct_excessive_bools)]
pub struct ContainerDetail {
    /// Image name.
    #[serde(default)]
    pub image_name: String,
    /// Image ID.
    #[serde(default)]
    pub image_id: String,
    /// Image version/tag.
    #[serde(default)]
    pub image_version: String,
    /// Full image reference (e.g. "nginx:latest").
    #[serde(default)]
    pub tag: String,
    /// Container name.
    #[serde(default)]
    pub container_name: String,
    /// CPU limit (0 = unlimited).
    #[serde(default)]
    pub cpu_limit: i64,
    /// Memory limit in bytes (0 = unlimited).
    #[serde(default)]
    pub mem_limit: i64,
    /// No resource restrictions.
    #[serde(default)]
    pub no_restrictions: bool,
    /// Network mode (e.g. "bridge", "host").
    #[serde(default)]
    pub network_mode: String,
    /// Hardware acceleration enabled.
    #[serde(default)]
    pub hardware_acceleration: bool,
    /// Privileged mode.
    #[serde(default)]
    pub privileged_mode: bool,
    /// Restart policy.
    #[serde(default)]
    pub abnormal_reset: bool,
    /// Whether the container should run after creation.
    #[serde(default)]
    pub run_container: bool,
    /// Port mappings.
    #[serde(default)]
    pub port_mapping: Vec<serde_json::Value>,
    /// Volume mounts.
    pub volumes: Option<Vec<serde_json::Value>>,
    /// Environment variables.
    #[serde(default)]
    pub environment_variables: Vec<EnvVar>,
    /// Container run command.
    #[serde(default)]
    pub container_run_command: Vec<String>,
    /// Linux capabilities.
    #[serde(default)]
    pub perm_and_func: Vec<String>,
    /// Compose project name.
    #[serde(default)]
    pub project_name: String,
}

/// Environment variable key-value pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVar {
    /// Variable name.
    #[serde(default)]
    pub variable: String,
    /// Variable value (UGOS calls this "price").
    #[serde(default)]
    pub price: String,
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
    /// Image ID (sha256:...).
    #[serde(default)]
    pub image_id: String,
    /// Image reference (e.g. "hello-world:latest").
    #[serde(default)]
    pub image_ref: String,
    /// Image name (e.g. "hello-world").
    #[serde(default)]
    pub image_name: String,
    /// Image size in bytes.
    #[serde(default)]
    pub image_size: i64,
    /// Image tag (e.g. "latest").
    #[serde(default)]
    pub image_version: String,
    /// Pull status (1 = ready).
    #[serde(default)]
    pub status: i64,
    /// Creation timestamp.
    #[serde(default)]
    pub create: i64,
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
