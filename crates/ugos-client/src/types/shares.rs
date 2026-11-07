//! Types for shared folders and the SMB service.
//!
//! A share is a top-level folder on a volume that UGOS offers over the
//! network. Its settings live in the file manager (`filemgr/…`), its access
//! rules in a per-path ACL, and the SMB service that publishes it under
//! `file/smb/…`. Responses carry far more fields than are useful here, so
//! these model the ones a share needs and let the rest fall away.

use serde::{Deserialize, Serialize};

/// A shared folder, as `filemgr/getShareList` lists it.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Share {
    /// Share id, used by the settings endpoints.
    #[serde(default)]
    pub id: i64,
    /// Share name, the folder's base name.
    #[serde(default)]
    pub name: String,
    /// Absolute path on the NAS, such as `/volume1/media`.
    #[serde(default)]
    pub path: String,
    /// Filesystem of the volume, for instance `btrfs`.
    #[serde(default)]
    pub filesystem: String,
    /// Owner of the folder.
    #[serde(default)]
    pub owner: String,
    /// Whether the share is offered as a Time Machine target.
    #[serde(default)]
    pub time_backup: bool,
    /// Creation time as a unix timestamp.
    #[serde(default)]
    pub ctime: i64,
    /// Modification time as a unix timestamp.
    #[serde(default)]
    pub mtime: i64,
}

/// The settings of one share, from `filemgr/getShareFolderInfo`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ShareInfo {
    /// Share id.
    #[serde(default)]
    pub id: i64,
    /// Share name.
    #[serde(default)]
    pub name: String,
    /// Absolute path on the NAS.
    #[serde(default)]
    pub path: String,
    /// Volume the share lives on, such as `/volume1`.
    #[serde(default)]
    pub volume: String,
    /// Space limit in `unit`s; 0 means no limit.
    #[serde(default)]
    pub quota: i64,
    /// Unit of `quota`: 0 megabytes, 1 gigabytes, 2 terabytes.
    #[serde(default)]
    pub unit: i64,
    /// Whether data checksums are on (btrfs).
    #[serde(default)]
    pub data_check: bool,
    /// Whether the share is hidden from network browsing. It stays reachable
    /// by name.
    #[serde(default)]
    pub network_places_hide: bool,
    /// Whether the share is hidden from users who have no access to it.
    #[serde(default)]
    pub no_permission_hide: bool,
    /// Encryption state; 0 when the share is not encrypted.
    #[serde(default)]
    pub encrypt_status: i64,
    /// Owner of the folder.
    #[serde(default)]
    pub owner: String,
}

/// What a quota's `unit` code stands for.
pub const QUOTA_UNITS: [&str; 3] = ["MB", "GB", "TB"];

/// A new share, as `v2/filemgr/createShareFolder` takes it.
///
/// The defaults match the web UI's dialog: recycle bin on and limited to
/// administrators, hidden from users without access, visible in the network,
/// no checksums, no encryption, no space limit.
// One flag per switch of the API's flat shape; an enum would only restate them.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Serialize)]
pub struct NewShare {
    /// Volume to create the share on, such as `/volume1`.
    pub volume: String,
    /// Name of the new folder.
    pub name: String,
    /// Space limit in `unit`s; 0 means none.
    pub quota: i64,
    /// Unit of `quota`: 0 megabytes, 1 gigabytes, 2 terabytes.
    pub unit: i64,
    /// Data checksums.
    pub data_check: bool,
    /// Hide from network browsing.
    pub network_places_hide: bool,
    /// Hide from users without access.
    pub no_permission_hide: bool,
    /// Access rules; empty leaves the default of administrators only.
    pub permissions: Vec<AclChange>,
    /// Keep a recycle bin.
    pub enable_recycle: bool,
    /// Only administrators may empty the recycle bin.
    pub allow_only_admin_recycle: bool,
    /// Always off: encryption needs a key dialog this client does not have.
    pub open_encrypt: bool,
    /// Always empty, see `open_encrypt`.
    pub encrypt_session_id: String,
}

