//! File manager API.
//!
//! Listing and the write operations live on the v2 API and require the
//! encrypted transport; volumes and the upload endpoints are v1 and plain.

use crate::client::UgosClient;
use crate::error::Result;
use crate::types::common::ResultWrapper;
use crate::types::files::{DirListing, FileEntry, Volume};

/// File manager operations.
#[allow(clippy::module_name_repetitions)]
pub trait FilesApi {
    /// List a directory.
    fn fs_list(&self, path: &str) -> impl Future<Output = Result<Vec<FileEntry>>> + Send;
    /// List the storage volumes.
    fn fs_volumes(&self) -> impl Future<Output = Result<Vec<Volume>>> + Send;
    /// Create a directory, named by its full target path.
    fn fs_mkdir(&self, path: &str) -> impl Future<Output = Result<()>> + Send;
    /// Delete paths. Without `forever` they go to the recycle bin.
    fn fs_remove(&self, paths: &[String], forever: bool)
    -> impl Future<Output = Result<()>> + Send;
    /// Rename a single entry.
    fn fs_rename(&self, path: &str, new_name: &str) -> impl Future<Output = Result<()>> + Send;
}

impl FilesApi for UgosClient {
    async fn fs_list(&self, path: &str) -> Result<Vec<FileEntry>> {
        let body = serde_json::json!({
            "path": path,
            "page": 1,
            "limit": 1000,
            "is_shield_recycle": false,
        });
        let listing: DirListing = self
            .post_encrypted("v2/filemgr/getDirFileListV2", &body)
            .await?;
        Ok(listing.right_files.files.unwrap_or_default())
    }

    async fn fs_volumes(&self) -> Result<Vec<Volume>> {
        let resp: ResultWrapper<Vec<Volume>> = self.get("filemgr/getVolumes").await?;
        Ok(resp.result)
    }

    async fn fs_mkdir(&self, path: &str) -> Result<()> {
        // The full target path, not a parent plus a name — the latter fails
        // with 1365.
        let body = serde_json::json!({"path": path});
        let _: serde_json::Value = self
            .post_encrypted("v2/filemgr/createFolder", &body)
            .await?;
        Ok(())
    }

    async fn fs_remove(&self, paths: &[String], forever: bool) -> Result<()> {
        let body = serde_json::json!({"paths": paths, "forever": forever});
        let _: serde_json::Value = self.post_encrypted("v2/filemgr/delPaths", &body).await?;
        Ok(())
    }

    async fn fs_rename(&self, path: &str, new_name: &str) -> Result<()> {
        let body = serde_json::json!({"path": path, "new_name": new_name});
        let _: serde_json::Value = self.post_encrypted("v2/filemgr/rename", &body).await?;
        Ok(())
    }
}
