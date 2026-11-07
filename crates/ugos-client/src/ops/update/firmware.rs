//! The firmware: what is available, how the NAS looks for it, installing it.

use std::path::Path;

use md5::{Digest, Md5};
use serde_json::{Value, json};

use super::refused;
use crate::client::UgosClient;
use crate::error::{Result, UgosError};
use crate::types::update::{FirmwareConfig, FirmwareStatus, ManualStatus, VersionFlag};

/// One chunk of a manual upload, as the web UI cuts it.
pub const CHUNK: usize = 10 * 1024 * 1024;

/// The firmware state.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn status(client: &UgosClient) -> Result<FirmwareStatus> {
    client.get("firmware/status").await
}

/// Whether a newer firmware exists.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn is_new(client: &UgosClient) -> Result<VersionFlag> {
    client.get("firmware/version/is_new").await
}

/// How the NAS looks for updates.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn config(client: &UgosClient) -> Result<FirmwareConfig> {
    client.get("firmware/config").await
}

/// How often the NAS looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interval {
    /// UGOS decides.
    Default,
    /// Every `value` hours, days or months (`unit` 1, 2, 3).
    Every {
        /// 1 to 72, whatever the unit.
        value: i64,
        /// `1` hours, `2` days, `3` months.
        unit: i64,
    },
}

/// What to change about the update settings.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ConfigChange {
    /// `0` tell me, `1` install important updates, `2` install any update.
    pub auto_update: Option<i64>,
    /// The check interval.
    pub interval: Option<Interval>,
}

/// Change the update settings and read them back.
///
/// # Errors
///
/// Returns [`UgosError::NeedsForce`] when the change lets the NAS install
/// updates, and so restart, without being asked.
pub async fn set_config(
    client: &UgosClient,
    change: &ConfigChange,
    force: bool,
) -> Result<FirmwareConfig> {
    let mut next = config(client).await?;
    let before = next.auto_update;
    if let Some(auto) = change.auto_update {
        if !(0..=2).contains(&auto) {
            return Err(refused("auto-update is 0, 1 or 2"));
        }
        next.auto_update = auto;
    }
    match change.interval {
        Some(Interval::Default) => next.custom_interval_time = false,
        Some(Interval::Every { value, unit }) => {
            if !(1..=72).contains(&value) {
                return Err(refused("the interval is 1 to 72"));
            }
            if !(1..=3).contains(&unit) {
                return Err(refused("the unit is hours, days or months"));
            }
            next.custom_interval_time = true;
            next.interval_time = value;
            next.unit = unit;
        }
        None => {}
    }
    if next.auto_update > before && !force {
        return Err(UgosError::NeedsForce(
            "the NAS then installs updates by itself, and restarts".into(),
        ));
    }
    let body = json!({
        "auto_update": next.auto_update,
        "custom_interval_time": next.custom_interval_time,
        "interval_time": next.interval_time,
        "unit": if next.unit == 0 { 1 } else { next.unit },
    });
    let _: Value = client.post("firmware/config", &body).await?;
    let after = config(client).await?;
    if after.auto_update != next.auto_update
        || after.custom_interval_time != next.custom_interval_time
    {
        return Err(refused(
            "UGOS said yes but the update settings did not change",
        ));
    }
    Ok(after)
}

fn install_error(e: UgosError) -> UgosError {
    match e {
        UgosError::Api { code: 1509, .. } => refused(
            "UGOS does not trust the signing key of this firmware: import its public key first",
        ),
        UgosError::Api {
            code: 9000 | 9001 | 1505,
            msg,
        } => refused(format!("UGOS refused: {msg}")),
        other => other,
    }
}

