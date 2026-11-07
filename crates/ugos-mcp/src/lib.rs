//! MCP server for UGOS NAS operations.
//!
//! Exposes UGOS API operations as MCP tools, allowing AI assistants
//! to manage UGREEN NAS devices programmatically.
//!
//! Supports multiple NAS targets via numbered env vars:
//! - `UGOS_HOST`, `UGOS_USER`, `UGOS_PASSWORD` — single target (or default)
//! - `UGOS_HOST_1`, `UGOS_USER_1`, `UGOS_PASSWORD_1` — numbered targets
//! - `UGOS_NAME_N` — optional friendly name for target N
//! - `UGOS_OTP` / `UGOS_OTP_N` — one-time authenticator code (type 1)
//! - `UGOS_OTP_SECRET` / `UGOS_OTP_SECRET_N` — TOTP secret; a fresh code is
//!   generated on every login, which is what a long-running server needs

use std::collections::HashMap;
use std::sync::Arc;

use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{Implementation, ServerCapabilities, ServerConfig},
    schemars, tool, tool_handler, tool_router,
};
mod manage;
mod nas;
mod nas_files;
mod nas_hw;
mod nas_net;

use tokio::sync::OnceCell;
use ugos_client::api::docker::DockerApi;
use ugos_client::api::kvm::KvmApi;
use ugos_client::{Credentials, TlsPolicy, UgosClient, tls};

use crate::manage::{
    AccountDeleteParam, AccountParam, ShareAclSetParam, ShareCreateParam, ShareDeleteParam,
    ShareParam, ShareUpdateParam, UserUpdateParam,
};
use crate::nas::NameParam;
use crate::nas_files::{
    DiscoveryParam, ForceOnlyParam, ServiceChangeParam, ServiceParam, ServiceStopParam,
};
use crate::nas_hw::{
    BuzzerParam, FanCurveParam, FinderParam, ForceParam, GpuParam, LedParam, ModeParam,
    PositionParam, PowerParam, ScheduleParam, ScreenParam, TerminalParam, TimeParam, UpsParam,
};
use crate::nas_net::{
    BondCreateParam, FlowAddParam, FlowPositionParam, GatewayOrderParam, IfaceForceParam,
    NetDnsParam, NetProxyParam, NetSetParam, PortsParam, WifiParam,
};

// ── Target config ───────────────────────────────────────────────────

