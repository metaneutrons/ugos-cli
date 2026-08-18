//! Certificate fingerprint store in the user's config directory
//! (`~/.config/ugos-cli/known_hosts.json` on Linux,
//! `~/Library/Application Support/ugos-cli/` on macOS).
//!
//! Holds one SHA-256 fingerprint per `host:port`, the way OpenSSH holds one
//! key per host. Shared by the CLI and the MCP server so both authenticate
//! a host the same way. See the [parent module](super) for why it exists.

use std::collections::BTreeMap;
use std::path::PathBuf;

use super::CertFingerprint;
use crate::error::{Result, UgosError};

/// Return the store's path.
fn store_path() -> Result<PathBuf> {
    let dir = dirs::config_dir()
        .ok_or_else(|| UgosError::Encryption("cannot determine config directory".into()))?
        .join("ugos-cli");
    Ok(dir.join("known_hosts.json"))
}

/// Key used for a host and port.
fn key(host: &str, port: u16) -> String {
    format!("{host}:{port}")
}

/// Read the whole store, treating an absent or unreadable file as empty.
fn read_all() -> BTreeMap<String, String> {
    store_path()
        .ok()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Look up the fingerprint recorded for a host.
///
/// A malformed entry is reported rather than ignored, because silently
/// falling back to first-use would defeat the check.
///
/// # Errors
///
/// Returns an error if the stored value is not a valid fingerprint.
pub fn get(host: &str, port: u16) -> Result<Option<CertFingerprint>> {
    let Some(stored) = read_all().get(&key(host, port)).cloned() else {
        return Ok(None);
    };
    let fp = CertFingerprint::from_hex(&stored).map_err(|e| {
        UgosError::Encryption(format!(
            "malformed fingerprint stored for {}: {e}",
            key(host, port)
        ))
    })?;
    Ok(Some(fp))
}

/// Record a fingerprint for a host, replacing any previous one.
///
/// # Errors
///
/// Returns an error if the store cannot be written.
pub fn put(host: &str, port: u16, fp: &CertFingerprint) -> Result<()> {
    let path = store_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| UgosError::Encryption(format!("creating {}: {e}", parent.display())))?;
    }
    let mut all = read_all();
    let _previous = all.insert(key(host, port), fp.to_hex());
    let json = serde_json::to_string_pretty(&all)?;
    std::fs::write(&path, json)
        .map_err(|e| UgosError::Encryption(format!("writing {}: {e}", path.display())))?;
    Ok(())
}

/// Drop the entry for a host. Returns whether one was present.
///
/// # Errors
///
/// Returns an error if the store cannot be written.
pub fn forget(host: &str, port: u16) -> Result<bool> {
    let mut all = read_all();
    if all.remove(&key(host, port)).is_none() {
        return Ok(false);
    }
    let path = store_path()?;
    let json = serde_json::to_string_pretty(&all)?;
    std::fs::write(&path, json)
        .map_err(|e| UgosError::Encryption(format!("writing {}: {e}", path.display())))?;
    Ok(true)
}

/// List every recorded host and fingerprint.
#[must_use]
pub fn list() -> Vec<(String, String)> {
    read_all().into_iter().collect()
}
