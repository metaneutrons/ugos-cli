//! Firmware updates, configuration backups, resets, the about pages and the
//! thumbnail service.
//!
//! The calls that install firmware, restore a configuration or reset the NAS
//! change it for good or restart it. They say so first and need `force`, as
//! the web UI asks for a confirmation.

pub mod about;
pub mod backup;
pub mod firmware;
pub mod index;

use crate::error::UgosError;

pub(crate) fn refused(message: impl Into<String>) -> UgosError {
    UgosError::Refused(message.into())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests;
