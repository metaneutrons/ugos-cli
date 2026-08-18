//! Download Center API.
//!
//! A mixed bag: `add` and `checkLinks` take **multipart** bodies and are on
//! the UI's no-encryption whitelist, while the listings and the task controls
//! require the encrypted transport.

use crate::client::UgosClient;
use crate::error::Result;
use crate::types::common::ResultWrapper;
use crate::types::download::{DownloadPath, DownloadSpeed, DownloadTask};

/// Download Center operations.
#[allow(clippy::module_name_repetitions)]
pub trait DownloadApi {
    /// Check whether a link can be downloaded.
    fn download_check(&self, url: &str) -> impl Future<Output = Result<i64>> + Send;
    /// Queue a download. `save_dir` defaults to the configured directory.
    fn download_add(
        &self,
        url: &str,
        save_dir: Option<&str>,
    ) -> impl Future<Output = Result<()>> + Send;
    /// The configured target directory and its free space.
    fn download_path(&self) -> impl Future<Output = Result<DownloadPath>> + Send;
    /// Current transfer rates and task counts.
    fn download_speed(&self) -> impl Future<Output = Result<DownloadSpeed>> + Send;
    /// Tasks in progress.
    fn download_list(&self) -> impl Future<Output = Result<Vec<DownloadTask>>> + Send;
    /// Finished tasks.
    fn download_completed(&self) -> impl Future<Output = Result<Vec<DownloadTask>>> + Send;
    /// Remove a task, optionally deleting what it already fetched.
    ///
    /// `id` is the numeric `id` from the listing. The `task_id` string is
    /// rejected with `9999`.
    fn download_remove(
        &self,
        id: &str,
        delete_file: bool,
        running: bool,
    ) -> impl Future<Output = Result<()>> + Send;
}

impl DownloadApi for UgosClient {
    async fn download_check(&self, url: &str) -> Result<i64> {
        let form = reqwest::multipart::Form::new().text("download_url", url.to_owned());
        let resp: serde_json::Value = self
            .post_multipart("downloadCenter/download/checkLinks", form)
            .await?;
        Ok(resp
            .get("status")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(-1))
    }

    async fn download_add(&self, url: &str, save_dir: Option<&str>) -> Result<()> {
        let dir = match save_dir {
            Some(d) => d.to_owned(),
            None => self.download_path().await?.path,
        };
        let form = reqwest::multipart::Form::new()
            .text("is_batch", "false")
            .text("save_dir", dir)
            .text("download_url", url.to_owned());
        let _: serde_json::Value = self
            .post_multipart("downloadCenter/download/add", form)
            .await?;
        Ok(())
    }

    async fn download_path(&self) -> Result<DownloadPath> {
        self.get("downloadCenter/download/getPath").await
    }

    async fn download_speed(&self) -> Result<DownloadSpeed> {
        self.get("downloadCenter/download/globalSpeed").await
    }

    async fn download_list(&self) -> Result<Vec<DownloadTask>> {
        // `result` is null rather than an empty list when nothing is running.
        let resp: ResultWrapper<Option<Vec<DownloadTask>>> = self
            .get_encrypted("downloadCenter/download/getListV3", &[])
            .await?;
        Ok(resp.result.unwrap_or_default())
    }

    async fn download_remove(&self, id: &str, delete_file: bool, running: bool) -> Result<()> {
        let _: serde_json::Value = self
            .delete_encrypted(
                "downloadCenter/download/deleteTask",
                &[
                    ("ids", id),
                    ("delete_file", if delete_file { "true" } else { "false" }),
                    ("is_download", if running { "true" } else { "false" }),
                ],
            )
            .await?;
        Ok(())
    }

    async fn download_completed(&self) -> Result<Vec<DownloadTask>> {
        let resp: ResultWrapper<Option<Vec<DownloadTask>>> = self
            .get_encrypted("downloadCenter/complete/getListV2", &[])
            .await?;
        Ok(resp.result.unwrap_or_default())
    }
}