impl NewShare {
    /// A share with the web UI's defaults.
    #[must_use]
    pub fn new(volume: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            volume: volume.into(),
            name: name.into(),
            quota: 0,
            unit: 1,
            data_check: false,
            network_places_hide: false,
            no_permission_hide: true,
            permissions: Vec::new(),
            enable_recycle: true,
            allow_only_admin_recycle: true,
            open_encrypt: false,
            encrypt_session_id: String::new(),
        }
    }
}

/// What `v2/filemgr/createShareFolder` answers.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreatedShare {
    /// Id of the new share.
    #[serde(default)]
    pub id: i64,
    /// Absolute path of the new share.
    #[serde(default)]
    pub path: String,
    /// Name of the new share.
    #[serde(default)]
    pub name: String,
}

/// The access rules of a path, from `v2/filemgr/pathAcl`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PathAcl {
    /// One entry per group, with its members.
    #[serde(default)]
    pub acls: Vec<AclGroup>,
    /// Whether the folder is hidden from users without access.
    #[serde(default)]
    pub no_permission_hide: bool,
    /// Number of users across all groups.
    #[serde(default)]
    pub total_users: i64,
}

/// A group and its members in an ACL.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AclGroup {
    /// Group name.
    #[serde(default)]
    pub name: String,
    /// Display name.
    #[serde(default)]
    pub full_name: String,
    /// Numeric group id.
    #[serde(default)]
    pub gid: i64,
    /// Effective access, a permission mask. See [`Access`].
    #[serde(default)]
    pub permission_mask: i64,
    /// Access set on this path for the group; -1 when none is set.
    #[serde(default)]
    pub select_permissions: i64,
    /// Access inherited from the parent; -1 when there is none.
    #[serde(default)]
    pub parent_permissions: i64,
    /// Members of the group.
    #[serde(default)]
    pub users: Vec<AclUser>,
}

/// A user in an ACL.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AclUser {
    /// Login name.
    #[serde(default)]
    pub username: String,
    /// Display name.
    #[serde(default)]
    pub full_name: String,
    /// Numeric user id.
    #[serde(default)]
    pub uid: i64,
    /// Whether the user is an administrator.
    #[serde(default)]
    pub is_admin: bool,
    /// Effective access, a permission mask. See [`Access`].
    #[serde(default)]
    pub permission_mask: i64,
    /// Access set on this path for the user; -1 when none is set.
    #[serde(default)]
    pub select_permissions: i64,
    /// Access inherited from the parent; -1 when there is none.
    #[serde(default)]
    pub parent_permissions: i64,
}

/// Whether an ACL entry names a user or a group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PrincipalKind {
    /// A user account.
    User,
    /// A group.
    Group,
}

/// One change to an ACL, as `v1/filemgr/modifyPathAlc` takes it.
///
/// Only the entries that change are sent; everything else keeps its access.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AclChange {
    /// User or group.
    #[serde(rename = "type")]
    pub kind: PrincipalKind,
    /// Login name for a user, group name for a group.
    pub name: String,
    /// The access to set, as a permission mask. See [`Access::mask`].
    pub permission_mask: i64,
    /// Whether the user is an administrator. The web UI sends it along.
    pub is_admin: bool,
}

/// Access to a shared folder, as the permission mask the API uses.
///
/// The web UI offers three settings per user or group: no access, read, and
/// read and write. It writes the masks `0`, `4` and `7`, and reads `5` as read
/// and `6` as read and write, so this does too.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Access {
    /// Denied: the share is not reachable.
    None,
    /// Read only.
    Read,
    /// Read and write.
    ReadWrite,
    /// Nothing set on this path; access comes from the groups the user is in.
    Inherit,
    /// A mask this client does not know.
    Other(i64),
}

impl Access {
    /// The mask for a setting, as the web UI writes it.
    ///
    /// [`Access::Other`] keeps the value it carries.
    #[must_use]
    pub const fn mask(self) -> i64 {
        match self {
            Self::None => 0,
            Self::Read => 4,
            Self::ReadWrite => 7,
            Self::Inherit => -1,
            Self::Other(mask) => mask,
        }
    }

