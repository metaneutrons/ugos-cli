//! Local images with the containers that use them.

use std::collections::HashMap;

use serde::Serialize;
use serde_json::json;

use crate::api::docker::DockerApi;
use crate::client::UgosClient;
use crate::error::Result;
use crate::types::common::ResultWrapper;
use crate::types::docker::{DockerImage, ImageUse};

/// An image and the containers made from it.
#[derive(Debug, Clone, Serialize)]
pub struct ImageView {
    /// The image.
    pub image: DockerImage,
    /// Names of the containers that use it. Empty for an unused image.
    pub used_by: Vec<String>,
}

/// The containers that use each of the images, by image ID.
///
/// # Errors
///
/// Returns the appropriate [`crate::error::UgosError`] on failure.
pub async fn usage(
    client: &UgosClient,
    images: &[DockerImage],
) -> Result<HashMap<String, Vec<String>>> {
    if images.is_empty() {
        return Ok(HashMap::new());
    }
    let list: Vec<_> = images
        .iter()
        .map(|i| json!({"imageId": i.image_id, "imageRef": i.image_ref}))
        .collect();
    let answer: ResultWrapper<Option<Vec<ImageUse>>> = client
        .post("docker/image/QueryAllContaners", &json!({ "list": list }))
        .await?;
    Ok(answer
        .result
        .unwrap_or_default()
        .into_iter()
        .map(|u| (u.image_id, u.names))
        .collect())
}

/// One page of the local images, each with the containers that use it.
///
/// # Errors
///
/// Returns the appropriate [`crate::error::UgosError`] on failure.
pub async fn images(client: &UgosClient, page: u32, page_size: u32) -> Result<Vec<ImageView>> {
    let list = client
        .docker_image_list(page, page_size)
        .await?
        .result
        .unwrap_or_default();
    let mut used = usage(client, &list).await?;
    Ok(list
        .into_iter()
        .map(|image| {
            let used_by = used.remove(&image.image_id).unwrap_or_default();
            ImageView { image, used_by }
        })
        .collect())
}
