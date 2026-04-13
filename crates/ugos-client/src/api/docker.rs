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

    /// Create a container from a detail spec.
    fn container_create(&self, spec: &ContainerDetail) -> impl Future<Output = Result<()>> + Send;

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

    /// Get container logs.
    fn container_logs(
        &self,
        id: &str,
        lines: u32,
    ) -> impl Future<Output = Result<serde_json::Value>> + Send;

    /// Update a container.
    fn container_update(&self, spec: &ContainerDetail) -> impl Future<Output = Result<()>> + Send;

    /// Clone a container.
    fn container_clone(&self, id: &str, new_name: &str) -> impl Future<Output = Result<()>> + Send;

    /// Batch operate on containers (start/stop/restart/remove).
    fn container_batch(
        &self,
        ids: &[String],
        action: &str,
    ) -> impl Future<Output = Result<()>> + Send;

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

    /// Export an image to a path on the NAS.
    fn docker_image_export(&self, id: &str, path: &str) -> impl Future<Output = Result<()>> + Send;

    /// Load an image from a URL.
    fn docker_image_load_url(&self, url: &str) -> impl Future<Output = Result<()>> + Send;

    /// Load an image from a NAS path.
    fn docker_image_load_path(&self, path: &str) -> impl Future<Output = Result<()>> + Send;

    // ── Registry ────────────────────────────────────────────────────

    /// List configured mirror sources.
    fn mirror_list(&self) -> impl Future<Output = Result<Vec<Mirror>>> + Send;

    /// Add a mirror source.
    fn mirror_add(&self, alias: &str, address: &str) -> impl Future<Output = Result<()>> + Send;

    /// Delete a mirror source.
    fn mirror_delete(&self, id: i64) -> impl Future<Output = Result<()>> + Send;

    /// Switch active mirror source.
    fn mirror_switch(&self, id: i64) -> impl Future<Output = Result<()>> + Send;

    // ── Compose ─────────────────────────────────────────────────────

    /// List offline containers for a compose project.
    fn compose_containers(
        &self,
        project: &str,
    ) -> impl Future<Output = Result<serde_json::Value>> + Send;

    // ── Proxy ───────────────────────────────────────────────────────

    /// Get HTTP proxy configuration.
    fn docker_proxy_get(&self) -> impl Future<Output = Result<serde_json::Value>> + Send;

    /// Set HTTP proxy configuration.
    fn docker_proxy_set(
        &self,
        proxy: &serde_json::Value,
    ) -> impl Future<Output = Result<()>> + Send;
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

    async fn container_create(&self, spec: &ContainerDetail) -> Result<()> {
        let _: serde_json::Value = self.post("docker/container/CreateContainer", spec).await?;
        Ok(())
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

    async fn container_logs(&self, id: &str, lines: u32) -> Result<serde_json::Value> {
        let body = serde_json::json!({"containerId": id, "tail": lines});
        self.post("docker/container/ShowContainerLogs", &body).await
    }

    async fn container_update(&self, spec: &ContainerDetail) -> Result<()> {
        let _: serde_json::Value = self.post("docker/container/UpdateContainer", spec).await?;
        Ok(())
    }

    async fn container_clone(&self, id: &str, new_name: &str) -> Result<()> {
        let body = serde_json::json!({"containerId": id, "containerName": new_name});
        let _: serde_json::Value = self.post("docker/container/CloneContainer", &body).await?;
        Ok(())
    }

    async fn container_batch(&self, ids: &[String], action: &str) -> Result<()> {
        let body = serde_json::json!({"containerIds": ids, "operate": action});
        let _: serde_json::Value = self
            .post("docker/container/BatchOperateContainer", &body)
            .await?;
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

    async fn docker_image_export(&self, id: &str, path: &str) -> Result<()> {
        let body = serde_json::json!({"id": id, "path": path});
        let _: serde_json::Value = self.post("docker/image/ImageExport", &body).await?;
        Ok(())
    }

    async fn docker_image_load_url(&self, url: &str) -> Result<()> {
        let body = serde_json::json!({"url": url});
        let _: serde_json::Value = self.post("docker/image/LoadUrl", &body).await?;
        Ok(())
    }

    async fn docker_image_load_path(&self, path: &str) -> Result<()> {
        let body = serde_json::json!({"path": path});
        let _: serde_json::Value = self.post("docker/image/LoadPath", &body).await?;
        Ok(())
    }

    // ── Registry ────────────────────────────────────────────────────

    async fn mirror_list(&self) -> Result<Vec<Mirror>> {
        let resp: ResultWrapper<Vec<Mirror>> = self.get("docker/view/ShowMirrorList").await?;
        Ok(resp.result)
    }

    async fn mirror_add(&self, alias: &str, address: &str) -> Result<()> {
        let body = serde_json::json!({"alias": alias, "address": address});
        let _: serde_json::Value = self.post("docker/view/AddMirrorSource", &body).await?;
        Ok(())
    }

    async fn mirror_delete(&self, id: i64) -> Result<()> {
        let _: serde_json::Value = self
            .get_with_params("docker/view/DeleteMirror", &[("id", &id.to_string())])
            .await?;
        Ok(())
    }

    async fn mirror_switch(&self, id: i64) -> Result<()> {
        let _: serde_json::Value = self
            .get_with_params("docker/view/SwitchMirrorSource", &[("id", &id.to_string())])
            .await?;
        Ok(())
    }

    // ── Compose ─────────────────────────────────────────────────────

    async fn compose_containers(&self, project: &str) -> Result<serde_json::Value> {
        self.get_with_params(
            "docker/compose/ShowOfflineContainers",
            &[("projectName", project)],
        )
        .await
    }

    // ── Proxy ───────────────────────────────────────────────────────

    async fn docker_proxy_get(&self) -> Result<serde_json::Value> {
        self.get("docker/view/GetHttpProxy").await
    }

    async fn docker_proxy_set(&self, proxy: &serde_json::Value) -> Result<()> {
        let _: serde_json::Value = self.post("docker/view/SetHttpProxy", proxy).await?;
        Ok(())
    }
}
