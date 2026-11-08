//! Types for groups, share permissions and per-user quotas.

use serde::{Deserialize, Serialize};

use super::common::null_vec_as_empty;
use super::users::GroupMember;

/// One share and what an account or a group may do there.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionRow {
    /// Share id.
    #[serde(default)]
    pub id: i64,
    /// Share name.
    #[serde(default)]
    pub name: String,
    /// Share path.
    #[serde(default)]
    pub path: String,
    /// What was set: `-1` nothing, `0` none, `4`/`5` read, `6`/`7` write.
    #[serde(default)]
    pub permission: i64,
    /// What counts after the groups are taken into account.
    #[serde(default)]
    pub permission_mask: i64,
}

/// A list wrapped in `{result: [...]}`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct PermissionList {
    /// The rows.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub result: Vec<PermissionRow>,
}

/// The limit on one volume.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuotaRow {
    /// The volume, or the folder the limit is on.
    #[serde(default)]
    pub path: String,
    /// The limit in KiB, `0` for none.
    #[serde(default)]
    pub hard: i64,
    /// The largest limit that fits, in bytes.
    #[serde(default)]
    pub max_value: i64,
    /// What is used, as UGOS counts it.
    #[serde(default)]
    pub used: i64,
    /// A label, for `ext4`.
    #[serde(default)]
    pub label: String,
}

/// `user/quota/{uid}`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Quota {
    /// Limits on btrfs volumes.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub btrfs: Vec<QuotaRow>,
    /// Limits on ext4 volumes.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub ext4: Vec<QuotaRow>,
}

/// A group with its members.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupInfo {
    /// Name.
    #[serde(default)]
    pub groupname: String,
    /// Description.
    #[serde(default)]
    pub description: String,
    /// The members.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub users: Vec<GroupMember>,
}
