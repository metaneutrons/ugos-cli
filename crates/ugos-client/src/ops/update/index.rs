//! The thumbnail service of the index service page.

use serde_json::{Value, json};

use super::refused;
use crate::client::UgosClient;
use crate::error::{Result, UgosError};
use crate::types::update::{PregenList, ThumbStatus};

const APP: &str = "com.ugreen.ctlmgr";

/// The state of the thumbnail service.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`](crate::error::UgosError) on failure.
pub async fn status(client: &UgosClient) -> Result<ThumbStatus> {
    client.get("api/ugreen.thumb/getThumbStatus").await
}

/// The folder ranges thumbnails are made for.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`](crate::error::UgosError) on failure.
pub async fn ranges(client: &UgosClient) -> Result<PregenList> {
    client
        .get_with_params("api/ugreen.thumb/getPregenList", &[("app_id", APP)])
        .await
}

/// Pause for 1, 2 or 5 hours.
///
/// # Errors
///
/// Refuses any other time.
pub async fn pause(client: &UgosClient, hours: i64) -> Result<ThumbStatus> {
    if ![1, 2, 5].contains(&hours) {
        return Err(refused("the thumbnail service pauses for 1, 2 or 5 hours"));
    }
    let body = json!({ "target_status": 3, "count_down_time": hours * 3600 });
    let sent: Result<Value> = client.post("api/ugreen.thumb/setThumbStatus", &body).await;
    match sent {
        Ok(_) => {}
        // 1109: no task in the queue, so there is nothing to pause.
        Err(UgosError::Api { code: 1109, .. }) => {
            return Err(refused(
                "no thumbnails are being made, so there is nothing to pause",
            ));
        }
        Err(e) => return Err(e),
    }
    let after = status(client).await?;
    if after.status != 3 {
        return Err(refused(
            "UGOS said yes but the thumbnail service is not paused",
        ));
    }
    Ok(after)
}

/// Resume after a pause.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`](crate::error::UgosError) on failure.
pub async fn resume(client: &UgosClient) -> Result<ThumbStatus> {
    let _: Value = client
        .post(
            "api/ugreen.thumb/setThumbStatus",
            &json!({ "target_status": 1 }),
        )
        .await?;
    status(client).await
}

/// Make thumbnails again, for all ranges or the ones with these ids.
///
/// # Errors
///
/// Refuses a call that names neither.
pub async fn regenerate(
    client: &UgosClient,
    all: bool,
    only_missing: bool,
    ids: &[i64],
) -> Result<ThumbStatus> {
    if !all && ids.is_empty() {
        return Err(refused(
            "name the ranges to rebuild, or ask for all of them",
        ));
    }
    let body = json!({
        "app_id": APP, "regen_all": all, "only_for_missing": only_missing, "ids": ids,
    });
    let _: Value = client.post("api/ugreen.thumb/regenerate", &body).await?;
    status(client).await
}