    /// The setting a mask stands for.
    #[must_use]
    pub const fn from_mask(mask: i64) -> Self {
        match mask {
            0 => Self::None,
            4 | 5 => Self::Read,
            6 | 7 => Self::ReadWrite,
            -1 => Self::Inherit,
            other => Self::Other(other),
        }
    }

    /// The word the CLI shows and accepts.
    #[must_use]
    pub fn label(self) -> String {
        match self {
            Self::None => "none".into(),
            Self::Read => "read".into(),
            Self::ReadWrite => "rw".into(),
            Self::Inherit => "inherit".into(),
            Self::Other(mask) => format!("mask {mask}"),
        }
    }

    /// Parse the word [`Access::label`] produces, without the `mask N` form.
    ///
    /// # Errors
    ///
    /// Returns the rejected text when it names no setting.
    pub fn parse(text: &str) -> Result<Self, String> {
        match text.trim().to_ascii_lowercase().as_str() {
            "none" | "deny" => Ok(Self::None),
            "read" | "ro" => Ok(Self::Read),
            "rw" | "write" | "readwrite" => Ok(Self::ReadWrite),
            "inherit" | "unset" => Ok(Self::Inherit),
            _ => Err(text.to_owned()),
        }
    }
}

/// The SMB service configuration, from `file/smb/config`.
///
/// The response holds some 40 switches. This models the ones worth showing;
/// the settings that tune protocol versions, signing and encryption are
/// numeric codes whose meaning UGOS does not document, so they are shown as
/// they come.
// One flag per switch of the API's flat shape; an enum would only restate them.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SmbConfig {
    /// Whether the SMB service is running.
    #[serde(default)]
    pub status: bool,
    /// Server name as other machines see it.
    #[serde(default)]
    pub nas_name: String,
    /// Workgroup.
    #[serde(default)]
    pub work_group: String,
    /// WINS server, empty for none.
    #[serde(default)]
    pub wins_server: String,
    /// Whether the NAS has joined a domain.
    #[serde(default)]
    pub is_joined_domain: bool,
    /// Whether users without access do not see a share in the list.
    #[serde(default)]
    pub hide_share_dir_for_no_permission: bool,
    /// Protocol selection codes.
    #[serde(default)]
    pub smb_protocols: Vec<i64>,
    /// Lowest protocol version, as a code.
    #[serde(default)]
    pub min_protocol: i64,
    /// Highest protocol version, as a code.
    #[serde(default)]
    pub max_protocol: i64,
    /// Transport encryption mode, as a code.
    #[serde(default)]
    pub encrypt_mode: i64,
    /// Signing mode, as a code.
    #[serde(default)]
    pub server_sign_mode: i64,
    /// Whether the weak `NTLMv1` authentication is allowed.
    #[serde(default)]
    pub enable_ntlmv1: bool,
}

/// Discovery services and Time Machine, from `file/advanced/config`.
// One flag per switch of the API's flat shape; an enum would only restate them.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SmbAdvanced {
    /// Bonjour announcements, which Macs use to find the NAS.
    #[serde(default, rename = "enableBonjourService")]
    pub bonjour: bool,
    /// WS-Discovery, which Windows uses to find the NAS.
    #[serde(default, rename = "enableWsdd2Service")]
    pub wsdd2: bool,
    /// `UPnP` announcements.
    #[serde(default, rename = "enableUPnPService")]
    pub upnp: bool,
    /// Whether Time Machine is offered at all.
    #[serde(default, rename = "enableTimeMachine")]
    pub time_machine: bool,
    /// The shares, with their Time Machine flag.
    #[serde(default, rename = "sharedFileInfos")]
    pub shares: Vec<AdvancedShare>,
}

/// A share in the advanced configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AdvancedShare {
    /// Share id.
    #[serde(default)]
    pub id: i64,
    /// Share name.
    #[serde(default)]
    pub name: String,
    /// Absolute path.
    #[serde(default)]
    pub path: String,
    /// Whether this share is a Time Machine target.
    #[serde(default)]
    pub time_backup: bool,
}