/// Install the update that UGOS found. The NAS restarts when it is done.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] when no update is ready, and
/// [`UgosError::NeedsForce`] without `force`.
pub async fn install(client: &UgosClient, force: bool) -> Result<FirmwareStatus> {
    let now = status(client).await?;
    if now.running {
        return Err(refused("an update is running already"));
    }
    if now.current_stat != 2 {
        return Err(refused("there is no update to install"));
    }
    if !force {
        let version = now
            .remote_base
            .as_ref()
            .map_or("the update", |r| r.version.as_str());
        return Err(UgosError::NeedsForce(format!(
            "installing {version} restarts the NAS"
        )));
    }
    let _: Value = client
        .post("firmware/install", &json!({}))
        .await
        .map_err(install_error)?;
    status(client).await
}

/// Clear the stored failure of the last install.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn clean(client: &UgosClient) -> Result<()> {
    let _: Value = client.get("firmware/status/clean").await?;
    Ok(())
}

/// Trust a firmware signing key, from the text of a `.pub` file.
///
/// # Errors
///
/// Returns [`UgosError::NeedsForce`] without `force`: it decides which
/// firmware the NAS will install.
pub async fn import_key(client: &UgosClient, file: &Path, force: bool) -> Result<()> {
    if file.extension().and_then(|e| e.to_str()) != Some("pub") {
        return Err(refused("a public key file ends in .pub"));
    }
    let text = std::fs::read_to_string(file)
        .map_err(|e| UgosError::io(format!("reading '{}'", file.display()), e))?;
    if text.trim().is_empty() {
        return Err(refused("the key file is empty"));
    }
    if !force {
        return Err(UgosError::NeedsForce(
            "a trusted key decides which firmware the NAS installs".into(),
        ));
    }
    let _: Value = client
        .post(
            "firmware/pubkey/import",
            &json!({ "data": text, "pkgName": "import" }),
        )
        .await?;
    Ok(())
}

/// Progress of a manual install.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn manual_status(client: &UgosClient) -> Result<ManualStatus> {
    client.get("firmware/manual/status").await
}

/// The checks the web UI makes before it sends a firmware file, on the name
/// and the size: the number of chunks it is cut in.
///
/// # Errors
///
/// Refuses a file that is not an `.img` or is empty.
pub fn check_image(name: &str, size: u64) -> Result<usize> {
    if Path::new(name).extension().and_then(|e| e.to_str()) != Some("img") {
        return Err(refused("a firmware file ends in .img"));
    }
    if size == 0 {
        return Err(refused("the firmware file is empty"));
    }
    Ok(usize::try_from(size.div_ceil(CHUNK as u64)).unwrap_or(usize::MAX))
}

/// Upload a firmware image in chunks. UGOS installs it and restarts the NAS.
///
/// `progress` is told how many chunks went out.
///
/// # Errors
///
/// Returns [`UgosError::NeedsForce`] without `force`, and
/// [`UgosError::Refused`] for a file the web UI would not send.
pub async fn upload(
    client: &UgosClient,
    file: &Path,
    force: bool,
    progress: &(dyn Fn(usize, usize) + Send + Sync),
) -> Result<()> {
    let name = file
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("firmware.img")
        .to_owned();
    let size = std::fs::metadata(file)
        .map_err(|e| UgosError::io(format!("reading '{}'", file.display()), e))?
        .len();
    let chunks = check_image(&name, size)?;
    if !force {
        return Err(UgosError::NeedsForce(
            "installing a firmware image flashes the NAS and restarts it".into(),
        ));
    }
    let bytes = std::fs::read(file)
        .map_err(|e| UgosError::io(format!("reading '{}'", file.display()), e))?;
    let md5 = hex::encode(Md5::digest(&bytes));
    for (index, part) in bytes.chunks(CHUNK).enumerate() {
        let form = reqwest::multipart::Form::new()
            .part(
                "file",
                reqwest::multipart::Part::bytes(part.to_vec()).file_name(name.clone()),
            )
            .text("chunks", chunks.to_string())
            .text("chunk", index.to_string())
            .text("md5", md5.clone());
        let _: Value = client
            .post_multipart("firmware/manual", form)
            .await
            .map_err(install_error)?;
        progress(index + 1, chunks);
    }
    Ok(())
}
