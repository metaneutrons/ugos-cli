//! Hardware, power, time and terminal settings of the NAS.
//!
//! These are calls on the core API that take a setting object, or the whole
//! of it, so each operation reads the object, edits a copy, sends it and
//! reads it again. The ones that restart the NAS, or open a way in from
//! outside, say so first and need `force`, as the web UI asks for a click.

pub mod basic;
pub mod clock;
pub mod gpu;
pub mod parse;
pub mod power;
pub mod terminal;
pub mod ups;

use serde_json::{Value, json};

use crate::client::UgosClient;
use crate::error::{Result, UgosError};

/// Reboot the NAS, as the web UI does after some changes.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn reboot(client: &UgosClient) -> Result<()> {
    let _: Value = client.post("desktop/reboot", &json!({})).await?;
    Ok(())
}

/// Power the NAS off. Nothing starts it again but the power button.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn poweroff(client: &UgosClient) -> Result<()> {
    let _: Value = client.post("desktop/poweroff", &json!({})).await?;
    Ok(())
}

/// The error for a change that reboots the NAS and was not forced.
pub(crate) fn needs_reboot(what: &str) -> UgosError {
    UgosError::NeedsForce(format!("{what} restarts the NAS"))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests;
