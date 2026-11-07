//! File services: the logic behind the `ugos_fileservice_*` and
//! `ugos_smb_discovery` tools.
//!
//! The same operations as `ugos-cli`. A change the web UI asks a confirmation
//! for needs `force`, and the model is told to ask the user first.

use std::collections::BTreeMap;

use rmcp::schemars;
use serde::Serialize;
use serde_json::Value;
use ugos_client::UgosClient;
use ugos_client::error::UgosError;
use ugos_client::ops::fileserver::advanced::{self, AdvancedChange};
use ugos_client::ops::fileserver::{self, Change, Service, smb};

use crate::manage::failure;

fn json<T: Serialize>(result: Result<T, UgosError>) -> String {
    match result {
        Ok(value) => serde_json::to_string_pretty(&value).unwrap_or_default(),
        Err(e) => failure(&e),
    }
}

/// A service by name.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ServiceParam {
    /// `smb`, `ftp`, `nfs`, `rsync` or `webdav`.
    pub service: String,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// Start or change a service.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ServiceChangeParam {
    /// `smb`, `ftp`, `nfs`, `rsync` or `webdav`.
    pub service: String,
    /// Settings by alias, like `{"port": 2121, "log": "on"}`. The settings
    /// tool lists the aliases and what they take.
    #[serde(default)]
    pub settings: BTreeMap<String, Value>,
    /// The rsync password. Only for a new rsync account.
    #[serde(default)]
    pub password: Option<String>,
    /// Go on although the web UI asks a confirmation: plain FTP, SMB1,
    /// `NTLMv1`, NFS, discovery services while SMB is off. Ask the user first.
    #[serde(default)]
    pub force: bool,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// Stop a service.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ServiceStopParam {
    /// `smb`, `ftp`, `nfs`, `rsync` or `webdav`.
    pub service: String,
    /// Go on although discovery services announce the SMB shares. Ask the
    /// user first.
    #[serde(default)]
    pub force: bool,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// Discovery services and Time Machine.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct DiscoveryParam {
    /// WS-Discovery, for Windows.
    #[serde(default)]
    pub wsdd2: Option<bool>,
    /// Bonjour, for Macs.
    #[serde(default)]
    pub bonjour: Option<bool>,
    /// `UPnP` announcements.
    #[serde(default)]
    pub upnp: Option<bool>,
    /// Shares that are Time Machine targets; an empty list switches it off.
    #[serde(default)]
    pub time_machine: Option<Vec<String>>,
    /// Go on although UGOS advises against it. Ask the user first.
    #[serde(default)]
    pub force: bool,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// Only the force switch.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ForceOnlyParam {
    /// Go on although the network service restarts. Ask the user first.
    #[serde(default)]
    pub force: bool,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

fn service(name: &str) -> Result<Service, UgosError> {
    match name.trim().to_ascii_lowercase().as_str() {
        "smb" => Ok(Service::Smb),
        "ftp" => Ok(Service::Ftp),
        "nfs" => Ok(Service::Nfs),
        "rsync" => Ok(Service::Rsync),
        "webdav" => Ok(Service::Webdav),
        other => Err(UgosError::Refused(format!(
            "{other:?} is no file service: smb, ftp, nfs, rsync or webdav"
        ))),
    }
}

/// A setting value as the text the parsers take.
fn text(value: &Value) -> String {
    match value {
        Value::Bool(true) => "on".into(),
        Value::Bool(false) => "off".into(),
        Value::String(s) => s.clone(),
        Value::Array(list) => list.iter().map(text).collect::<Vec<_>>().join(","),
        other => other.to_string(),
    }
}

pub async fn status(client: &UgosClient, p: &ServiceParam) -> String {
    let run = async {
        let svc = service(&p.service)?;
        let mut out = serde_json::json!({ "running": false });
        let config = fileserver::config(client, svc).await?;
        out["running"] = Value::Bool(svc.running(&config));
        out["config"] = Value::Object(config);
        if svc == Service::Smb {
            out["discovery"] = serde_json::to_value(advanced::get(client).await?)?;
        }
        Ok(out)
    };
    json(run.await)
}

pub fn settings(p: &ServiceParam) -> String {
    let run = || {
        let svc = service(&p.service)?;
        let list: Vec<Value> = svc
            .fields()
            .iter()
            .map(|f| serde_json::json!({ "setting": f.alias, "takes": format!("{:?}", f.kind), "meaning": f.help }))
            .collect();
        Ok::<_, UgosError>(list)
    };
    json(run())
}

pub async fn change(client: &UgosClient, p: &ServiceChangeParam, enable: bool) -> String {
    let run = async {
        let svc = service(&p.service)?;
        let change = Change {
            settings: p
                .settings
                .iter()
                .map(|(k, v)| (k.clone(), text(v)))
                .collect(),
            password: p.password.clone(),
            force: p.force,
        };
        fileserver::apply(client, svc, enable, &change).await
    };
    json(run.await)
}

pub async fn stop(client: &UgosClient, p: &ServiceStopParam) -> String {
    let run = async { fileserver::stop(client, service(&p.service)?, p.force).await };
    json(run.await)
}

pub async fn discovery(client: &UgosClient, p: &DiscoveryParam) -> String {
    let change = AdvancedChange {
        wsdd2: p.wsdd2,
        bonjour: p.bonjour,
        upnp: p.upnp,
        time_machine: p.time_machine.clone(),
    };
    if change == AdvancedChange::default() {
        return json(advanced::get(client).await);
    }
    json(advanced::apply(client, &change, p.force).await)
}

pub async fn clear_cache(client: &UgosClient, p: &ForceOnlyParam) -> String {
    match smb::clear_cache(client, p.force).await {
        Ok(()) => "The SMB cache is cleared; the network service restarts".into(),
        Err(e) => failure(&e),
    }
}
