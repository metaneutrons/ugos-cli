//! Shared folders and the SMB service.
//!
//! The share settings and access rules live on the file manager API. Creating
//! and deleting a share and reading its rules are v2 and need the encrypted
//! transport; listing, the settings and changing the rules are v1 and plain.
//! The SMB service answers under `file/`.
//!
//! Only reading the SMB service is wired. Starting, stopping and
//! reconfiguring it interrupts every client that is connected, which cannot
//! be tried safely on a NAS in use. `docs/api-shares.md` lists the endpoints.

use crate::client::UgosClient;
use crate::error::{Result, UgosError};
use crate::types::common::ResultWrapper;
use crate::types::shares::{
    AclChange, CreatedShare, NewShare, PathAcl, Share, ShareInfo, SmbAdvanced, SmbConfig,
};

/// Shared folder and SMB operations.
#[allow(clippy::module_name_repetitions)]
pub trait SharesApi {
    /// List the shared folders.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on failure.
    fn share_list(&self) -> impl Future<Output = Result<Vec<Share>>> + Send;

    /// Find a share by its name, or by its numeric id.
    ///
    /// A name wins over an id, since a share may be called `7`.
    ///
    /// # Errors
    ///
    /// Returns [`UgosError::NotFound`] when no share matches.
    fn share_find(&self, name_or_id: &str) -> impl Future<Output = Result<Share>> + Send;

    /// The settings of one share.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on failure.
    fn share_info(&self, id: i64) -> impl Future<Output = Result<ShareInfo>> + Send;

    /// The access rules of a path.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on failure.
    fn share_acl(&self, path: &str) -> impl Future<Output = Result<PathAcl>> + Send;

    /// Create a share.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on failure.
    fn share_create(&self, new: &NewShare) -> impl Future<Output = Result<CreatedShare>> + Send;

    /// Write the settings of a share back.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on failure.
    fn share_update(&self, info: &ShareInfo) -> impl Future<Output = Result<()>> + Send;

    /// Change the access rules of a path. Entries not named keep their access.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on failure.
    fn share_acl_set(
        &self,
        path: &str,
        changes: &[AclChange],
    ) -> impl Future<Output = Result<()>> + Send;

    /// Delete a share, named by its path.
    ///
    /// UGOS wants the account's password for this, and the client sends the
    /// one it logged in with.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on failure.
    fn share_delete(&self, path: &str) -> impl Future<Output = Result<()>> + Send;

    /// The SMB service configuration.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on failure.
    fn smb_config(&self) -> impl Future<Output = Result<SmbConfig>> + Send;

    /// Discovery services and Time Machine.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on failure.
    fn smb_advanced(&self) -> impl Future<Output = Result<SmbAdvanced>> + Send;
}

/// The body of `filemgr/modifyPathAlc`.
#[must_use]
pub fn acl_body(path: &str, changes: &[AclChange]) -> serde_json::Value {
    serde_json::json!({ "path": path, "permissions": changes })
}

/// The body of `filemgr/updateShareFolderInfo`, in the shape the web UI sends.
///
/// The settings go in `set_share_folder_info`; `no_permission_hide` travels
/// beside it and is not repeated inside. `permissions` stays empty, access
/// rules have their own call. A request that merely carries the settings at
/// the top level is answered with success and changes nothing.
#[must_use]
pub fn update_body(info: &ShareInfo) -> serde_json::Value {
    serde_json::json!({
        "id": info.id,
        "set_share_folder_info": {
            "data_check": info.data_check,
            "name": info.name,
            "network_places_hide": info.network_places_hide,
            "quota": info.quota,
            "volume": info.volume,
            "unit": info.unit,
        },
        "permissions": [],
        "no_permission_hide": info.no_permission_hide,
    })
}

/// Pick a share by name, then by numeric id.
///
/// # Errors
///
/// Returns [`UgosError::NotFound`] when neither matches.
pub fn pick_share(shares: &[Share], name_or_id: &str) -> Result<Share> {
    if let Some(found) = shares.iter().find(|share| share.name == name_or_id) {
        return Ok(found.clone());
    }
    if let Ok(id) = name_or_id.parse::<i64>()
        && let Some(found) = shares.iter().find(|share| share.id == id)
    {
        return Ok(found.clone());
    }
    Err(UgosError::NotFound {
        kind: "share",
        name: name_or_id.to_owned(),
    })
}

impl SharesApi for UgosClient {
    async fn share_list(&self) -> Result<Vec<Share>> {
        let resp: ResultWrapper<Vec<Share>> = self.get("filemgr/getShareList").await?;
        Ok(resp.result)
    }

    async fn share_find(&self, name_or_id: &str) -> Result<Share> {
        pick_share(&self.share_list().await?, name_or_id)
    }

    async fn share_info(&self, id: i64) -> Result<ShareInfo> {
        self.get_with_params("filemgr/getShareFolderInfo", &[("id", &id.to_string())])
            .await
    }

    async fn share_acl(&self, path: &str) -> Result<PathAcl> {
        self.get_encrypted("v2/filemgr/pathAcl", &[("path", path)])
            .await
    }

    async fn share_create(&self, new: &NewShare) -> Result<CreatedShare> {
        self.post_encrypted("v2/filemgr/createShareFolder", new)
            .await
    }

    async fn share_update(&self, info: &ShareInfo) -> Result<()> {
        let _: serde_json::Value = self
            .post("filemgr/updateShareFolderInfo", &update_body(info))
            .await?;
        Ok(())
    }

    async fn share_acl_set(&self, path: &str, changes: &[AclChange]) -> Result<()> {
        let _: serde_json::Value = self
            .post("filemgr/modifyPathAlc", &acl_body(path, changes))
            .await?;
        Ok(())
    }

    async fn share_delete(&self, path: &str) -> Result<()> {
        let _: serde_json::Value = self
            .delete_encrypted_confirmed("v2/filemgr/delSharedFolder", &[("path", path)])
            .await?;
        Ok(())
    }

    async fn smb_config(&self) -> Result<SmbConfig> {
        self.get("file/smb/config").await
    }

    async fn smb_advanced(&self) -> Result<SmbAdvanced> {
        self.get("file/advanced/config").await
    }
}
