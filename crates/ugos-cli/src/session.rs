//! Session token cache at `~/.config/ugos-cli/session.json`.

use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Cached session data persisted to disk.
#[derive(Debug, Serialize, Deserialize)]
pub struct CachedSession {
    /// NAS host used for this session.
    pub host: String,
    /// HTTPS port.
    pub port: u16,
    /// Username.
    pub user: String,
    /// Session token.
    pub token: String,
}

/// Return the session cache file path.
fn cache_path() -> Result<PathBuf> {
    let dir = dirs::config_dir()
        .context("cannot determine config directory")?
        .join("ugos-cli");
    Ok(dir.join("session.json"))
}

/// Load a cached session if it exists and matches the given host/user.
///
/// Returns `None` if the file doesn't exist or host/user don't match.
pub fn load(host: &str, port: u16, user: &str) -> Option<CachedSession> {
    let path = cache_path().ok()?;
    let data = std::fs::read_to_string(&path).ok()?;
    let cached: CachedSession = serde_json::from_str(&data).ok()?;

    if cached.host == host && cached.port == port && cached.user == user {
        Some(cached)
    } else {
        None
    }
}

/// Save a session to the cache file.
pub fn save(session: &CachedSession) -> Result<()> {
    let path = cache_path()?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).context("creating config directory")?;
    }
    let json = serde_json::to_string_pretty(session).context("serializing session")?;
    std::fs::write(&path, json).context("writing session cache")?;
    Ok(())
}
