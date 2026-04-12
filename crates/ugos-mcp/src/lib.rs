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
