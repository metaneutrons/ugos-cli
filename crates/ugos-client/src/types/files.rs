//! Types for the file manager.
//!
//! The listing endpoint lives on the v2 API and answers only to encrypted
//! requests. Entries carry far more fields than are useful here — tags, mount
//! state, recycle-bin bookkeeping — so this models the ones a file listing
//! actually needs and lets the rest fall away.

use serde::{Deserialize, Serialize};

/// A directory listing.
///
/// UGOS answers with two panes, mirroring its web UI: `left_tree` for the
/// navigation sidebar and `right_files` for the directory itself.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DirListing {
    /// The directory's own contents.
    #[serde(default)]
    pub right_files: FilePane,
    /// Status code; 0 on success.
    #[serde(default)]
    pub status: i64,
}

/// One pane of a listing.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FilePane {
    /// The entries. `null` rather than an empty list for an empty directory.
    #[serde(default)]
    pub files: Option<Vec<FileEntry>>,
    /// Permission bits for the directory itself.
    #[serde(default)]
    pub permission_mask: i64,
    /// Status code for this pane.
    #[serde(default)]
    pub status: i64,
}

/// A file or directory.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileEntry {
    /// Base name.
    #[serde(default)]
    pub name: String,
    /// Absolute path on the NAS.
    #[serde(default)]
    pub path: String,
    /// Size in bytes; 0 for directories.
    #[serde(default)]
    pub size: i64,
    /// 0 for a file, 1 for a directory.
    #[serde(default)]
    pub file_type: i64,
    /// Extension without the dot.
    #[serde(default)]
    pub ext: String,
    /// Modification time as a unix timestamp.
    #[serde(default)]
    pub mtime: i64,
    /// Creation time as a unix timestamp.
    #[serde(default)]
    pub ctime: i64,
    /// Owner, often empty.
    #[serde(default)]
    pub owner: String,
    /// Permission bits.
    #[serde(default)]
    pub permission_mask: i64,
}

impl FileEntry {
    /// Whether this entry is a directory.
    #[must_use]
    pub const fn is_dir(&self) -> bool {
        self.file_type == 1
    }
}

/// A storage volume as the file manager reports it.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Volume {
    /// Display name, e.g. `Volume 1`.
    #[serde(default)]
    pub name: String,
    /// Mount path, e.g. `/volume1`.
    #[serde(default)]
    pub path: String,
    /// Filesystem type.
    #[serde(default)]
    pub fs_type: String,
    /// Total capacity in bytes.
    #[serde(default)]
    pub all: i64,
    /// Used capacity in bytes.
    #[serde(default)]
    pub used: i64,
    /// Free capacity in bytes.
    #[serde(default)]
    pub free: i64,
}
