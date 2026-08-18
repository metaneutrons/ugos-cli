//! Types for the filesystem snapshot app (`com.ugreen.snapshot`).
//!
//! These are btrfs snapshots of home directories and shared folders, and
//! have nothing to do with the KVM snapshots in [`crate::types::kvm`].
//!
//! This app uses `snake_case` and a REST-shaped API, unlike the rest of UGOS.

use serde::{Deserialize, Serialize};

/// Which class of folder a snapshot belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FolderType {
    /// A user's home directory.
    Home,
    /// A shared folder.
    Share,
}

impl FolderType {
    /// The value UGOS expects in `folder_type`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Home => "home",
            Self::Share => "share",
        }
    }
}

impl std::fmt::Display for FolderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for FolderType {
    type Err = crate::error::UgosError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "home" => Ok(Self::Home),
            "share" => Ok(Self::Share),
            other => Err(crate::error::UgosError::ParameterError(format!(
                "unknown folder type '{other}', expected 'home' or 'share'"
            ))),
        }
    }
}

/// A folder that can hold snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotFolder {
    /// Numeric id used in every other call.
    pub id: i64,
    /// Display name.
    #[serde(default)]
    pub folder_name: String,
    /// How many snapshots the folder currently holds.
    #[serde(default)]
    pub snapshot_number: i64,
    /// Schedule state; 2 means no schedule is active.
    #[serde(default)]
    pub snapshot_schedule_status: i64,
    /// Unix timestamp of the newest snapshot, `-1` when there is none.
    #[serde(default)]
    pub latest_snapshot_timestamp: i64,
    /// Whether snapshots may be created and removed here.
    #[serde(default)]
    pub allow_operations: bool,
}

/// Response of the folder listing.
#[derive(Debug, Deserialize)]
pub struct FolderList {
    /// The folders.
    #[serde(default)]
    pub folders: Vec<SnapshotFolder>,
    /// Total across all pages.
    #[serde(default)]
    pub total_number: i64,
}

/// A single filesystem snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    /// Numeric id, unique within its folder.
    pub id: i64,
    /// Generated name, e.g. `GMT+01_2026-03-01_084041`.
    #[serde(default)]
    pub name: String,
    /// Creation time as a Unix timestamp.
    #[serde(default)]
    pub create_timestamp: i64,
    /// Free-text description given at creation.
    #[serde(default)]
    pub desc: String,
    /// Lock flag. Honoured by the web UI, not enforced by the API: a
    /// locked snapshot can still be deleted through it.
    #[serde(default)]
    pub is_locked: bool,
    /// Where the snapshot is mounted on the NAS.
    #[serde(default)]
    pub abs_path: String,
    /// Whether the contents can currently be read.
    #[serde(default)]
    pub accessible: bool,
    /// Why it cannot be read, when `accessible` is false.
    #[serde(default)]
    pub inaccessible_reason: String,
    /// Id of the folder it belongs to.
    #[serde(default)]
    pub folder_id: i64,
    /// Whether that folder is a home directory or a share.
    #[serde(default)]
    pub folder_type: String,
    /// Name of that folder.
    #[serde(default)]
    pub folder_name: String,
}

/// Response of the snapshot listing.
#[derive(Debug, Deserialize)]
pub struct SnapshotList {
    /// The snapshots on this page.
    #[serde(default)]
    pub list: Vec<Snapshot>,
    /// Total across all pages.
    #[serde(default)]
    pub total: i64,
    /// Id of the folder listed.
    #[serde(default)]
    pub folder_id: i64,
    /// Name of the folder listed.
    #[serde(default)]
    pub folder_name: String,
    /// Schedule state of that folder.
    #[serde(default)]
    pub snapshot_schedule_status: i64,
}

/// Body for creating a snapshot.
#[derive(Debug, Serialize)]
pub struct CreateSnapshot<'a> {
    /// Folder to snapshot.
    pub folder_id: i64,
    /// Whether that folder is a home directory or a share.
    pub folder_type: &'a str,
    /// Free-text description. UGOS names the snapshot itself.
    pub desc: &'a str,
    /// Set the lock flag. The web UI then refuses to delete it; the API
    /// does not.
    pub is_locked: bool,
}

/// Body for editing a snapshot's description or lock.
#[derive(Debug, Serialize)]
pub struct EditSnapshot<'a> {
    /// Folder the snapshot lives in.
    pub folder_id: i64,
    /// Whether that folder is a home directory or a share.
    pub folder_type: &'a str,
    /// The snapshot to edit.
    pub snapshot_id: i64,
    /// New description.
    pub desc: &'a str,
    /// New lock state.
    pub is_locked: bool,
}

/// Body for deleting snapshots.
#[derive(Debug, Serialize)]
pub struct DeleteSnapshots<'a> {
    /// Folder the snapshots live in.
    pub folder_id: i64,
    /// Whether that folder is a home directory or a share.
    pub folder_type: &'a str,
    /// Ids to remove. UGOS deletes in bulk.
    pub snapshot_ids: &'a [i64],
}

/// Body for cloning a snapshot into a new folder.
#[derive(Debug, Serialize)]
pub struct CloneSnapshot<'a> {
    /// Folder the snapshot lives in.
    pub folder_id: i64,
    /// Whether that folder is a home directory or a share.
    pub folder_type: &'a str,
    /// The snapshot to clone.
    pub snapshot_id: i64,
    /// Name for the clone.
    pub new_name: &'a str,
}
