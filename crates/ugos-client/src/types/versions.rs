//! Types for the File Version Explorer.

use serde::{Deserialize, Serialize};

use super::common::null_vec_as_empty;

/// A folder and its version settings.
// One flag per field of the API's flat shape; an enum would only restate them.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionFolder {
    /// Name of the share or user.
    #[serde(default)]
    pub name: String,
    /// The folder.
    #[serde(default)]
    pub path: String,
    /// Whether versions are kept.
    #[serde(default)]
    pub enable: bool,
    /// How many versions of a file are kept, 1 to 32.
    #[serde(default)]
    pub save_number: i64,
    /// Space the versions take, in bytes.
    #[serde(default)]
    pub size: i64,
    /// Whether the volume can keep versions.
    #[serde(default)]
    pub is_available_volume: bool,
    /// Whether the folder can keep versions.
    #[serde(default)]
    pub is_enable_folder: bool,
    /// Whether making versions failed.
    #[serde(default)]
    pub is_generation_failure: bool,
    /// Whether UGOS repairs the version store.
    #[serde(default)]
    pub is_repairing: bool,
    /// `ext4` or `btrfs`.
    #[serde(default)]
    pub fs_type: String,
}

/// `folderInfo`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VersionFolders {
    /// The folders.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub folders: Vec<VersionFolder>,
    /// How many there are.
    #[serde(default)]
    pub total: i64,
    /// Whether UGOS migrates old histories, which blocks changes.
    #[serde(default)]
    pub is_data_compatibility: bool,
}

/// A file as the version explorer lists it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionItem {
    /// Full path.
    #[serde(default)]
    pub path: String,
    /// Name.
    #[serde(default)]
    pub name: String,
    /// Size in bytes.
    #[serde(default)]
    pub size: i64,
    /// `0` file, `1` folder.
    #[serde(default)]
    pub file_type: i64,
    /// The inode, which names a deleted file together with `uuid`. UGOS sends
    /// a number or a text, and wants it back as it came.
    #[serde(default)]
    pub inode: serde_json::Value,
    /// The uuid.
    #[serde(default)]
    pub uuid: String,
    /// The version, whose first 13 characters are a millisecond timestamp.
    #[serde(default)]
    pub version_number: String,
    /// Whether it was deleted before the viewed time.
    #[serde(default)]
    pub is_deleted: bool,
    /// Modification time, Unix seconds.
    #[serde(default)]
    pub mtime: i64,
}

/// One historic version of a file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileVersion {
    /// Name.
    #[serde(default)]
    pub name: String,
    /// Path.
    #[serde(default)]
    pub path: String,
    /// `0` file, `1` folder.
    #[serde(default)]
    pub file_type: i64,
    /// The version number.
    #[serde(default)]
    pub version_number: String,
    /// When the version was made, Unix seconds.
    #[serde(default)]
    pub version_timestamp: i64,
    /// When the file was last changed, Unix seconds.
    #[serde(default)]
    pub last_modify_time: i64,
    /// Size in bytes.
    #[serde(default)]
    pub size: i64,
}

/// What happened on a day, for the calendar.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DayMarks {
    /// Something was made.
    #[serde(default)]
    pub create: bool,
    /// Something was changed.
    #[serde(default)]
    pub modify: bool,
    /// Something was deleted.
    #[serde(default)]
    pub delete: bool,
}

/// `getMonthVersion`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MonthMarks {
    /// One entry per day, from the 1st.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub day: Vec<DayMarks>,
}
