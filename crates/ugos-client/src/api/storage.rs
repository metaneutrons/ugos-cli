//! Storage Manager: pools, volumes and disks, read only.
//!
//! Creating and deleting pools and volumes, replacing disks and erasing them
//! is in the web UI and not wired here. `docs/api-storage.md` lists the
//! endpoints. The disk list is a v2 call and travels encrypted.

use serde::Deserialize;

use crate::client::UgosClient;
use crate::error::{Result, UgosError};
use crate::types::common::ResultWrapper;
use crate::types::storage::{
    Disk, DiskAttribute, Pool, Smart, StorageVolume, TestPlan, TestRecord,
};

/// Storage Manager reads.
#[allow(clippy::module_name_repetitions)]
pub trait StorageApi {
    /// The storage pools.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on failure.
    fn pools(&self) -> impl Future<Output = Result<Vec<Pool>>> + Send;

    /// The volumes.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on failure.
    fn volumes(&self) -> impl Future<Output = Result<Vec<StorageVolume>>> + Send;

    /// The disks.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on failure.
    fn disks(&self) -> impl Future<Output = Result<Vec<Disk>>> + Send;

    /// Find a disk by short name, device path, slot or label.
    ///
    /// # Errors
    ///
    /// Returns [`UgosError::NotFound`] when no disk matches.
    fn disk_find(&self, which: &str) -> impl Future<Output = Result<Disk>> + Send;

    /// Fixed facts about a disk, named by its device path.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on failure.
    fn disk_attribute(&self, dev: &str) -> impl Future<Output = Result<DiskAttribute>> + Send;

    /// A disk's SMART state and report, named by its device path.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on failure.
    fn disk_smart(&self, dev: &str) -> impl Future<Output = Result<Smart>> + Send;

    /// The history of a disk's tests, named by its device path.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on failure.
    fn disk_history(&self, dev: &str) -> impl Future<Output = Result<Vec<TestRecord>>> + Send;

    /// The scheduled disk test plans.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on failure.
    fn disk_plans(&self) -> impl Future<Output = Result<Vec<TestPlan>>> + Send;
}

/// Pick a disk by short name, device path, slot or label.
///
/// # Errors
///
/// Returns [`UgosError::NotFound`] when none matches.
pub fn pick_disk(disks: &[Disk], which: &str) -> Result<Disk> {
    let wanted = which.trim();
    let bare = wanted.strip_prefix("/dev/").unwrap_or(wanted);
    disks
        .iter()
        .find(|disk| {
            disk.name == bare
                || disk.dev_name == wanted
                || disk.slot.eq_ignore_ascii_case(wanted)
                || disk.label.eq_ignore_ascii_case(wanted)
        })
        .cloned()
        .ok_or_else(|| UgosError::NotFound {
            kind: "disk",
            name: which.to_owned(),
        })
}

/// Pick a pool by name or label.
///
/// # Errors
///
/// Returns [`UgosError::NotFound`] when none matches.
pub fn pick_pool(pools: &[Pool], which: &str) -> Result<Pool> {
    pools
        .iter()
        .find(|pool| pool.name == which || pool.label.eq_ignore_ascii_case(which))
        .cloned()
        .ok_or_else(|| UgosError::NotFound {
            kind: "pool",
            name: which.to_owned(),
        })
}

/// Pick a volume by name, label or mount path.
///
/// # Errors
///
/// Returns [`UgosError::NotFound`] when none matches.
pub fn pick_volume(volumes: &[StorageVolume], which: &str) -> Result<StorageVolume> {
    let path = which.trim_end_matches('/');
    volumes
        .iter()
        .find(|volume| {
            volume.name == which
                || volume.label.eq_ignore_ascii_case(which)
                || volume.mntpaths.iter().any(|mount| mount == path)
        })
        .cloned()
        .ok_or_else(|| UgosError::NotFound {
            kind: "volume",
            name: which.to_owned(),
        })
}

impl StorageApi for UgosClient {
    async fn pools(&self) -> Result<Vec<Pool>> {
        let reply: ResultWrapper<Vec<Pool>> = self.get("storage/pool/list").await?;
        Ok(reply.result)
    }

    async fn volumes(&self) -> Result<Vec<StorageVolume>> {
        let reply: ResultWrapper<Vec<StorageVolume>> = self.get("storage/volume/list").await?;
        Ok(reply.result)
    }

    async fn disks(&self) -> Result<Vec<Disk>> {
        #[derive(Deserialize)]
        struct List {
            #[serde(default, deserialize_with = "crate::types::common::null_vec_as_empty")]
            result: Vec<Disk>,
        }
        let reply: List = self.get_encrypted("v2/storage/disk/list", &[]).await?;
        Ok(reply.result)
    }

    async fn disk_find(&self, which: &str) -> Result<Disk> {
        pick_disk(&self.disks().await?, which)
    }

    async fn disk_attribute(&self, dev: &str) -> Result<DiskAttribute> {
        self.get_with_params("storage/disk/attribute", &[("disk", dev)])
            .await
    }

    async fn disk_smart(&self, dev: &str) -> Result<Smart> {
        self.get_with_params("storage/disk/smart/info", &[("disk", dev)])
            .await
    }

    async fn disk_history(&self, dev: &str) -> Result<Vec<TestRecord>> {
        let reply: ResultWrapper<Vec<TestRecord>> = self
            .get_with_params("storage/disk/detection/list", &[("disk", dev)])
            .await?;
        Ok(reply.result)
    }

    async fn disk_plans(&self) -> Result<Vec<TestPlan>> {
        let reply: ResultWrapper<Vec<TestPlan>> = self.get("storage/disk/cron/list").await?;
        Ok(reply.result)
    }
}
