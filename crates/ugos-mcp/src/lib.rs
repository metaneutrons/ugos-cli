//! MCP server for UGOS NAS operations.
//!
//! Exposes UGOS API operations as MCP tools, allowing AI assistants
//! to manage UGREEN NAS devices programmatically.
//!
//! Supports multiple NAS targets via numbered env vars:
//! - `UGOS_HOST`, `UGOS_USER`, `UGOS_PASSWORD` — single target (or default)
//! - `UGOS_HOST_1`, `UGOS_USER_1`, `UGOS_PASSWORD_1` — numbered targets
//! - `UGOS_NAME_N` — optional friendly name for target N

use std::collections::HashMap;
use std::sync::Arc;

use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    schemars, tool, tool_router,
};
use tokio::sync::OnceCell;
use ugos_client::api::docker::DockerApi;
use ugos_client::api::kvm::KvmApi;
use ugos_client::{Credentials, UgosClient};

// ── Target config ───────────────────────────────────────────────────

/// Configuration for a single UGOS NAS target.
#[derive(Debug, Clone)]
pub struct TargetConfig {
    /// Friendly name (e.g. "picard"). Falls back to host if unset.
    pub name: String,
    /// Hostname or IP.
    pub host: String,
    /// HTTPS port.
    pub port: u16,
    /// Login credentials.
    pub creds: Credentials,
}

/// Parse target configs from environment variables.
///
/// Reads `UGOS_HOST`/`UGOS_USER`/`UGOS_PASSWORD`/`UGOS_PORT`/`UGOS_NAME`
/// for the unnumbered default, then `UGOS_HOST_1` .. `UGOS_HOST_N` for
/// numbered targets.
#[must_use]
pub fn parse_targets_from_env() -> Vec<TargetConfig> {
    let mut targets = Vec::new();

    // Unnumbered default.
    if let Some(t) = read_target("") {
        targets.push(t);
    }

    // Numbered: _1, _2, ...
    for i in 1.. {
        let suffix = format!("_{i}");
        if let Some(t) = read_target(&suffix) {
            targets.push(t);
        } else {
            break;
        }
    }

    targets
}

/// Read a single target from env vars with the given suffix (e.g. "" or "_1").
fn read_target(suffix: &str) -> Option<TargetConfig> {
    let host = std::env::var(format!("UGOS_HOST{suffix}")).ok()?;
    let user = std::env::var(format!("UGOS_USER{suffix}")).ok()?;
    let password = std::env::var(format!("UGOS_PASSWORD{suffix}")).ok()?;
    let port: u16 = std::env::var(format!("UGOS_PORT{suffix}"))
        .unwrap_or_else(|_| "9443".into())
        .parse()
        .unwrap_or(9443);
    let name = std::env::var(format!("UGOS_NAME{suffix}")).unwrap_or_else(|_| host.clone());

    Some(TargetConfig {
        name,
        host,
        port,
        creds: Credentials {
            username: user,
            password,
        },
    })
}