/// Configuration for a single UGOS NAS target.
#[derive(Debug, Clone)]
pub struct TargetConfig {
    /// Friendly name (e.g. "nas1"). Falls back to host if unset.
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
/// Reads `UGOS_HOST`/`UGOS_USER`/`UGOS_PASSWORD`/`UGOS_PORT`/`UGOS_NAME` for
/// the unnumbered default, then `UGOS_HOST_1` .. `UGOS_HOST_N` for numbered
/// targets. `UGOS_OTP` and `UGOS_OTP_SECRET` are optional and take the same
/// suffix.
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

/// Read an environment variable, treating a missing or blank value as unset.
fn env_nonempty(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
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
            otp: env_nonempty(&format!("UGOS_OTP{suffix}")),
            otp_secret: env_nonempty(&format!("UGOS_OTP_SECRET{suffix}")),
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
    target: String,
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
    target: String,
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
    target: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct SnapshotListParam {
    /// VM display name or UUID.
    vm: String,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    target: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct SnapshotParam {
    /// VM display name or UUID.
    vm: String,
    /// Snapshot name.
    name: String,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    target: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct SnapshotDescribeParam {
    /// VM display name or UUID.
    vm: String,
    /// Snapshot name.
    name: String,
    /// Description text.
    description: String,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    target: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct NetworkNameParam {
    /// Network name.
    name: String,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    target: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct TargetOnlyParam {
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    target: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct ImageDeleteParam {
    /// Image file name (e.g. "CachyOS.iso").
    file_name: String,
    /// Image display name (e.g. `CachyOS`).
    image_name: String,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    target: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct ImageNameParam {
    /// Image name.
    name: String,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    target: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct StorageRefParam {
    /// Storage volume name.
    name: String,
    /// Storage volume UUID.
    uuid: String,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    target: String,
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
    target: String,
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
    target: String,
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
    target: String,
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
    target: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct OvaParseParam {
    /// OVA file path on the NAS.
    ova_path: String,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    target: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct ContainerIdParam {
    /// Container ID.
    id: String,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    target: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct DockerImageSearchParam {
    /// Image name to search for.
    name: String,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    target: String,
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
    target: String,
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
    target: String,
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
    target: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct ContainerBatchParam {
    /// Container IDs.
    ids: Vec<String>,
    /// Operation: start, stop, restart, remove.
    action: String,
    /// Target NAS.
    #[serde(default)]
    target: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct ImageExportParam {
    /// Image ID.
    id: String,
    /// Destination path on the NAS.
    path: String,
    /// Target NAS.
    #[serde(default)]
    target: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct ImageLoadParam {
    /// URL or NAS path to load image from.
    source: String,
    /// Target NAS.
    #[serde(default)]
    target: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct MirrorAddParam {
    /// Display name.
    alias: String,
    /// Mirror URL.
    address: String,
    /// Target NAS.
    #[serde(default)]
    target: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct MirrorIdParam {
    /// Mirror ID.
    id: i64,
    /// Target NAS.
    #[serde(default)]
    target: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct ComposeParam {
    /// Compose project name.
    project: String,
    /// Target NAS.
    #[serde(default)]
    target: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct ProjectNameParam {
    /// Compose project name.
    name: String,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    target: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct ProjectRemoveParam {
    /// Compose project name.
    name: String,
    /// Also remove images pulled for the project.
    #[serde(default)]
    del_images: bool,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    target: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct ProjectCreateParam {
    /// Compose project name.
    name: String,
    /// NAS storage path for the project (e.g. "/volume1/docker/<name>").
    path: String,
    /// Raw `docker-compose.yml` content.
    content: String,
    /// Start the project immediately after creation.
    #[serde(default)]
    run: bool,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    target: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct ProxySetParam {
    /// Proxy configuration JSON object.
    proxy: serde_json::Value,
    /// Target NAS.
    #[serde(default)]
    target: String,
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
    target: String,
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
    async fn client(&self, target: &str) -> Result<&UgosClient, String> {
        let t = if target.is_empty() {
            None
        } else {
            Some(target)
        };
        let cfg = self.resolve_target(t)?;
        let cell = self
            .clients
            .get(&cfg.name)
            .ok_or("internal: missing client cell")?;
        cell.get_or_try_init(|| async {
            // Same trust-on-first-use store the CLI writes, so a host pinned
            // there is already authenticated here.
            let tls = resolve_tls(&cfg.host, cfg.port)
                .await
                .map_err(|e| format!("tls setup for {}: {e}", cfg.name))?;
            UgosClient::connect(&cfg.host, cfg.port, cfg.creds.clone(), &tls)
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
        match self.client(&p.target).await {
            Ok(c) => match c.vm_list().await {
                Ok(vms) => serde_json::to_string_pretty(&vms).unwrap_or_default(),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Show detailed VM configuration")]
    async fn ugos_vm_show(&self, Parameters(p): Parameters<VmNameParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => match c.vm_show(&p.name).await {
                Ok(d) => serde_json::to_string_pretty(&d).unwrap_or_default(),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Power on a virtual machine")]
    async fn ugos_vm_start(&self, Parameters(p): Parameters<VmNameParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => match c.vm_start(&p.name).await {
                Ok(()) => format!("Started {}", p.name),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Shut down a virtual machine (graceful or forced)")]
    async fn ugos_vm_stop(&self, Parameters(p): Parameters<VmStopParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => match c.vm_stop(&p.name, p.force).await {
                Ok(()) => format!("Stopped {}", p.name),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Reboot a virtual machine (graceful or forced)")]
    async fn ugos_vm_reboot(&self, Parameters(p): Parameters<VmRebootParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => match c.vm_reboot(&p.name, p.force).await {
                Ok(()) => format!("Rebooted {}", p.name),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Delete a virtual machine")]
    async fn ugos_vm_delete(&self, Parameters(p): Parameters<VmNameParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => match c.vm_delete(&p.name).await {
                Ok(()) => format!("Deleted {}", p.name),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "List snapshots for a virtual machine")]
    async fn ugos_snapshot_list(&self, Parameters(p): Parameters<SnapshotListParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => match c.snapshot_list(&p.vm).await {
                Ok(s) => serde_json::to_string_pretty(&s).unwrap_or_default(),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Create a snapshot of a virtual machine")]
    async fn ugos_snapshot_create(&self, Parameters(p): Parameters<SnapshotParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => match c.snapshot_create(&p.vm).await {
                Ok(name) => format!("Created snapshot {name}"),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Delete a snapshot")]
    async fn ugos_snapshot_delete(&self, Parameters(p): Parameters<SnapshotParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => match c.snapshot_delete(&p.vm, &p.name).await {
                Ok(()) => format!("Deleted snapshot {}", p.name),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Revert a virtual machine to a snapshot")]
    async fn ugos_snapshot_revert(&self, Parameters(p): Parameters<SnapshotParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => match c.snapshot_revert(&p.vm, &p.name, false).await {
                Ok(()) => format!("Reverted to snapshot {}", p.name),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Set a snapshot's description")]
    async fn ugos_snapshot_describe(
        &self,
        Parameters(p): Parameters<SnapshotDescribeParam>,
    ) -> String {
        match self.client(&p.target).await {
            Ok(c) => match c.snapshot_describe(&p.vm, &p.name, &p.description).await {
                Ok(()) => format!("Described snapshot {}", p.name),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "List KVM networks")]
    async fn ugos_network_list(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => match c.network_list().await {
                Ok(n) => serde_json::to_string_pretty(&n).unwrap_or_default(),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Show KVM network details")]
    async fn ugos_network_show(&self, Parameters(p): Parameters<NetworkNameParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => match c.network_show(&p.name).await {
                Ok(d) => serde_json::to_string_pretty(&d).unwrap_or_default(),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "List KVM storage volumes")]
    async fn ugos_storage_list(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => match c.storage_list().await {
                Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_default(),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "List ISO/disk images available for KVM")]
    async fn ugos_image_list(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => match c.image_list().await {
                Ok(i) => serde_json::to_string_pretty(&i).unwrap_or_default(),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Show NAS host hardware info (CPU cores, memory)")]
    async fn ugos_host_info(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(&p.target).await {
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
        match self.client(&p.target).await {
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
        match self.client(&p.target).await {
            Ok(c) => match c.image_delete(&p.file_name, &p.image_name).await {
                Ok(()) => format!("Deleted image {}", p.image_name),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Check which VMs use an image")]
    async fn ugos_image_usage(&self, Parameters(p): Parameters<ImageNameParam>) -> String {
        match self.client(&p.target).await {
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
        match self.client(&p.target).await {
            Ok(c) => match c.storage_check_usage(&p.name, &p.uuid).await {
                Ok(vms) => serde_json::to_string_pretty(&vms).unwrap_or_default(),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Add a storage volume to KVM")]
    async fn ugos_storage_add(&self, Parameters(p): Parameters<StorageRefParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => match c.storage_add(&p.name, &p.uuid).await {
                Ok(()) => format!("Added storage {}", p.name),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Remove a storage volume from KVM")]
    async fn ugos_storage_delete(&self, Parameters(p): Parameters<StorageRefParam>) -> String {
        match self.client(&p.target).await {
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
        match self.client(&p.target).await {
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
        match self.client(&p.target).await {
            Ok(c) => match c.vnc_list(&p.vm).await {
                Ok(l) => serde_json::to_string_pretty(&l).unwrap_or_default(),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Generate a noVNC link to access a VM console in the browser")]
    async fn ugos_vnc_generate(&self, Parameters(p): Parameters<VncGenerateParam>) -> String {
        match self.client(&p.target).await {
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
        match self.client(&p.target).await {
            Ok(c) => match c.log_search(p.page, p.page_size).await {
                Ok(l) => serde_json::to_string_pretty(&l).unwrap_or_default(),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "List all operator usernames from KVM logs")]
    async fn ugos_log_operators(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(&p.target).await {
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
        match self.client(&p.target).await {
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
        match self.client(&p.target).await {
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
        match self.client(&p.target).await {
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
        match self.client(&p.target).await {
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
        match self.client(&p.target).await {
            Ok(c) => match c.docker_overview().await {
                Ok(o) => serde_json::to_string_pretty(&o).unwrap_or_default(),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Get Docker engine status (online/offline)")]
    async fn ugos_docker_status(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => match c.docker_engine_status().await {
                Ok(s) => s,
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "List Docker containers")]
    async fn ugos_docker_ps(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => match c.container_list(1, 100).await {
                Ok(r) => serde_json::to_string_pretty(&r).unwrap_or_default(),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Start a Docker container")]
    async fn ugos_docker_start(&self, Parameters(p): Parameters<ContainerIdParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => match c.container_start(&p.id).await {
                Ok(()) => format!("Started {}", p.id),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Show detailed Docker container configuration")]
    async fn ugos_docker_show(&self, Parameters(p): Parameters<ContainerIdParam>) -> String {
        match self.client(&p.target).await {
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
        match self.client(&p.target).await {
            Ok(c) => match c.container_create(&spec).await {
                Ok(()) => format!("Created container {}", spec.container_name),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Stop a Docker container")]
    async fn ugos_docker_stop(&self, Parameters(p): Parameters<ContainerIdParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => match c.container_stop(&p.id).await {
                Ok(()) => format!("Stopped {}", p.id),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Restart a Docker container")]
    async fn ugos_docker_restart(&self, Parameters(p): Parameters<ContainerIdParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => match c.container_restart(&p.id).await {
                Ok(()) => format!("Restarted {}", p.id),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Remove a Docker container")]
    async fn ugos_docker_rm(&self, Parameters(p): Parameters<ContainerIdParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => match c.container_remove(&p.id).await {
                Ok(()) => format!("Removed {}", p.id),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "List local Docker images")]
    async fn ugos_docker_images(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(&p.target).await {
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
        match self.client(&p.target).await {
            Ok(c) => match c.docker_image_search(&p.name, 1, 20).await {
                Ok(r) => serde_json::to_string_pretty(&r).unwrap_or_default(),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Pull a Docker image from a registry")]
    async fn ugos_docker_pull(&self, Parameters(p): Parameters<DockerImagePullParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => match c.docker_image_download(&p.image, &p.tag).await {
                Ok(()) => format!("Pulling {}:{}", p.image, p.tag),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "List Docker registry mirror sources")]
    async fn ugos_docker_mirrors(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => match c.mirror_list().await {
                Ok(m) => serde_json::to_string_pretty(&m).unwrap_or_default(),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Get Docker container logs")]
    async fn ugos_docker_logs(&self, Parameters(p): Parameters<ContainerLogsParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => match c.container_logs(&p.id, p.lines).await {
                Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_default(),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Clone a Docker container")]
    async fn ugos_docker_clone(&self, Parameters(p): Parameters<ContainerCloneParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => match c.container_clone(&p.id, &p.name).await {
                Ok(()) => format!("Cloned {} as {}", p.id, p.name),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Batch operate on Docker containers (start/stop/restart/remove)")]
    async fn ugos_docker_batch(&self, Parameters(p): Parameters<ContainerBatchParam>) -> String {
        match self.client(&p.target).await {
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
        match self.client(&p.target).await {
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
        match self.client(&p.target).await {
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
        match self.client(&p.target).await {
            Ok(c) => match c.docker_image_load_path(&p.source).await {
                Ok(()) => format!("Loading image from {}", p.source),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Add a Docker registry mirror source")]
    async fn ugos_docker_mirror_add(&self, Parameters(p): Parameters<MirrorAddParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => match c.mirror_add(&p.alias, &p.address, None).await {
                Ok(()) => format!("Added mirror {}", p.alias),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Delete a Docker registry mirror source")]
    async fn ugos_docker_mirror_delete(&self, Parameters(p): Parameters<MirrorIdParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => match c.mirror_delete(p.id).await {
                Ok(()) => format!("Deleted mirror {}", p.id),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Switch active Docker registry mirror source")]
    async fn ugos_docker_mirror_switch(&self, Parameters(p): Parameters<MirrorIdParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => match c.mirror_switch(p.id).await {
                Ok(()) => format!("Switched to mirror {}", p.id),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "List containers in a Docker Compose project")]
    async fn ugos_docker_compose(&self, Parameters(p): Parameters<ComposeParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => match c.compose_containers(&p.project).await {
                Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_default(),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "List Docker Compose projects")]
    async fn ugos_project_list(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => match c.project_list().await {
                Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_default(),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Show details of a Docker Compose project")]
    async fn ugos_project_show(&self, Parameters(p): Parameters<ProjectNameParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => match c.project_show(&p.name).await {
                Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_default(),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(
        description = "Create a Docker Compose project from raw docker-compose.yml content. Use ugos_project_list first to check the shared-folder root for the storage path."
    )]
    async fn ugos_project_create(&self, Parameters(p): Parameters<ProjectCreateParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => match c.project_create(&p.name, &p.path, &p.content, p.run).await {
                Ok(()) => format!("Created project {}", p.name),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Start a Docker Compose project")]
    async fn ugos_project_start(&self, Parameters(p): Parameters<ProjectNameParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => match c.project_start(&p.name).await {
                Ok(()) => format!("Started project {}", p.name),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Stop a Docker Compose project (containers remain, not removed)")]
    async fn ugos_project_stop(&self, Parameters(p): Parameters<ProjectNameParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => match c.project_stop(&p.name).await {
                Ok(()) => format!("Stopped project {}", p.name),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Restart a Docker Compose project")]
    async fn ugos_project_restart(&self, Parameters(p): Parameters<ProjectNameParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => match c.project_restart(&p.name).await {
                Ok(()) => format!("Restarted project {}", p.name),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Remove a Docker Compose project (docker compose down)")]
    async fn ugos_project_remove(&self, Parameters(p): Parameters<ProjectRemoveParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => match c.project_remove(&p.name, p.del_images).await {
                Ok(()) => format!("Removed project {}", p.name),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Get Docker HTTP proxy configuration")]
    async fn ugos_docker_proxy_get(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => match c.docker_proxy_get().await {
                Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_default(),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }

    #[tool(description = "Set Docker HTTP proxy configuration")]
    async fn ugos_docker_proxy_set(&self, Parameters(p): Parameters<ProxySetParam>) -> String {
        match self.client(&p.target).await {
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
        match self.client(&p.target).await {
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

    // ── Shared folders, accounts and groups ─────────────────────────
    //
    // The logic is in `manage`. Passwords, roles and `--force` are left to
    // `ugos-cli`; see there for why.

    #[tool(description = "List the shared folders (SMB shares) of a UGOS NAS")]
    async fn ugos_share_list(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => manage::share_list(c).await,
            Err(e) => e,
        }
    }

    #[tool(
        description = "Show the settings of a shared folder: volume, quota, visibility, encryption"
    )]
    async fn ugos_share_show(&self, Parameters(p): Parameters<ShareParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => manage::share_show(c, &p.share).await,
            Err(e) => e,
        }
    }

    #[tool(description = "Show who may access a shared folder: groups, users and their access")]
    async fn ugos_share_acl_show(&self, Parameters(p): Parameters<ShareParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => manage::share_acl_show(c, &p.share).await,
            Err(e) => e,
        }
    }

    #[tool(description = "Show the SMB service settings and the discovery services")]
    async fn ugos_smb_status(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => manage::smb_status(c).await,
            Err(e) => e,
        }
    }

    #[tool(
        description = "Create a shared folder. Only administrators can open it at first; set access with ugos_share_acl_set"
    )]
    async fn ugos_share_create(&self, Parameters(p): Parameters<ShareCreateParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => manage::share_create(c, &p).await,
            Err(e) => e,
        }
    }

    #[tool(
        description = "Change the quota or the visibility of a shared folder. The result is read back; an error says what UGOS did not apply"
    )]
    async fn ugos_share_update(&self, Parameters(p): Parameters<ShareUpdateParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => manage::share_update(c, &p).await,
            Err(e) => e,
        }
    }

    #[tool(
        description = "Set access rules on a shared folder. The administrators group always keeps read and write; a rule asking for less is reported, not applied"
    )]
    async fn ugos_share_acl_set(&self, Parameters(p): Parameters<ShareAclSetParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => manage::share_acl_set(c, &p).await,
            Err(e) => e,
        }
    }

    #[tool(
        description = "Delete a shared folder and everything in it. Refused while the folder holds files unless force is given; ask the user first"
    )]
    async fn ugos_share_delete(&self, Parameters(p): Parameters<ShareDeleteParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => manage::share_delete(c, &p).await,
            Err(e) => e,
        }
    }

    #[tool(description = "List user accounts with role, groups and status")]
    async fn ugos_user_list(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => manage::user_list(c).await,
            Err(e) => e,
        }
    }

    #[tool(description = "Show one user account")]
    async fn ugos_user_show(&self, Parameters(p): Parameters<AccountParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => manage::user_show(c, &p.account).await,
            Err(e) => e,
        }
    }

    #[tool(
        description = "List the groups an administrator made, with their members. The built-in groups are not listed"
    )]
    async fn ugos_group_list(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => manage::group_list(c).await,
            Err(e) => e,
        }
    }

    #[tool(
        description = "Change a user account: email, description, groups, home folder, password rule, disable or expiry. Cannot set a password or a role; use ugos-cli for those. Refuses to lock out the account this server is logged in with"
    )]
    async fn ugos_user_update(&self, Parameters(p): Parameters<UserUpdateParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => manage::user_update(c, &p).await,
            Err(e) => e,
        }
    }

    #[tool(
        description = "Delete a user account. Its home folder and the files in it go with it, so an account with files or snapshots is refused unless force is given; ask the user first"
    )]
    async fn ugos_user_delete(&self, Parameters(p): Parameters<AccountDeleteParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => manage::user_delete(c, &p).await,
            Err(e) => e,
        }
    }

    // ── Storage Manager (read only) ─────────────────────────────────

    #[tool(description = "List the storage pools (RAID sets) with level, capacity and disks")]
    async fn ugos_pool_list(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas::pool_list(c).await,
            Err(e) => e,
        }
    }

    #[tool(description = "Show one storage pool with its disks, scrub and trim")]
    async fn ugos_pool_show(&self, Parameters(p): Parameters<NameParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas::pool_show(c, &p.name).await,
            Err(e) => e,
        }
    }

    #[tool(description = "List the NAS volumes (file systems on the pools) with usage")]
    async fn ugos_volume_list(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas::volume_list(c).await,
            Err(e) => e,
        }
    }

    #[tool(description = "Show one NAS volume, found by name, label or mount path")]
    async fn ugos_volume_show(&self, Parameters(p): Parameters<NameParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas::volume_show(c, &p.name).await,
            Err(e) => e,
        }
    }

    #[tool(
        description = "List the disks with model, size, temperature, hours on and the pool they serve"
    )]
    async fn ugos_disk_list(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas::disk_list(c).await,
            Err(e) => e,
        }
    }

    #[tool(
        description = "Show one disk with serial, firmware and interface. Name it by short name, device path, slot or label"
    )]
    async fn ugos_disk_show(&self, Parameters(p): Parameters<NameParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas::disk_show(c, &p.name).await,
            Err(e) => e,
        }
    }

    #[tool(
        description = "Show the SMART report of a disk: wear, spare, errors, unsafe shutdowns and the test state"
    )]
    async fn ugos_disk_smart(&self, Parameters(p): Parameters<NameParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas::disk_smart(c, &p.name).await,
            Err(e) => e,
        }
    }

    #[tool(description = "Show the history of a disk's SMART tests")]
    async fn ugos_disk_history(&self, Parameters(p): Parameters<NameParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas::disk_history(c, &p.name).await,
            Err(e) => e,
        }
    }

    #[tool(description = "List the scheduled disk test plans")]
    async fn ugos_disk_plans(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas::disk_plans(c).await,
            Err(e) => e,
        }
    }

    // ── Network settings of the NAS ─────────────────────────────────
    #[tool(description = "List the network interfaces of the NAS with link, address and speed")]
    async fn ugos_net_list(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_net::list(c).await,
            Err(e) => e,
        }
    }

    #[tool(
        description = "Show one network interface: addresses, DNS, gateway, MTU, ports of a bridge or bond"
    )]
    async fn ugos_net_show(&self, Parameters(p): Parameters<NameParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_net::show(c, &p.name).await,
            Err(e) => e,
        }
    }

    #[tool(description = "Show DNS, default gateways, the IPv6 interface and the proxy of the NAS")]
    async fn ugos_net_general(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_net::general(c).await,
            Err(e) => e,
        }
    }

    #[tool(
        description = "Change the setup of one network interface (IPv4, IPv6, MTU, bond mode). Refused for the interface this session talks through unless force is given: a changed address cuts it off"
    )]
    async fn ugos_net_set(&self, Parameters(p): Parameters<NetSetParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_net::set(c, &p).await,
            Err(e) => e,
        }
    }

    #[tool(
        description = "Set the DNS servers (automatic, or one or two) and the general switches: several gateways, prefer IPv4, address clash check"
    )]
    async fn ugos_net_dns_set(&self, Parameters(p): Parameters<NetDnsParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_net::dns(c, &p).await,
            Err(e) => e,
        }
    }

    #[tool(description = "Put the default gateways in an order of preference, by interface")]
    async fn ugos_net_gateway_order(&self, Parameters(p): Parameters<GatewayOrderParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_net::gateway_order(c, &p).await,
            Err(e) => e,
        }
    }

    #[tool(
        description = "Set or switch off the HTTP proxy of the NAS. A password passes through the model; prefer ugos-cli for that"
    )]
    async fn ugos_net_proxy_set(&self, Parameters(p): Parameters<NetProxyParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_net::proxy(c, &p).await,
            Err(e) => e,
        }
    }

    #[tool(description = "List the ports that could join a bond")]
    async fn ugos_net_bond_candidates(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_net::candidates(c, true).await,
            Err(e) => e,
        }
    }

    #[tool(
        description = "Create a bond (link aggregation) from two or more ports. Refused when a port carries this session or the ports are in different networks unless force is given"
    )]
    async fn ugos_net_bond_create(&self, Parameters(p): Parameters<BondCreateParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_net::bond_create(c, &p).await,
            Err(e) => e,
        }
    }

    #[tool(
        description = "Delete a bond. Refused when firewall or traffic rules name it or it carries this session unless force is given"
    )]
    async fn ugos_net_bond_delete(&self, Parameters(p): Parameters<IfaceForceParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_net::bond_delete(c, &p).await,
            Err(e) => e,
        }
    }

    #[tool(description = "List the ports that could join a bridge")]
    async fn ugos_net_bridge_candidates(
        &self,
        Parameters(p): Parameters<TargetOnlyParam>,
    ) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_net::candidates(c, false).await,
            Err(e) => e,
        }
    }

    #[tool(
        description = "Create a bridge from two or more ports. Refused when a port carries this session unless force is given"
    )]
    async fn ugos_net_bridge_create(&self, Parameters(p): Parameters<PortsParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_net::bridge_create(c, &p).await,
            Err(e) => e,
        }
    }

    #[tool(description = "Create a virtual bridge, which VMs attach to, from one or more ports")]
    async fn ugos_net_bridge_create_virtual(
        &self,
        Parameters(p): Parameters<PortsParam>,
    ) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_net::bridge_create_virtual(c, &p).await,
            Err(e) => e,
        }
    }

    #[tool(
        description = "Delete a bridge. Refused when rules name it or it carries this session unless force is given"
    )]
    async fn ugos_net_bridge_delete(&self, Parameters(p): Parameters<IfaceForceParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_net::bridge_delete(c, &p).await,
            Err(e) => e,
        }
    }

    #[tool(description = "Show whether the Wi-Fi radio is on and which network it is connected to")]
    async fn ugos_net_wifi_status(&self, Parameters(p): Parameters<WifiParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_net::wifi_status(c, &p).await,
            Err(e) => e,
        }
    }

    #[tool(description = "Switch the Wi-Fi radio on or off (on: true or false)")]
    async fn ugos_net_wifi_radio(&self, Parameters(p): Parameters<WifiParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_net::wifi_radio(c, &p).await,
            Err(e) => e,
        }
    }

    #[tool(description = "List the Wi-Fi networks in range and the saved ones")]
    async fn ugos_net_wifi_list(&self, Parameters(p): Parameters<WifiParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_net::wifi_list(c, &p).await,
            Err(e) => e,
        }
    }

    #[tool(
        description = "Connect to a Wi-Fi network (ssid). A saved one is switched to. A password passes through the model; prefer ugos-cli for that"
    )]
    async fn ugos_net_wifi_connect(&self, Parameters(p): Parameters<WifiParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_net::wifi_connect(c, &p).await,
            Err(e) => e,
        }
    }

    #[tool(description = "Forget a saved Wi-Fi network (ssid)")]
    async fn ugos_net_wifi_forget(&self, Parameters(p): Parameters<WifiParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_net::wifi_forget(c, &p).await,
            Err(e) => e,
        }
    }

    #[tool(description = "Change whether a saved Wi-Fi network joins by itself (ssid, auto_join)")]
    async fn ugos_net_wifi_edit(&self, Parameters(p): Parameters<WifiParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_net::wifi_edit(c, &p).await,
            Err(e) => e,
        }
    }

    #[tool(description = "List the traffic control rules in order of priority")]
    async fn ugos_net_flow_list(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_net::flow_list(c).await,
            Err(e) => e,
        }
    }

    #[tool(description = "List the built-in services a traffic control rule can name")]
    async fn ugos_net_flow_services(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_net::flow_services(c).await,
            Err(e) => e,
        }
    }

    #[tool(
        description = "Add a traffic control rule at the end of the list: all ports, custom ports, or built-in services, with a rate limit in KB/s"
    )]
    async fn ugos_net_flow_add(&self, Parameters(p): Parameters<FlowAddParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_net::flow_add(c, &p).await,
            Err(e) => e,
        }
    }

    #[tool(description = "Remove the traffic control rule at a position (counting from 1)")]
    async fn ugos_net_flow_remove(&self, Parameters(p): Parameters<FlowPositionParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_net::flow_remove(c, &p).await,
            Err(e) => e,
        }
    }

    #[tool(
        description = "Switch the traffic control rule at a position on or off (on: true or false)"
    )]
    async fn ugos_net_flow_enable(&self, Parameters(p): Parameters<FlowPositionParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_net::flow_enable(c, &p).await,
            Err(e) => e,
        }
    }

    // ── Hardware, power, time, language, terminal ───────────────────
    #[tool(description = "Show the buzzer settings: which events sound it")]
    async fn ugos_hw_buzzer(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_hw::buzzer_show(c).await,
            Err(e) => e,
        }
    }

    #[tool(description = "List the events the buzzer can sound for")]
    async fn ugos_hw_buzzer_alarms(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_hw::buzzer_alarms(c).await,
            Err(e) => e,
        }
    }

    #[tool(description = "Change which events sound the buzzer")]
    async fn ugos_hw_buzzer_set(&self, Parameters(p): Parameters<BuzzerParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_hw::buzzer_set(c, &p).await,
            Err(e) => e,
        }
    }

    #[tool(description = "Show the CPU performance mode")]
    async fn ugos_hw_cpu(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_hw::cpu_show(c).await,
            Err(e) => e,
        }
    }

    #[tool(description = "Set the CPU performance mode: high, balanced or save")]
    async fn ugos_hw_cpu_set(&self, Parameters(p): Parameters<ModeParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_hw::cpu_set(c, &p).await,
            Err(e) => e,
        }
    }

    #[tool(description = "Show the fan mode and the fan curve")]
    async fn ugos_hw_fan(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_hw::fan_show(c).await,
            Err(e) => e,
        }
    }

    #[tool(description = "Set the fan mode: quiet, auto or full")]
    async fn ugos_hw_fan_set(&self, Parameters(p): Parameters<ModeParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_hw::fan_set(c, &p).await,
            Err(e) => e,
        }
    }

    #[tool(
        description = "Set a custom fan curve of 17 duties per fan (20 to 100 percent at 0 to 80 degrees). Only on models that support it"
    )]
    async fn ugos_hw_fan_curve(&self, Parameters(p): Parameters<FanCurveParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_hw::fan_curve(c, &p).await,
            Err(e) => e,
        }
    }

    #[tool(description = "Show the LED settings and brightness schedule")]
    async fn ugos_hw_led(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_hw::led_show(c).await,
            Err(e) => e,
        }
    }

    #[tool(description = "Switch the LEDs on")]
    async fn ugos_hw_led_on(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_hw::led_on(c).await,
            Err(e) => e,
        }
    }

    #[tool(description = "Switch the LEDs off")]
    async fn ugos_hw_led_off(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_hw::led_off(c).await,
            Err(e) => e,
        }
    }

    #[tool(description = "Set the LED brightness on weekdays")]
    async fn ugos_hw_led_brightness(&self, Parameters(p): Parameters<LedParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_hw::led_brightness(c, &p).await,
            Err(e) => e,
        }
    }

    #[tool(description = "Show the settings of the small display")]
    async fn ugos_hw_screen(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_hw::screen_show(c).await,
            Err(e) => e,
        }
    }

    #[tool(description = "List the wallpapers of the small display")]
    async fn ugos_hw_screen_wallpapers(
        &self,
        Parameters(p): Parameters<TargetOnlyParam>,
    ) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_hw::screen_wallpapers(c).await,
            Err(e) => e,
        }
    }

    #[tool(description = "Change brightness, timeout, wallpaper or language of the small display")]
    async fn ugos_hw_screen_set(&self, Parameters(p): Parameters<ScreenParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_hw::screen_set(c, &p).await,
            Err(e) => e,
        }
    }

    #[tool(description = "Show whether memory compression (zram) is on")]
    async fn ugos_hw_zram(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_hw::zram_show(c).await,
            Err(e) => e,
        }
    }

    #[tool(description = "Switch memory compression on")]
    async fn ugos_hw_zram_on(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_hw::zram_on(c).await,
            Err(e) => e,
        }
    }

    #[tool(
        description = "Switch memory compression off. The NAS restarts, so this is refused unless force is given: ask the user first"
    )]
    async fn ugos_hw_zram_off(&self, Parameters(p): Parameters<ForceParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_hw::zram_off(c, &p).await,
            Err(e) => e,
        }
    }

    #[tool(
        description = "Show the power settings: restart after power loss, wake on LAN, disk sleep and the schedule"
    )]
    async fn ugos_hw_power(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_hw::power_show(c).await,
            Err(e) => e,
        }
    }

    #[tool(description = "Change the power settings")]
    async fn ugos_hw_power_set(&self, Parameters(p): Parameters<PowerParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_hw::power_set(c, &p).await,
            Err(e) => e,
        }
    }

    #[tool(
        description = "Add an entry to the power schedule (power on or off at a time on weekdays)"
    )]
    async fn ugos_hw_schedule_add(&self, Parameters(p): Parameters<ScheduleParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_hw::schedule_add(c, &p).await,
            Err(e) => e,
        }
    }

