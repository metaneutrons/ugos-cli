//! SAN Manager: iSCSI targets, LUNs, initiator groups and the global setting.
//!
//! Checks follow the web UI's rules (names, IQNs, CHAP users and passwords,
//! sizes). What cuts a connection, deletes data or cannot be undone says so
//! first and needs `force`, as the web UI asks a confirmation.

pub mod check;
pub mod group;
pub mod lun;
pub mod setting;
pub mod target;

use std::future::Future;

use serde::de::DeserializeOwned;

use crate::client::UgosClient;
use crate::error::{Result, UgosError};
use crate::types::san::Page;

pub(crate) fn refused(message: impl Into<String>) -> UgosError {
    UgosError::Refused(message.into())
}

/// Wait for something UGOS is deleting to be gone: it passes through states
/// where it is still listed. Gives up after about ten seconds.
pub(crate) async fn gone<F, Fut>(still_there: F) -> Result<bool>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<bool>>,
{
    let mut pause = std::time::Duration::from_millis(300);
    for _ in 0..8 {
        if !still_there().await? {
            return Ok(true);
        }
        tokio::time::sleep(pause).await;
        pause = (pause * 2).min(std::time::Duration::from_secs(3));
    }
    Ok(false)
}

/// Every item of a list, in one request.
pub(crate) async fn everything<T: DeserializeOwned + Send>(
    client: &UgosClient,
    path: &str,
    extra: &[(&str, &str)],
) -> Result<Vec<T>> {
    let mut params = vec![("page", "1"), ("page_size", "999999")];
    params.extend_from_slice(extra);
    let page: Page<T> = client.get_with_params(path, &params).await?;
    Ok(page.results)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests;
