//! Docker container and image management API.

use crate::client::UgosClient;
use crate::error::Result;
use crate::types::common::ResultWrapper;
use crate::types::docker::{
    ContainerDetail, ContainerPage, DockerImage, DockerOverview, ImagePage, Mirror,
};

/// Docker management operations on a UGOS NAS.
#[allow(clippy::module_name_repetitions)]
pub trait DockerApi {
    // ── Overview ────────────────────────────────────────────────────

    /// Get Docker engine overview (container/image counts, resource usage).
    fn docker_overview(&self) -> impl Future<Output = Result<DockerOverview>> + Send;

    /// Get Docker engine status ("online" / "offline").
    fn docker_engine_status(&self) -> impl Future<Output = Result<String>> + Send;

    // ── Containers ──────────────────────────────────────────────────

    /// List containers with pagination.
    fn container_list(
        &self,
        page: u32,
        page_size: u32,
    ) -> impl Future<Output = Result<ContainerPage>> + Send;

    /// Show detailed container configuration.
    fn container_show(&self, id: &str) -> impl Future<Output = Result<ContainerDetail>> + Send;

    /// Start a container.
    fn container_start(&self, id: &str) -> impl Future<Output = Result<()>> + Send;

    /// Stop a container.
    fn container_stop(&self, id: &str) -> impl Future<Output = Result<()>> + Send;

    /// Restart a container.
    fn container_restart(&self, id: &str) -> impl Future<Output = Result<()>> + Send;

    /// Kill a container.
    fn container_kill(&self, id: &str) -> impl Future<Output = Result<()>> + Send;

    /// Remove a container.
    fn container_remove(&self, id: &str) -> impl Future<Output = Result<()>> + Send;

    // ── Images ──────────────────────────────────────────────────────

    /// List local Docker images.
    fn docker_image_list(
        &self,
        page: u32,
        page_size: u32,
    ) -> impl Future<Output = Result<ImagePage>> + Send;

    /// Search Docker Hub for images.
    fn docker_image_search(
        &self,
        name: &str,
        page: u32,
        page_size: u32,
    ) -> impl Future<Output = Result<Vec<DockerImage>>> + Send;

    /// Pull/download an image.
    fn docker_image_download(
        &self,
        image: &str,
        tag: &str,
    ) -> impl Future<Output = Result<()>> + Send;

    /// Delete an image.
    fn docker_image_delete(&self, id: &str) -> impl Future<Output = Result<()>> + Send;

    // ── Registry ────────────────────────────────────────────────────

    /// List configured mirror sources.
    fn mirror_list(&self) -> impl Future<Output = Result<Vec<Mirror>>> + Send;
}

impl DockerApi for UgosClient {
    // ── Overview ────────────────────────────────────────────────────

    async fn docker_overview(&self) -> Result<DockerOverview> {
        self.get("docker/view/ObtainOverviewInfo").await
    }

    async fn docker_engine_status(&self) -> Result<String> {
        let resp: ResultWrapper<String> = self.get("docker/view/GetEngineStatus").await?;
        Ok(resp.result)
    }

    // ── Containers ──────────────────────────────────────────────────

    async fn container_list(&self, page: u32, page_size: u32) -> Result<ContainerPage> {
        let body = serde_json::json!({"pageNum": page, "pageSize": page_size});
        self.post("docker/container/ContainerListV2", &body).await
    }

    async fn container_show(&self, id: &str) -> Result<ContainerDetail> {
        self.get_with_params("docker/container/GetContainerById", &[("containerId", id)])
            .await
    }

    async fn container_start(&self, id: &str) -> Result<()> {
        let _: serde_json::Value = self
            .get_with_params("docker/container/StartContainer", &[("containerId", id)])
            .await?;
        Ok(())
    }

    async fn container_stop(&self, id: &str) -> Result<()> {
        let _: serde_json::Value = self
            .get_with_params("docker/container/StopContainer", &[("containerId", id)])
            .await?;
        Ok(())
    }

    async fn container_restart(&self, id: &str) -> Result<()> {
        let _: serde_json::Value = self
            .get_with_params("docker/container/RestartContainer", &[("containerId", id)])
            .await?;
        Ok(())
    }

    async fn container_kill(&self, id: &str) -> Result<()> {
        let _: serde_json::Value = self
            .get_with_params("docker/container/ContainerKill", &[("containerId", id)])
            .await?;
        Ok(())
    }

    async fn container_remove(&self, id: &str) -> Result<()> {
        let body = serde_json::json!({"containerId": id});
        let _: serde_json::Value = self.post("docker/container/RemoveContainer", &body).await?;
        Ok(())
    }

    // ── Images ──────────────────────────────────────────────────────

    async fn docker_image_list(&self, page: u32, page_size: u32) -> Result<ImagePage> {
        let body = serde_json::json!({"pageNum": page, "pageSize": page_size});
        self.post("docker/image/ShowLocalImageV2", &body).await
    }

    async fn docker_image_search(
        &self,
        name: &str,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<DockerImage>> {
        let resp: ResultWrapper<Vec<DockerImage>> = self
            .get_with_params(
                "docker/image/SearchImage",
                &[
                    ("name", name),
                    ("pageNum", &page.to_string()),
                    ("pageSize", &page_size.to_string()),
                ],
            )
            .await?;
        Ok(resp.result)
    }

    async fn docker_image_download(&self, image: &str, tag: &str) -> Result<()> {
        let body = serde_json::json!({"name": image, "tag": tag});
        let _: serde_json::Value = self.post("docker/image/DownloadImage", &body).await?;
        Ok(())
    }

    async fn docker_image_delete(&self, id: &str) -> Result<()> {
        let body = serde_json::json!({"id": id});
        let _: serde_json::Value = self.post("docker/image/DeleteImage", &body).await?;
        Ok(())
    }

    // ── Registry ────────────────────────────────────────────────────

    async fn mirror_list(&self) -> Result<Vec<Mirror>> {
        let resp: ResultWrapper<Vec<Mirror>> = self.get("docker/view/ShowMirrorList").await?;
        Ok(resp.result)
    }
}
