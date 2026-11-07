//! The file services: SMB, FTP, NFS, rsync and `WebDAV`.
//!
//! All five work the same way. There is no status call: whether a service
//! runs is a field of `file/{service}/config`, a service is switched on or
//! re-applied by posting its complete configuration to `file/{service}/start`,
//! and switched off by posting nothing to `file/{service}/stop`. So every
//! change reads the configuration, edits a copy, posts it and reads it again,
//! and keeps the fields this client does not model.
//!
//! Settings are named by short aliases that map to the API's field names, and
//! checked against the rules of the web UI (ranges, reserved ports, forbidden
//! combinations) before anything is sent.

pub mod advanced;
pub mod fields;
pub mod ftp;
pub mod nfs;
pub mod rsync;
pub mod smb;
pub mod webdav;

use serde_json::{Map, Value, json};

use crate::client::UgosClient;
use crate::error::{Result, UgosError};

pub use fields::{Field, Kind};

/// A service configuration as UGOS keeps it.
pub type Config = Map<String, Value>;

/// One of the file services.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Service {
    /// Windows and macOS file sharing.
    Smb,
    /// FTP and FTPS.
    Ftp,
    /// NFS.
    Nfs,
    /// rsync daemon.
    Rsync,
    /// `WebDAV`.
    Webdav,
}

pub(crate) fn refused(message: impl Into<String>) -> UgosError {
    UgosError::Refused(message.into())
}

pub(crate) fn flag(config: &Config, key: &str) -> bool {
    config.get(key).and_then(Value::as_bool).unwrap_or(false)
}

pub(crate) fn number(config: &Config, key: &str) -> i64 {
    config.get(key).and_then(Value::as_i64).unwrap_or(0)
}

impl Service {
    /// The name people use for it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Smb => "SMB",
            Self::Ftp => "FTP",
            Self::Nfs => "NFS",
            Self::Rsync => "rsync",
            Self::Webdav => "WebDAV",
        }
    }

    /// The path segment of its calls.
    const fn path(self) -> &'static str {
        match self {
            Self::Smb => "smb",
            Self::Ftp => "ftp",
            Self::Nfs => "nfs",
            Self::Rsync => "rsync",
            Self::Webdav => "webdav",
        }
    }

    /// Whether the configuration says the service runs.
    #[must_use]
    pub fn running(self, config: &Config) -> bool {
        match self {
            Self::Ftp => flag(config, "enable_ftp") || flag(config, "enable_ftps"),
            Self::Nfs => flag(config, "enableNfsServer"),
            _ => flag(config, "status"),
        }
    }

    /// The settings it accepts.
    #[must_use]
    pub const fn fields(self) -> &'static [Field] {
        match self {
            Self::Smb => smb::FIELDS,
            Self::Ftp => ftp::FIELDS,
            Self::Nfs => nfs::FIELDS,
            Self::Rsync => rsync::FIELDS,
            Self::Webdav => webdav::FIELDS,
        }
    }
}

/// The configuration of a service.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn config(client: &UgosClient, service: Service) -> Result<Config> {
    let value: Value = client
        .get(&format!("file/{}/config", service.path()))
        .await?;
    match value {
        Value::Object(map) => Ok(map),
        _ => Err(refused(format!(
            "UGOS gave no configuration for {}",
            service.name()
        ))),
    }
}

/// Ports no file service may take.
const RESERVED: [i64; 7] = [80, 20, 21, 22, 23, 443, 3493];

/// A port to check with UGOS before it is used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PortUse {
    pub module: &'static str,
    /// The configuration key that holds the port.
    pub key: &'static str,
    /// NFS tells its two helper ports apart by name.
    pub name: Option<&'static str>,
    pub port: i64,
}

/// The ports of a configuration that changed, or all of them when the
/// service is being switched on.
pub(crate) fn port_check(use_: &PortUse, skip_ftp_21: bool) -> Result<()> {
    if !(1..=65535).contains(&use_.port) {
        return Err(refused("a port is 1 to 65535"));
    }
    let reserved = RESERVED
        .iter()
        .copied()
        .filter(|p| !(skip_ftp_21 && *p == 21));
    if reserved.into_iter().any(|p| p == use_.port) {
        return Err(refused(format!(
            "port {} is taken by the system",
            use_.port
        )));
    }
    Ok(())
}

async fn check_port(client: &UgosClient, use_: &PortUse) -> Result<()> {
    port_check(use_, use_.module == "ftp")?;
    let port = use_.port.to_string();
    let mut params = vec![("port", port.as_str()), ("module", use_.module)];
    if let Some(name) = use_.name {
        params.push(("name", name));
    }
    let answer: Result<Value> = client.get_with_params("file/checkport", &params).await;
    match answer {
        Ok(_) => Ok(()),
        Err(
            UgosError::Api { .. } | UgosError::OperationFailed(_) | UgosError::ParameterError(_),
        ) => Err(refused(format!("port {} is already in use", use_.port))),
        Err(e) => Err(e),
    }
}

