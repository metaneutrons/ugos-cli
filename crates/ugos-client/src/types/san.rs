//! Types for the SAN Manager: iSCSI targets, LUNs, initiator groups and the
//! global setting.

use serde::{Deserialize, Serialize};

use super::common::null_vec_as_empty;

/// A page of a list. UGOS answers `null` for an empty one.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(bound(deserialize = "T: Deserialize<'de>"))]
pub struct Page<T> {
    /// The items.
    #[serde(default = "Vec::new", deserialize_with = "null_vec_as_empty")]
    pub results: Vec<T>,
    /// How many there are in all.
    #[serde(default)]
    pub total: i64,
}

/// A LUN mapped to a target.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetLun {
    /// LUN id.
    #[serde(default)]
    pub lun_id: i64,
    /// LUN name.
    #[serde(default)]
    pub lun_name: String,
}

/// CHAP settings as UGOS shows them: never the passwords.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChapInfo {
    /// `0` none, `1` one-way, `2` mutual.
    #[serde(default, rename = "type")]
    pub kind: i64,
    /// The user of the initiator.
    #[serde(default)]
    pub userid: String,
    /// The user of the target, for mutual CHAP.
    #[serde(default)]
    pub mutual_userid: String,
}

/// An iSCSI target.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Target {
    /// Id.
    #[serde(default)]
    pub id: i64,
    /// Name.
    #[serde(default)]
    pub name: String,
    /// IQN.
    #[serde(default)]
    pub iqn: String,
    /// The LUNs mapped to it.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub luns: Vec<TargetLun>,
    /// `close`, `default` or `custom`.
    #[serde(default)]
    pub chap_type: String,
    /// CHAP users.
    #[serde(default)]
    pub chap_info: ChapInfo,
    /// Header digest.
    #[serde(default)]
    pub enable_head_crc: bool,
    /// Data digest.
    #[serde(default)]
    pub enable_data_crc: bool,
    /// Several connections per initiator.
    #[serde(default)]
    pub multi_conn: bool,
    /// `0` or `1` normal, `2` deactivated, `3` to `7` busy.
    #[serde(default)]
    pub status: i64,
    /// `0` or `1`.
    #[serde(default)]
    pub ready_state: i64,
}

/// A group's access to a LUN.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupAccess {
    /// Group id.
    #[serde(default)]
    pub group_id: i64,
    /// Group name.
    #[serde(default)]
    pub group_name: String,
    /// `rw` or `ro`.
    #[serde(default)]
    pub access_mode: String,
    /// Description.
    #[serde(default)]
    pub desc: String,
}

/// The target a LUN is mapped to.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LunTarget {
    /// Target id.
    #[serde(default)]
    pub target_id: i64,
    /// Target name.
    #[serde(default)]
    pub target_name: String,
}

/// A LUN.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lun {
    /// Id.
    #[serde(default)]
    pub id: i64,
    /// Name.
    #[serde(default)]
    pub name: String,
    /// Description.
    #[serde(default)]
    pub desc: String,
    /// `thin` or `thick`.
    #[serde(default, rename = "type")]
    pub kind: String,
    /// The volume it lives on.
    #[serde(default)]
    pub volume_name: String,
    /// The volume's label.
    #[serde(default)]
    pub volume_label: String,
    /// Size in bytes.
    #[serde(default)]
    pub size: i64,
    /// `GB` or `TB`, as it was made.
    #[serde(default)]
    pub size_type: String,
    /// Used bytes, negative when unknown.
    #[serde(default)]
    pub used_size: i64,
    /// `0` normal, `1` damaged, `2` read only, `3` not enough space,
    /// `4` protected, `5` creating, `6` editing, `7` removing, `8` expanding.
    #[serde(default)]
    pub status: i64,
    /// Force unit access.
    #[serde(default)]
    pub enable_fua: bool,
    /// `default` for all groups, `custom` for the listed ones.
    #[serde(default)]
    pub access_type: String,
    /// The groups with access, for `custom`.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub group_access: Vec<GroupAccess>,
    /// The targets it is mapped to.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub targets: Vec<LunTarget>,
}

/// An initiator group.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Group {
    /// Id.
    #[serde(default)]
    pub id: i64,
    /// Name.
    #[serde(default)]
    pub name: String,
    /// Description.
    #[serde(default)]
    pub desc: String,
    /// The initiator IQNs.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub initiator_iqns: Vec<String>,
    /// `1` creating, `2` editing, `3` deleting.
    #[serde(default)]
    pub status: i64,
}

/// The global setting.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Setting {
    /// The start of every new IQN.
    #[serde(default)]
    pub iqn_prefix: String,
    /// The port of the iSCSI service.
    #[serde(default)]
    pub port: i64,
    /// The default CHAP: `0` off, `1` one-way, `2` mutual.
    #[serde(default, rename = "type")]
    pub kind: i64,
    /// The default CHAP user.
    #[serde(default)]
    pub userid: String,
    /// The default CHAP user of the target.
    #[serde(default)]
    pub mutual_userid: String,
}

/// An initiator connected to a target.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Connection {
    /// The target's name.
    #[serde(default)]
    pub label: String,
    /// The initiator's address.
    #[serde(default)]
    pub ip: String,
    /// The initiator's IQN.
    #[serde(default)]
    pub iqn: String,
    /// `connected` while it is.
    #[serde(default)]
    pub status: String,
}

/// A list wrapped in `{results: [...]}`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Connections {
    /// The connections.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub results: Vec<Connection>,
}