    #[tool(description = "Remove the power schedule entry at a position")]
    async fn ugos_hw_schedule_remove(&self, Parameters(p): Parameters<PositionParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_hw::schedule_remove(c, &p).await,
            Err(e) => e,
        }
    }

    #[tool(description = "Show the UPS settings and state")]
    async fn ugos_hw_ups(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_hw::ups_show(c).await,
            Err(e) => e,
        }
    }

    #[tool(
        description = "Connect a UPS (USB, SNMP or network) and set what to do on battery. Stops and restarts the UPS service"
    )]
    async fn ugos_hw_ups_set(&self, Parameters(p): Parameters<UpsParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_hw::ups_set(c, &p).await,
            Err(e) => e,
        }
    }

    #[tool(description = "Disconnect the UPS")]
    async fn ugos_hw_ups_disconnect(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_hw::ups_disconnect(c).await,
            Err(e) => e,
        }
    }

    #[tool(description = "Check that a NUT server answers at an address")]
    async fn ugos_hw_ups_check(&self, Parameters(p): Parameters<NameParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_hw::ups_check(c, &p.name).await,
            Err(e) => e,
        }
    }

    #[tool(description = "List the SNMP MIBs for a UPS")]
    async fn ugos_hw_ups_mibs(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_hw::ups_mibs(c).await,
            Err(e) => e,
        }
    }

    #[tool(description = "Show the firmware of the USB UPS")]
    async fn ugos_hw_ups_firmware(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_hw::ups_firmware(c).await,
            Err(e) => e,
        }
    }

    #[tool(description = "List the GPUs of the NAS and their mode")]
    async fn ugos_hw_gpu(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_hw::gpu_list(c).await,
            Err(e) => e,
        }
    }

    #[tool(
        description = "Set a GPU to NAS use or VFIO passthrough. The NAS restarts, so this is refused unless force is given: ask the user first"
    )]
    async fn ugos_hw_gpu_mode(&self, Parameters(p): Parameters<GpuParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_hw::gpu_mode(c, &p).await,
            Err(e) => e,
        }
    }

    #[tool(description = "Show time zone, time and how the clock is kept")]
    async fn ugos_time_show(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_hw::time_show(c).await,
            Err(e) => e,
        }
    }

    #[tool(description = "List the time zones")]
    async fn ugos_time_zones(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_hw::time_zones(c).await,
            Err(e) => e,
        }
    }

    #[tool(
        description = "Set the time zone, a time server or the clock by hand. A change that may invalidate the cloud certificate needs force"
    )]
    async fn ugos_time_set(&self, Parameters(p): Parameters<TimeParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_hw::time_set(c, &p).await,
            Err(e) => e,
        }
    }

    #[tool(description = "List the time servers")]
    async fn ugos_time_servers(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_hw::time_servers(c).await,
            Err(e) => e,
        }
    }

    #[tool(description = "Add a time server (name is the host)")]
    async fn ugos_time_server_add(&self, Parameters(p): Parameters<NameParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_hw::time_server_add(c, &p.name).await,
            Err(e) => e,
        }
    }

    #[tool(description = "Remove a time server (name is the host)")]
    async fn ugos_time_server_remove(&self, Parameters(p): Parameters<NameParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_hw::time_server_remove(c, &p.name).await,
            Err(e) => e,
        }
    }

    #[tool(description = "Show the language of the NAS")]
    async fn ugos_lang_show(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_hw::lang_show(c).await,
            Err(e) => e,
        }
    }

    #[tool(description = "List the languages")]
    async fn ugos_lang_list(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_hw::lang_list(c).await,
            Err(e) => e,
        }
    }

    #[tool(description = "Set the language (name is the language key, or auto)")]
    async fn ugos_lang_set(&self, Parameters(p): Parameters<NameParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_hw::lang_set(c, &p.name).await,
            Err(e) => e,
        }
    }

    #[tool(description = "Show SSH and Telnet settings")]
    async fn ugos_terminal_show(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_hw::terminal_show(c).await,
            Err(e) => e,
        }
    }

    #[tool(
        description = "Change SSH and Telnet. Switching a service on is refused unless force is given: it opens a way in, so ask the user first"
    )]
    async fn ugos_terminal_set(&self, Parameters(p): Parameters<TerminalParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_hw::terminal_set(c, &p).await,
            Err(e) => e,
        }
    }

    #[tool(description = "Set the host name of the NAS (name is the new host name)")]
    async fn ugos_system_hostname_set(&self, Parameters(p): Parameters<NameParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_hw::hostname_set(c, &p.name).await,
            Err(e) => e,
        }
    }

    #[tool(description = "Show the finder switches: improvement plan and findable from the web")]
    async fn ugos_system_finder(&self, Parameters(p): Parameters<TargetOnlyParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_hw::finder_show(c).await,
            Err(e) => e,
        }
    }

    #[tool(description = "Change the finder switches")]
    async fn ugos_system_finder_set(&self, Parameters(p): Parameters<FinderParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_hw::finder_set(c, &p).await,
            Err(e) => e,
        }
    }

    // ── File services ───────────────────────────────────────────────
    #[tool(
        description = "Show a file service (smb, ftp, nfs, rsync or webdav): whether it runs and its settings. For smb, also the discovery services"
    )]
    async fn ugos_fileservice_status(&self, Parameters(p): Parameters<ServiceParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_files::status(c, &p).await,
            Err(e) => e,
        }
    }

    #[tool(description = "List the settings of a file service by alias, what they take and mean")]
    async fn ugos_fileservice_settings(&self, Parameters(p): Parameters<ServiceParam>) -> String {
        nas_files::settings(&p)
    }

    #[tool(
        description = "Switch a file service on, with settings. Plain FTP, SMB1, NTLMv1 and NFS are refused unless force is given: ask the user first. rsync needs an account, and a password for a new one"
    )]
    async fn ugos_fileservice_start(
        &self,
        Parameters(p): Parameters<ServiceChangeParam>,
    ) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_files::change(c, &p, true).await,
            Err(e) => e,
        }
    }

    #[tool(
        description = "Change the settings of a file service that runs. Refused if it is off: start it with the settings instead"
    )]
    async fn ugos_fileservice_set(&self, Parameters(p): Parameters<ServiceChangeParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_files::change(c, &p, false).await,
            Err(e) => e,
        }
    }

    #[tool(
        description = "Switch a file service off. SMB is refused while Bonjour or WS-Discovery announce it unless force is given: ask the user first"
    )]
    async fn ugos_fileservice_stop(&self, Parameters(p): Parameters<ServiceStopParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_files::stop(c, &p).await,
            Err(e) => e,
        }
    }

    #[tool(
        description = "Show or change the SMB discovery services (WS-Discovery, Bonjour, UPnP) and the Time Machine targets. Without arguments it only reads"
    )]
    async fn ugos_smb_discovery(&self, Parameters(p): Parameters<DiscoveryParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_files::discovery(c, &p).await,
            Err(e) => e,
        }
    }

    #[tool(
        description = "Clear the SMB cache. The network service restarts, so this is refused unless force is given: ask the user first"
    )]
    async fn ugos_smb_cache_clear(&self, Parameters(p): Parameters<ForceOnlyParam>) -> String {
        match self.client(&p.target).await {
            Ok(c) => nas_files::clear_cache(c, &p).await,
            Err(e) => e,
        }
    }

    #[tool(description = "Update a KVM network")]
    async fn ugos_network_update(&self, Parameters(p): Parameters<VmSpecParam>) -> String {
        let net: ugos_client::types::kvm::NetworkDetail = match serde_json::from_value(p.spec) {
            Ok(s) => s,
            Err(e) => return format!("error parsing spec: {e}"),
        };
        match self.client(&p.target).await {
            Ok(c) => match c.network_update(&net).await {
                Ok(()) => format!("Updated network {}", net.network_name),
                Err(e) => format!("error: {e}"),
            },
            Err(e) => e,
        }
    }
}

