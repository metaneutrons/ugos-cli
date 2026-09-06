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
    /// PEM key for encrypted endpoints; empty in caches from older versions.
    #[serde(default)]
    pub public_key: String,
    /// When the session was created (Unix timestamp).
    #[serde(default)]
    pub created_at: i64,
}

/// Maximum session age before we force a fresh login (25 minutes).
const MAX_SESSION_AGE_SECS: i64 = 25 * 60;

/// Current Unix timestamp in seconds.
#[allow(clippy::cast_possible_wrap)]
pub fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
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
        // Reject stale sessions.
        let now = unix_now();
        if cached.created_at > 0 && (now - cached.created_at) > MAX_SESSION_AGE_SECS {
            tracing::debug!("cached session expired (age > {MAX_SESSION_AGE_SECS}s)");
            return None;
        }
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
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))
                .context("restricting config directory")?;
        }
    }
    let json = serde_json::to_string_pretty(session).context("serializing session")?;

    // Die Rechte gehoeren in den erzeugenden Aufruf. `write` gefolgt von
    // `set_permissions` legte die Datei mit der Umask an, schrieb das Token
    // hinein und engte erst danach ein; in diesem Fenster kann jeder lokale
    // Nutzer das Token lesen.
    let mut opts = std::fs::OpenOptions::new();
    let _ = opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let _ = opts.mode(0o600);
    }
    let mut file = opts.open(&path).context("writing session cache")?;
    std::io::Write::write_all(&mut file, json.as_bytes()).context("writing session cache")?;

    Ok(())
}
