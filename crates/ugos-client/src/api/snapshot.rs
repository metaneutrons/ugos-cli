//! Filesystem snapshots (`com.ugreen.snapshot`).
//!
//! Unrelated to the KVM snapshots in [`crate::api::kvm`]: these are btrfs
//! snapshots of home directories and shared folders.
//!
//! The app is the only REST-shaped corner of UGOS. The same path
//! `snapshot/snapshot` lists on GET, creates on POST, edits on PUT and
//! deletes on DELETE, with the payload in the body even for DELETE.

use std::future::Future;

use crate::client::UgosClient;
use crate::error::Result;
use crate::types::snapshot::{
    CloneSnapshot, CreateSnapshot, DeleteSnapshots, EditSnapshot, FolderList, FolderType,
    SnapshotList,
};

/// Page size used when listing. UGOS requires both paging parameters and
/// rejects the call outright when either is zero.
const PAGE_SIZE: i64 = 500;

/// Filesystem snapshot operations.
pub trait SnapshotApi {
    /// List folders of one class that can hold snapshots.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`crate::error::UgosError`] on failure.
    fn fs_snapshot_folders(
        &self,
        kind: FolderType,
    ) -> impl Future<Output = Result<FolderList>> + Send;

    /// Resolve a folder name to its id, searching homes and shares.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::UgosError::NotFound`] if no folder matches.
    fn fs_snapshot_folder_by_name(
        &self,
        name: &str,
    ) -> impl Future<Output = Result<(i64, FolderType)>> + Send;

    /// List the snapshots held by one folder.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`crate::error::UgosError`] on failure.
    fn fs_snapshot_list(
        &self,
        folder_id: i64,
        kind: FolderType,
    ) -> impl Future<Output = Result<SnapshotList>> + Send;

    /// Take a snapshot of a folder. UGOS names it after the current time.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`crate::error::UgosError`] on failure.
    fn fs_snapshot_create(
        &self,
        folder_id: i64,
        kind: FolderType,
        desc: &str,
        locked: bool,
    ) -> impl Future<Output = Result<()>> + Send;

    /// Change a snapshot's description or lock state.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`crate::error::UgosError`] on failure.
    fn fs_snapshot_edit(
        &self,
        folder_id: i64,
        kind: FolderType,
        snapshot_id: i64,
        desc: &str,
        locked: bool,
    ) -> impl Future<Output = Result<()>> + Send;

    /// Delete snapshots.
    ///
    /// A locked snapshot is deleted just the same. Verified against a live
    /// NAS on 2026-08-19: `is_locked` is a hint the web UI honours, not a
    /// rule the API enforces, so nothing here can rely on it.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`crate::error::UgosError`] on failure.
    fn fs_snapshot_delete(
        &self,
        folder_id: i64,
        kind: FolderType,
        ids: &[i64],
    ) -> impl Future<Output = Result<()>> + Send;

    /// Clone a snapshot into a new folder of its own.
    ///
    /// This is the non-destructive way to get at a snapshot's contents: it
    /// materialises them beside the original instead of rolling it back.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`crate::error::UgosError`] on failure.
    fn fs_snapshot_clone(
        &self,
        folder_id: i64,
        kind: FolderType,
        snapshot_id: i64,
        new_name: &str,
    ) -> impl Future<Output = Result<()>> + Send;
}

impl SnapshotApi for UgosClient {
    async fn fs_snapshot_folders(&self, kind: FolderType) -> Result<FolderList> {
        let page_size = PAGE_SIZE.to_string();
        self.get_with_params(
            &format!("snapshot/folder/{kind}"),
            &[("page_number", "1"), ("page_size", &page_size)],
        )
        .await
    }

    async fn fs_snapshot_folder_by_name(&self, name: &str) -> Result<(i64, FolderType)> {
        for kind in [FolderType::Share, FolderType::Home] {
            let folders = self.fs_snapshot_folders(kind).await?;
            if let Some(found) = folders.folders.iter().find(|f| f.folder_name == name) {
                return Ok((found.id, kind));
            }
        }
        Err(crate::error::UgosError::NotFound {
            kind: "snapshot folder",
            name: name.to_owned(),
        })
    }

    async fn fs_snapshot_list(&self, folder_id: i64, kind: FolderType) -> Result<SnapshotList> {
        let id = folder_id.to_string();
        let page_size = PAGE_SIZE.to_string();
        self.get_with_params(
            "snapshot/snapshot",
            &[
                ("folder_id", &id),
                ("folder_type", kind.as_str()),
                ("page_number", "1"),
                ("page_size", &page_size),
            ],
        )
        .await
    }

    async fn fs_snapshot_create(
        &self,
        folder_id: i64,
        kind: FolderType,
        desc: &str,
        locked: bool,
    ) -> Result<()> {
        let body = CreateSnapshot {
            folder_id,
            folder_type: kind.as_str(),
            desc,
            is_locked: locked,
        };
        let _: serde_json::Value = self.post("snapshot/snapshot", &body).await?;
        Ok(())
    }

    async fn fs_snapshot_edit(
        &self,
        folder_id: i64,
        kind: FolderType,
        snapshot_id: i64,
        desc: &str,
        locked: bool,
    ) -> Result<()> {
        let body = EditSnapshot {
            folder_id,
            folder_type: kind.as_str(),
            snapshot_id,
            desc,
            is_locked: locked,
        };
        let _: serde_json::Value = self.put("snapshot/snapshot", &body).await?;
        Ok(())
    }

    async fn fs_snapshot_delete(
        &self,
        folder_id: i64,
        kind: FolderType,
        ids: &[i64],
    ) -> Result<()> {
        let body = DeleteSnapshots {
            folder_id,
            folder_type: kind.as_str(),
            snapshot_ids: ids,
        };
        let _: serde_json::Value = self.delete_with_body("snapshot/snapshot", &body).await?;
        Ok(())
    }

    async fn fs_snapshot_clone(
        &self,
        folder_id: i64,
        kind: FolderType,
        snapshot_id: i64,
        new_name: &str,
    ) -> Result<()> {
        let body = CloneSnapshot {
            folder_id,
            folder_type: kind.as_str(),
            snapshot_id,
            new_name,
        };
        let _: serde_json::Value = self.post("snapshot/snapshot/clone", &body).await?;
        Ok(())
    }
}

/// Re-exported so callers do not need the types module for the common case.
pub use crate::types::snapshot::{Snapshot as FsSnapshot, SnapshotFolder};