// ── Parameter types ─────────────────────────────────────────────────

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct VmNameParam {
    /// VM display name or UUID.
    name: String,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    target: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct VmStopParam {
    /// VM display name or UUID.
    name: String,
    /// Force shutdown (default: false).
    #[serde(default)]
    force: bool,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    target: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct VmRebootParam {
    /// VM display name or UUID.
    name: String,
    /// Force reboot (default: false).
    #[serde(default)]
    force: bool,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    target: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct SnapshotListParam {
    /// VM display name or UUID.
    vm: String,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    target: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct SnapshotParam {
    /// VM display name or UUID.
    vm: String,
    /// Snapshot name.
    name: String,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    target: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct SnapshotRenameParam {
    /// VM display name or UUID.
    vm: String,
    /// Current snapshot name.
    old_name: String,
    /// New snapshot name.
    new_name: String,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    target: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct NetworkNameParam {
    /// Network name.
    name: String,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    target: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct TargetOnlyParam {
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    target: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct ImageDeleteParam {
    /// Image file name (e.g. "CachyOS.iso").
    file_name: String,
    /// Image display name (e.g. `CachyOS`).
    image_name: String,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    target: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct ImageNameParam {
    /// Image name.
    name: String,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    target: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct StorageRefParam {
    /// Storage volume name.
    name: String,
    /// Storage volume UUID.
    uuid: String,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    target: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct VncGenerateParam {
    /// VM display name or UUID.
    vm: String,
    /// Base URL for the noVNC link.
    #[serde(default)]
    source_url: String,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    target: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct LogSearchParam {
    /// Page number (default: 1).
    #[serde(default = "default_page")]
    page: u32,
    /// Page size (default: 20).
    #[serde(default = "default_page_size")]
    page_size: u32,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    target: Option<String>,
}

const fn default_page() -> u32 {
    1
}
const fn default_page_size() -> u32 {
    20
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct VmSpecParam {
    /// Full VM specification as a JSON object.
    spec: serde_json::Value,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    target: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct OvaExportParam {
    /// VM display name or UUID.
    vm: String,
    /// Storage volume name.
    storage_name: String,
    /// Storage volume UUID.
    storage_uuid: String,
    /// Output OVA file path on the NAS.
    ova_path: String,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    target: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct OvaParseParam {
    /// OVA file path on the NAS.
    ova_path: String,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    target: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct ContainerIdParam {
    /// Container ID.
    id: String,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    target: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct DockerImageSearchParam {
    /// Image name to search for.
    name: String,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    target: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct DockerImagePullParam {
    /// Image name (e.g. "nginx").
    image: String,
    /// Image tag (default: "latest").
    #[serde(default = "default_tag")]
    tag: String,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    target: Option<String>,
}

const fn default_tag_str() -> &'static str {
    "latest"
}
fn default_tag() -> String {
    default_tag_str().to_owned()
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct ContainerLogsParam {
    /// Container ID.
    id: String,
    /// Number of log lines (default: 100).
    #[serde(default = "default_log_lines")]
    lines: u32,
    /// Target NAS.
    #[serde(default)]
    target: Option<String>,
}
const fn default_log_lines() -> u32 {
    100
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct ContainerCloneParam {
    /// Source container ID.
    id: String,
    /// New container name.
    name: String,
    /// Target NAS.
    #[serde(default)]
    target: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct ContainerBatchParam {
    /// Container IDs.
    ids: Vec<String>,
    /// Operation: start, stop, restart, remove.
    action: String,
    /// Target NAS.
    #[serde(default)]
    target: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct ImageExportParam {
    /// Image ID.
    id: String,
    /// Destination path on the NAS.
    path: String,
    /// Target NAS.
    #[serde(default)]
    target: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct ImageLoadParam {
    /// URL or NAS path to load image from.
    source: String,
    /// Target NAS.
    #[serde(default)]
    target: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct MirrorAddParam {
    /// Display name.
    alias: String,
    /// Mirror URL.
    address: String,
    /// Target NAS.
    #[serde(default)]
    target: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct MirrorIdParam {
    /// Mirror ID.
    id: i64,
    /// Target NAS.
    #[serde(default)]
    target: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct ComposeParam {
    /// Compose project name.
    project: String,
    /// Target NAS.
    #[serde(default)]
    target: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct ProxySetParam {
    /// Proxy configuration JSON object.
    proxy: serde_json::Value,
    /// Target NAS.
    #[serde(default)]
    target: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct NetworkCreateParam {
    /// Network name.
    name: String,
    /// Network type: bridge, nat, none.
    #[serde(default = "default_bridge")]
    net_type: String,
    /// Mapping network interface.
    interface: String,
    /// Target NAS.
    #[serde(default)]
    target: Option<String>,
}
fn default_bridge() -> String {
    "bridge".to_owned()
}

// ── Server ──────────────────────────────────────────────────────────

/// MCP server exposing UGOS NAS operations as tools.
#[derive(Debug, Clone)]
pub struct UgosMcp {
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
    targets: Vec<TargetConfig>,
    clients: Arc<HashMap<String, OnceCell<UgosClient>>>,
}

impl UgosMcp {
    /// Create a new MCP server from a list of target configs.
    ///
    /// # Panics
    ///
    /// Panics if `targets` is empty.
    #[must_use]
    pub fn new(targets: Vec<TargetConfig>) -> Self {
        assert!(!targets.is_empty(), "at least one target required");
        let clients = targets
            .iter()
            .map(|t| (t.name.clone(), OnceCell::new()))
            .collect();
        Self {
            tool_router: Self::tool_router(),
            targets,
            clients: Arc::new(clients),
        }
    }

    /// Resolve which target to use and return a connected client.
    async fn client(&self, target: Option<&str>) -> Result<&UgosClient, String> {
        let cfg = self.resolve_target(target)?;
        let cell = self
            .clients
            .get(&cfg.name)
            .ok_or("internal: missing client cell")?;
        cell.get_or_try_init(|| async {
            UgosClient::connect(&cfg.host, cfg.port, cfg.creds.clone())
                .await
                .map_err(|e| format!("auth failed for {}: {e}", cfg.name))
        })
        .await
    }

    /// Find the target config by name or host. If only one target, use it as default.
    fn resolve_target(&self, target: Option<&str>) -> Result<&TargetConfig, String> {
        match target {
            None | Some("") => {
                if self.targets.len() == 1 {
                    Ok(&self.targets[0])
                } else {
                    let names: Vec<&str> = self.targets.iter().map(|t| t.name.as_str()).collect();
                    Err(format!(
                        "multiple targets configured, specify one of: {}",
                        names.join(", ")
                    ))
                }
            }
            Some(sel) => self
                .targets
                .iter()
                .find(|t| t.name.eq_ignore_ascii_case(sel) || t.host == sel)
                .ok_or_else(|| {
                    let names: Vec<&str> = self.targets.iter().map(|t| t.name.as_str()).collect();
                    format!("unknown target '{sel}', available: {}", names.join(", "))
                }),
        }
    }
}

#[tool_router]
impl UgosMcp {
    #[tool(
        description = "List available UGOS NAS targets. Call this first to discover target names for use in other tools."
    )]
    fn ugos_target_list(&self) -> String {
        let list: Vec<serde_json::Value> = self
            .targets
            .iter()
            .map(|t| serde_json::json!({"name": t.name, "host": t.host, "port": t.port}))
            .collect();
        serde_json::to_string_pretty(&list).unwrap_or_default()
    }

    #[tool(description = "List all virtual machines on a UGOS NAS")]
    async fn ugos_vm_list(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.vm_list().await {
                Ok(vms) => serde_json::to_string_pretty(&vms).unwrap_or_default(),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Show detailed VM configuration")]
    async fn ugos_vm_show(&self, Parameters(p): Parameters<VmNameParam>) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.vm_show(&p.name).await {
                Ok(d) => serde_json::to_string_pretty(&d).unwrap_or_default(),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Power on a virtual machine")]
    async fn ugos_vm_start(&self, Parameters(p): Parameters<VmNameParam>) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.vm_start(&p.name).await {
                Ok(()) => format!("Started {}", p.name),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Shut down a virtual machine (graceful or forced)")]
    async fn ugos_vm_stop(&self, Parameters(p): Parameters<VmStopParam>) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.vm_stop(&p.name, p.force).await {
                Ok(()) => format!("Stopped {}", p.name),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Reboot a virtual machine (graceful or forced)")]
    async fn ugos_vm_reboot(&self, Parameters(p): Parameters<VmRebootParam>) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.vm_reboot(&p.name, p.force).await {
                Ok(()) => format!("Rebooted {}", p.name),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Delete a virtual machine")]
    async fn ugos_vm_delete(&self, Parameters(p): Parameters<VmNameParam>) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.vm_delete(&p.name).await {
                Ok(()) => format!("Deleted {}", p.name),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "List snapshots for a virtual machine")]
    async fn ugos_snapshot_list(&self, Parameters(p): Parameters<SnapshotListParam>) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.snapshot_list(&p.vm).await {
                Ok(s) => serde_json::to_string_pretty(&s).unwrap_or_default(),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Create a snapshot of a virtual machine")]
    async fn ugos_snapshot_create(&self, Parameters(p): Parameters<SnapshotParam>) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.snapshot_create(&p.vm, &p.name).await {
                Ok(()) => format!("Created snapshot {}", p.name),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Delete a snapshot")]
    async fn ugos_snapshot_delete(&self, Parameters(p): Parameters<SnapshotParam>) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.snapshot_delete(&p.vm, &p.name).await {
                Ok(()) => format!("Deleted snapshot {}", p.name),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Revert a virtual machine to a snapshot")]
    async fn ugos_snapshot_revert(&self, Parameters(p): Parameters<SnapshotParam>) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.snapshot_revert(&p.vm, &p.name).await {
                Ok(()) => format!("Reverted to snapshot {}", p.name),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Rename a snapshot")]
    async fn ugos_snapshot_rename(&self, Parameters(p): Parameters<SnapshotRenameParam>) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.snapshot_rename(&p.vm, &p.old_name, &p.new_name).await {
                Ok(()) => format!("Renamed {} → {}", p.old_name, p.new_name),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "List KVM networks")]
    async fn ugos_network_list(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.network_list().await {
                Ok(n) => serde_json::to_string_pretty(&n).unwrap_or_default(),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Show KVM network details")]
    async fn ugos_network_show(&self, Parameters(p): Parameters<NetworkNameParam>) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.network_show(&p.name).await {
                Ok(d) => serde_json::to_string_pretty(&d).unwrap_or_default(),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "List KVM storage volumes")]
    async fn ugos_storage_list(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.storage_list().await {
                Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_default(),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "List ISO/disk images available for KVM")]
    async fn ugos_image_list(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.image_list().await {
                Ok(i) => serde_json::to_string_pretty(&i).unwrap_or_default(),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Show NAS host hardware info (CPU cores, memory)")]
    async fn ugos_host_info(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.host_info().await {
                Ok(i) => serde_json::to_string_pretty(&i).unwrap_or_default(),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    // ── USB ─────────────────────────────────────────────────────────

    #[tool(description = "List USB devices available to a VM")]
    async fn ugos_usb_list(&self, Parameters(p): Parameters<SnapshotListParam>) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.usb_list(&p.vm).await {
                Ok(d) => serde_json::to_string_pretty(&d).unwrap_or_default(),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    // ── Image ops ───────────────────────────────────────────────────

    #[tool(description = "Delete an ISO/disk image")]
    async fn ugos_image_delete(&self, Parameters(p): Parameters<ImageDeleteParam>) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.image_delete(&p.file_name, &p.image_name).await {
                Ok(()) => format!("Deleted image {}", p.image_name),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Check which VMs use an image")]
    async fn ugos_image_usage(&self, Parameters(p): Parameters<ImageNameParam>) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.image_check_usage(&p.name).await {
                Ok(vms) => serde_json::to_string_pretty(&vms).unwrap_or_default(),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    // ── Storage ops ─────────────────────────────────────────────────

    #[tool(description = "Check which VMs use a storage volume")]
    async fn ugos_storage_usage(&self, Parameters(p): Parameters<StorageRefParam>) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.storage_check_usage(&p.name, &p.uuid).await {
                Ok(vms) => serde_json::to_string_pretty(&vms).unwrap_or_default(),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Add a storage volume to KVM")]
    async fn ugos_storage_add(&self, Parameters(p): Parameters<StorageRefParam>) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.storage_add(&p.name, &p.uuid).await {
                Ok(()) => format!("Added storage {}", p.name),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Remove a storage volume from KVM")]
    async fn ugos_storage_delete(&self, Parameters(p): Parameters<StorageRefParam>) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.storage_delete(&p.name, &p.uuid).await {
                Ok(()) => format!("Deleted storage {}", p.name),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    // ── Network ops ─────────────────────────────────────────────────

    #[tool(description = "Delete a KVM network")]
    async fn ugos_network_delete(&self, Parameters(p): Parameters<NetworkNameParam>) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.network_delete(&p.name).await {
                Ok(()) => format!("Deleted network {}", p.name),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    // ── VNC ─────────────────────────────────────────────────────────

    #[tool(description = "List VNC links for a VM")]
    async fn ugos_vnc_list(&self, Parameters(p): Parameters<SnapshotListParam>) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.vnc_list(&p.vm).await {
                Ok(l) => serde_json::to_string_pretty(&l).unwrap_or_default(),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Generate a noVNC link to access a VM console in the browser")]
    async fn ugos_vnc_generate(&self, Parameters(p): Parameters<VncGenerateParam>) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.vnc_generate(&p.vm, &p.source_url).await {
                Ok(link) => link,
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    // ── Logs ────────────────────────────────────────────────────────

    #[tool(description = "Search KVM audit logs")]
    async fn ugos_log_search(&self, Parameters(p): Parameters<LogSearchParam>) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.log_search(p.page, p.page_size).await {
                Ok(l) => serde_json::to_string_pretty(&l).unwrap_or_default(),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "List all operator usernames from KVM logs")]
    async fn ugos_log_operators(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.log_operators().await {
                Ok(ops) => serde_json::to_string_pretty(&ops).unwrap_or_default(),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    // ── VM create/update ────────────────────────────────────────────

    #[tool(
        description = "Create a new VM from a full VM spec (JSON object matching VmDetail schema). Use ugos_vm_show to see an example spec."
    )]
    async fn ugos_vm_create(&self, Parameters(p): Parameters<VmSpecParam>) -> String {
        let spec: ugos_client::types::kvm::VmDetail = match serde_json::from_value(p.spec) {
            Ok(s) => s,
            Err(e) => return format!("error parsing spec: {e}"),
        };
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.vm_create(&spec).await {
                Ok(uuid) => format!("Created VM {uuid}"),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(
        description = "Update an existing VM (must be shut off). Takes a full VM spec (JSON object matching VmDetail schema)."
    )]
    async fn ugos_vm_update(&self, Parameters(p): Parameters<VmSpecParam>) -> String {
        let spec: ugos_client::types::kvm::VmDetail = match serde_json::from_value(p.spec) {
            Ok(s) => s,
            Err(e) => return format!("error parsing spec: {e}"),
        };
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.vm_update(&spec).await {
                Ok(()) => format!("Updated VM {}", spec.virtual_machine_display_name),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    // ── OVA ─────────────────────────────────────────────────────────

    #[tool(description = "Export a VM as an OVA file to a path on the NAS")]
    async fn ugos_ova_export(&self, Parameters(p): Parameters<OvaExportParam>) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c
                .ova_export(&p.vm, &p.storage_name, &p.storage_uuid, &p.ova_path)
                .await
            {
                Ok(()) => format!("Exported {} to {}", p.vm, p.ova_path),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(
        description = "Parse an OVA file on the NAS and return the VM configuration it contains"
    )]
    async fn ugos_ova_parse(&self, Parameters(p): Parameters<OvaParseParam>) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.ova_parse(&p.ova_path).await {
                Ok(d) => serde_json::to_string_pretty(&d).unwrap_or_default(),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    // ── Docker ──────────────────────────────────────────────────────

    #[tool(description = "Get Docker engine overview (container/image counts, CPU/memory usage)")]
    async fn ugos_docker_overview(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.docker_overview().await {
                Ok(o) => serde_json::to_string_pretty(&o).unwrap_or_default(),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Get Docker engine status (online/offline)")]
    async fn ugos_docker_status(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.docker_engine_status().await {
                Ok(s) => s,
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "List Docker containers")]
    async fn ugos_docker_ps(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.container_list(1, 100).await {
                Ok(r) => serde_json::to_string_pretty(&r).unwrap_or_default(),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Start a Docker container")]
    async fn ugos_docker_start(&self, Parameters(p): Parameters<ContainerIdParam>) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.container_start(&p.id).await {
                Ok(()) => format!("Started {}", p.id),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Show detailed Docker container configuration")]
    async fn ugos_docker_show(&self, Parameters(p): Parameters<ContainerIdParam>) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.container_show(&p.id).await {
                Ok(d) => serde_json::to_string_pretty(&d).unwrap_or_default(),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(
        description = "Create a Docker container from a spec (JSON object matching ContainerDetail schema). Use ugos_docker_show to get an example spec."
    )]
    async fn ugos_docker_create(&self, Parameters(p): Parameters<VmSpecParam>) -> String {
        let spec: ugos_client::types::docker::ContainerDetail = match serde_json::from_value(p.spec)
        {
            Ok(s) => s,
            Err(e) => return format!("error parsing spec: {e}"),
        };
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.container_create(&spec).await {
                Ok(()) => format!("Created container {}", spec.container_name),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Stop a Docker container")]
    async fn ugos_docker_stop(&self, Parameters(p): Parameters<ContainerIdParam>) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.container_stop(&p.id).await {
                Ok(()) => format!("Stopped {}", p.id),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Restart a Docker container")]
    async fn ugos_docker_restart(&self, Parameters(p): Parameters<ContainerIdParam>) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.container_restart(&p.id).await {
                Ok(()) => format!("Restarted {}", p.id),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Remove a Docker container")]
    async fn ugos_docker_rm(&self, Parameters(p): Parameters<ContainerIdParam>) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.container_remove(&p.id).await {
                Ok(()) => format!("Removed {}", p.id),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "List local Docker images")]
    async fn ugos_docker_images(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.docker_image_list(1, 100).await {
                Ok(r) => serde_json::to_string_pretty(&r).unwrap_or_default(),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Search Docker Hub for images")]
    async fn ugos_docker_search(
        &self,
        Parameters(p): Parameters<DockerImageSearchParam>,
    ) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.docker_image_search(&p.name, 1, 20).await {
                Ok(r) => serde_json::to_string_pretty(&r).unwrap_or_default(),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Pull a Docker image from a registry")]
    async fn ugos_docker_pull(&self, Parameters(p): Parameters<DockerImagePullParam>) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.docker_image_download(&p.image, &p.tag).await {
                Ok(()) => format!("Pulling {}:{}", p.image, p.tag),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "List Docker registry mirror sources")]
    async fn ugos_docker_mirrors(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.mirror_list().await {
                Ok(m) => serde_json::to_string_pretty(&m).unwrap_or_default(),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Get Docker container logs")]
    async fn ugos_docker_logs(&self, Parameters(p): Parameters<ContainerLogsParam>) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.container_logs(&p.id, p.lines).await {
                Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_default(),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Clone a Docker container")]
    async fn ugos_docker_clone(&self, Parameters(p): Parameters<ContainerCloneParam>) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.container_clone(&p.id, &p.name).await {
                Ok(()) => format!("Cloned {} as {}", p.id, p.name),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Batch operate on Docker containers (start/stop/restart/remove)")]
    async fn ugos_docker_batch(&self, Parameters(p): Parameters<ContainerBatchParam>) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.container_batch(&p.ids, &p.action).await {
                Ok(()) => format!("{} {} containers", p.action, p.ids.len()),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Export a Docker image to a NAS path")]
    async fn ugos_docker_image_export(
        &self,
        Parameters(p): Parameters<ImageExportParam>,
    ) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.docker_image_export(&p.id, &p.path).await {
                Ok(()) => format!("Exporting image {} to {}", p.id, p.path),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Load a Docker image from a URL")]
    async fn ugos_docker_image_load_url(
        &self,
        Parameters(p): Parameters<ImageLoadParam>,
    ) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.docker_image_load_url(&p.source).await {
                Ok(()) => format!("Loading image from {}", p.source),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Load a Docker image from a NAS path")]
    async fn ugos_docker_image_load_path(
        &self,
        Parameters(p): Parameters<ImageLoadParam>,
    ) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.docker_image_load_path(&p.source).await {
                Ok(()) => format!("Loading image from {}", p.source),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Add a Docker registry mirror source")]
    async fn ugos_docker_mirror_add(&self, Parameters(p): Parameters<MirrorAddParam>) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.mirror_add(&p.alias, &p.address).await {
                Ok(()) => format!("Added mirror {}", p.alias),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Delete a Docker registry mirror source")]
    async fn ugos_docker_mirror_delete(&self, Parameters(p): Parameters<MirrorIdParam>) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.mirror_delete(p.id).await {
                Ok(()) => format!("Deleted mirror {}", p.id),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Switch active Docker registry mirror source")]
    async fn ugos_docker_mirror_switch(&self, Parameters(p): Parameters<MirrorIdParam>) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.mirror_switch(p.id).await {
                Ok(()) => format!("Switched to mirror {}", p.id),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "List containers in a Docker Compose project")]
    async fn ugos_docker_compose(&self, Parameters(p): Parameters<ComposeParam>) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.compose_containers(&p.project).await {
                Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_default(),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Get Docker HTTP proxy configuration")]
    async fn ugos_docker_proxy_get(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.docker_proxy_get().await {
                Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_default(),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Set Docker HTTP proxy configuration")]
    async fn ugos_docker_proxy_set(&self, Parameters(p): Parameters<ProxySetParam>) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.docker_proxy_set(&p.proxy).await {
                Ok(()) => "Updated HTTP proxy".into(),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    // ── KVM Network Create/Update ───────────────────────────────────

    #[tool(description = "Create a KVM network")]
    async fn ugos_network_create(&self, Parameters(p): Parameters<NetworkCreateParam>) -> String {
        match self.client(p.target.as_deref()).await {
            Ok(c) => {
                let net = ugos_client::types::kvm::NetworkDetail {
                    network_name: p.name.clone(),
                    network_type: p.net_type.clone(),
                    network_mode: p.net_type.clone(),
                    mapping_network: p.interface.clone(),
                    ..Default::default()
                };
                match c.network_create(&net).await {
                    Ok(()) => format!("Created network {}", p.name),
                    Err(e) => format!("error: {e}"),
                }
            }
            Err(e) => e,
        }
    }

    #[tool(description = "Update a KVM network")]
    async fn ugos_network_update(&self, Parameters(p): Parameters<VmSpecParam>) -> String {
        let net: ugos_client::types::kvm::NetworkDetail = match serde_json::from_value(p.spec) {
            Ok(s) => s,
            Err(e) => return format!("error parsing spec: {e}"),
        };
        match self.client(p.target.as_deref()).await {
            Ok(c) => match c.network_update(&net).await {
                Ok(()) => format!("Updated network {}", net.network_name),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }
}

impl ServerHandler for UgosMcp {
    fn get_info(&self) -> ServerInfo {
        let target_names: Vec<&str> = self.targets.iter().map(|t| t.name.as_str()).collect();
        let instructions = if self.targets.len() == 1 {
            format!(
                "UGOS NAS management server. Connected target: {}. \
                 Manages VMs, snapshots, networks, storage, and images.",
                target_names[0]
            )
        } else {
            format!(
                "UGOS NAS management server with {} targets: {}. \
                 Call ugos_target_list first to see available targets. \
                 All tools require a 'target' parameter to select which NAS to operate on. \
                 Manages VMs, snapshots, networks, storage, and images.",
                self.targets.len(),
                target_names.join(", ")
            )
        };
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions(instructions)
    }
}