/// What a change needs besides the settings.
#[derive(Debug, Default)]
pub struct Change {
    /// The settings to change, as alias and text.
    pub settings: Vec<(String, String)>,
    /// A password, for rsync.
    pub password: Option<String>,
    /// Go on although the change is one the web UI asks a confirmation for.
    pub force: bool,
}

/// Switch a service on, or change a running one, and read it back.
///
/// `enable` says the call switches the service on (and applies the settings
/// given); without it the service has to run already, because UGOS only keeps
/// settings that are posted together with `start`.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] for a setting the web UI would refuse,
/// [`UgosError::NeedsForce`] for a change it asks a confirmation for, and the
/// appropriate [`UgosError`] on failure.
pub async fn apply(
    client: &UgosClient,
    service: Service,
    enable: bool,
    change: &Change,
) -> Result<Config> {
    let current = config(client, service).await?;
    let was_on = service.running(&current);
    if !enable && !was_on {
        return Err(refused(format!(
            "{} is off: start it, with these settings, to apply them",
            service.name()
        )));
    }
    let patch = fields::patch(service.fields(), &change.settings)?;
    let mut next = current.clone();
    for (key, value) in &patch {
        let _ = next.insert(key.clone(), value.clone());
    }
    if enable && service != Service::Ftp {
        mark_on(service, &mut next);
    }
    let ports = match service {
        Service::Smb => smb::finish(&current, &mut next, change.force)?,
        Service::Ftp => ftp::finish(&current, &mut next, enable, change.force)?,
        Service::Nfs => nfs::finish(&current, &mut next, enable, change.force)?,
        Service::Rsync => {
            rsync::finish(
                client,
                &current,
                &mut next,
                enable,
                change.password.as_deref(),
            )
            .await?
        }
        Service::Webdav => webdav::finish(&current, &mut next)?,
    };
    for port in &ports {
        let changed = !was_on || number(&current, port.key) != port.port;
        if changed {
            check_port(client, port).await?;
        }
    }
    post_start(client, service, &next).await?;
    let after = config(client, service).await?;
    verify(service, &patch, &after, enable)?;
    Ok(after)
}

fn mark_on(service: Service, config: &mut Config) {
    let key = match service {
        Service::Nfs => "enableNfsServer",
        _ => "status",
    };
    let _ = config.insert(key.into(), Value::Bool(true));
}

async fn post_start(client: &UgosClient, service: Service, body: &Config) -> Result<()> {
    let path = format!("file/{}/start", service.path());
    let answer: Result<Value> = client.post(&path, body).await;
    match answer {
        Ok(_) => Ok(()),
        Err(UgosError::Api {
            code: 8202 | 8203, ..
        }) => Err(refused(format!(
            "UGOS could not start {}; the service stays off",
            service.name()
        ))),
        Err(e) => Err(e),
    }
}

fn same(sent: &Value, got: &Value) -> bool {
    sent == got
        || match (sent.as_i64(), got.as_i64()) {
            (Some(a), Some(b)) => a == b,
            _ => false,
        }
}

fn verify(service: Service, patch: &Config, after: &Config, enable: bool) -> Result<()> {
    if enable && !service.running(after) {
        return Err(refused(format!(
            "UGOS said yes but {} is not running",
            service.name()
        )));
    }
    for (key, sent) in patch {
        // The server may fill a zero with its own default, and keeps the
        // password to itself.
        let got = after.get(key).unwrap_or(&Value::Null);
        let unset = sent.as_i64() == Some(0) || sent == &json!("");
        if !same(sent, got) && !unset {
            return Err(refused(format!(
                "UGOS said yes but {key} is still {got}, not {sent}"
            )));
        }
    }
    Ok(())
}

/// Switch a service off.
///
/// # Errors
///
/// Returns [`UgosError::NeedsForce`] for SMB while its discovery services run
/// (the web UI asks first), and the appropriate [`UgosError`] on failure.
pub async fn stop(client: &UgosClient, service: Service, force: bool) -> Result<Config> {
    let current = config(client, service).await?;
    if !service.running(&current) {
        return Ok(current);
    }
    if service == Service::Smb && !force {
        let advanced = advanced::get(client).await?;
        if advanced.bonjour || advanced.wsdd2 {
            return Err(UgosError::NeedsForce(
                "Bonjour or WS-Discovery announce the SMB shares, and go quiet with it".into(),
            ));
        }
    }
    let _: Value = client
        .post(&format!("file/{}/stop", service.path()), &json!({}))
        .await?;
    let after = config(client, service).await?;
    if service.running(&after) {
        return Err(refused(format!(
            "UGOS said yes but {} is still running",
            service.name()
        )));
    }
    Ok(after)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests;
