//! Docker container and image management types.

use serde::{Deserialize, Deserializer, Serialize};

/// Accept the UGOS convention of returning either an array or `null` for an
/// optional list field while presenting callers with an ordinary empty vector.
fn null_vec_as_empty<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<Vec<T>>::deserialize(deserializer).map(Option::unwrap_or_default)
}

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
    /// GPU device IDs passed through to the container (empty if none).
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub gpu_ids: Vec<String>,
    /// Bridge/macvlan subnet assignment. Required by the UGOS backend even
    /// for the default `bridge` network — omitting it was not tested and
    /// may be rejected.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub subnet_settings: Vec<SubnetSetting>,
    /// Whether the container should run after creation.
    #[serde(default)]
    pub run_container: bool,
    /// Port mappings.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub port_mapping: Vec<PortMapping>,
    /// Volume mounts.
    pub volumes: Option<Vec<serde_json::Value>>,
    /// Environment variables.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub environment_variables: Vec<EnvVar>,
    /// Container run command.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub container_run_command: Vec<String>,
    /// Linux capabilities. The live API has been observed sending `null`
    /// here (not an empty array) when none are set, hence `Option`.
    #[serde(default)]
    pub perm_and_func: Option<Vec<String>>,
    /// Compose project name.
    #[serde(default)]
    pub project_name: String,
}

/// Network/subnet assignment for a container, sent alongside `network_mode`.
///
/// Live-captured from `CreateContainer`: even the default `bridge` network
/// is sent explicitly as `{networkName: "bridge", subnet: "172.17.0.0/16"}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubnetSetting {
    /// Network name (e.g. "bridge", or a custom/macvlan network name).
    pub network_name: String,
    /// CIDR subnet for this network.
    pub subnet: String,
}

/// A single container port mapping.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortMapping {
    /// Port on the NAS host (the UGOS UI auto-assigns one if left at 0).
    pub nas_port: i64,
    /// Port inside the container.
    pub container_port: i64,
    /// Protocol: "tcp" or "udp".
    pub port_type: String,
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

// ── Compose Project ─────────────────────────────────────────────────

/// Compose project from `GetProjectListV3`/`GetProjectInfoV2`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComposeProject {
    /// Project name.
    #[serde(default)]
    pub name: String,
    /// Storage path on the NAS (e.g. `/volume1/docker/<name>`).
    #[serde(default)]
    pub path: String,
    /// Project status (observed: 1 = up).
    #[serde(default)]
    pub status: i64,
    /// Total container count.
    #[serde(default)]
    pub container_sum: i64,
    /// Running container count.
    #[serde(default)]
    pub run_container_sum: i64,
    /// Whether the compose file is missing from disk.
    #[serde(default)]
    pub config_file_missing: bool,
    /// Creation timestamp.
    #[serde(default)]
    pub create_time: String,
    /// Containers belonging to this project. `None`/`null` briefly right
    /// after creation, before UGOS finishes materializing them.
    #[serde(default)]
    pub container_list: Option<Vec<ComposeProjectContainer>>,
    /// Application label, if the project was created from a template.
    #[serde(default)]
    pub application: String,
    /// Total container count (duplicate of `container_sum` in list responses).
    #[serde(default)]
    pub container_num: i64,
    /// Deployment progress percentage.
    #[serde(default)]
    pub progress: i64,
    /// Whether any image in the project has an available update.
    #[serde(default)]
    pub img_has_update: bool,
}

/// A single container within a [`ComposeProject`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComposeProjectContainer {
    /// Container name (e.g. "re-test-web-1").
    #[serde(default)]
    pub container_name: String,
    /// Container ID.
    #[serde(default)]
    pub container_id: String,
    /// Image name (without tag).
    #[serde(default)]
    pub image_name: String,
    /// Image tag.
    #[serde(default)]
    pub version: String,
    /// Restart policy (e.g. "no").
    #[serde(default)]
    pub restart_policy: String,
}

/// Paginated compose project list response from `GetProjectListV3`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComposeProjectPage {
    /// Total projects (unfiltered).
    #[serde(default)]
    pub original_total: i64,
    /// Projects on this page.
    #[serde(default)]
    pub list: Option<Vec<ComposeProject>>,
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
