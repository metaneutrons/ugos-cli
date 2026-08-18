//! File manager API.
//!
//! Listing and the write operations live on the v2 API and require the
//! encrypted transport; volumes and the upload endpoints are v1 and plain.

use crate::client::UgosClient;
use crate::error::{Result, UgosError};
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

    /// Download a file to a local path.
    ///
    /// `progress` is called with the bytes written so far.
    fn fs_download(
        &self,
        remote: &str,
        local: &std::path::Path,
        progress: &(dyn Fn(u64) + Send + Sync),
    ) -> impl Future<Output = Result<u64>> + Send;

    /// Upload a local file into a directory on the NAS.
    ///
    /// Returns the path the file landed at.
    fn fs_upload(
        &self,
        local: &std::path::Path,
        remote_dir: &str,
    ) -> impl Future<Output = Result<String>> + Send;
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

    async fn fs_download(
        &self,
        remote: &str,
        local: &std::path::Path,
        progress: &(dyn Fn(u64) + Send + Sync),
    ) -> Result<u64> {
        use std::io::Write;

        // The v1 endpoint answers with the file itself and takes plain
        // requests, so no token dance is needed here.
        let mut resp = self
            .get_bytes("filemgr/downloadFile", &[("paths", remote)])
            .await?;

        let mut out = std::fs::File::create(local)
            .map_err(|e| UgosError::Encryption(format!("creating '{}': {e}", local.display())))?;
        let mut written = 0u64;
        while let Some(chunk) = resp.chunk().await? {
            out.write_all(&chunk)
                .map_err(|e| UgosError::Encryption(format!("writing download: {e}")))?;
            written += chunk.len() as u64;
            progress(written);
        }
        Ok(written)
    }

    async fn fs_upload(&self, local: &std::path::Path, remote_dir: &str) -> Result<String> {
        let name = local
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .ok_or_else(|| UgosError::Encryption("source has no file name".into()))?;

        let meta = std::fs::metadata(local)
            .map_err(|e| UgosError::Encryption(format!("reading '{}': {e}", local.display())))?;
        let size = meta.len();
        let mtime = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_secs());

        // Step one announces the transfer; step two repeats the same uuid.
        let uuid = upload_uuid();
        let form = reqwest::multipart::Form::new()
            .text("uuid", uuid.clone())
            .text("dir", remote_dir.to_owned())
            .text("action_type", "0")
            .text("size", size.to_string())
            .text("begin_size", "0")
            .text("current_size", "0")
            .text("change_time", mtime.to_string())
            .text("filename", name.clone())
            .text("resume", "true")
            .text("first_request", "true");
        let announced: Result<serde_json::Value> =
            self.post_multipart("filemgr/fileUpload", form).await;
        let _ = announced
            .map_err(|e| UgosError::OperationFailed(format!("upload handshake rejected: {e}")))?;

        // Step two sends the bytes, with the same metadata repeated in a
        // `ug-param` header. `dir` is URL-encoded in there, unlike in step one.
        let bytes = std::fs::read(local)
            .map_err(|e| UgosError::Encryption(format!("reading '{}': {e}", local.display())))?;
        let ug_param = serde_json::json!({
            "uuid": uuid,
            "file_name": name,
            "action_type": 0,
            "size": size,
            "current_size": size,
            "resume": true,
            "dir": crate::crypto::urlencode_component(remote_dir),
            "change_time": mtime,
            "is_live_photo": false,
            "first_request": false,
            "begin_size": 0,
        });
        let sent: Result<serde_json::Value> = self
            .post_bytes("filemgr/fileUploadV2", bytes, &name, &ug_param)
            .await;
        let placed = sent.map_err(|e| {
            UgosError::OperationFailed(format!("sending file contents failed: {e}"))
        })?;

        Ok(placed
            .get("result")
            .and_then(|v| v.as_str())
            .map_or_else(|| format!("{remote_dir}/{name}"), ToOwned::to_owned))
    }
}

/// Build the identifier the upload handshake expects.
///
/// The web UI uses `<uuid>_<n>_<random>`; only uniqueness appears to matter.
fn upload_uuid() -> String {
    let mut raw = [0u8; 24];
    rsa::rand_core::RngCore::fill_bytes(&mut rsa::rand_core::OsRng, &mut raw);
    let h = hex::encode(raw);
    format!(
        "{}-{}-{}-{}-{}_1_{}",
        &h[0..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..32],
        &h[32..]
    )
}