// The rmcp macro expands to an async trait method that never awaits, which
// clippy flags from 1.98 onwards. The code is generated, so the lint cannot
// be satisfied here.
#[allow(clippy::unused_async_trait_impl)]
#[tool_handler]
impl ServerHandler for UgosMcp {
    fn get_info(&self) -> ServerConfig {
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
        // Without with_server_info the server reports itself as "rmcp" in
        // rmcp's version: ServerInfo::new calls
        // Implementation::from_build_env(), and the env!("CARGO_CRATE_NAME")
        // inside it stands in the library, not here. A client would then not
        // see what it is talking to.
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION"),
            ))
            .with_instructions(instructions)
    }
}

#[cfg(test)]
mod tests;

/// Pin a host's certificate on first contact, then require it to match.
///
/// There is no terminal here to warn on, so an unknown host is recorded
/// silently; the CLI prints the fingerprint when it does the same.
///
/// # Errors
///
/// Returns an error if the host cannot be reached or the store is unusable.
async fn resolve_tls(host: &str, port: u16) -> Result<TlsPolicy, ugos_client::error::UgosError> {
    if let Some(fp) = tls::known_hosts::get(host, port)? {
        return Ok(TlsPolicy::Pinned(fp));
    }
    let seen = tls::probe_fingerprint(host, port).await?;
    tls::known_hosts::put(host, port, &seen)?;
    Ok(TlsPolicy::Pinned(seen))
}
