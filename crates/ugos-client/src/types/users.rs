//! Types for creating, changing and deleting user accounts.

use serde::{Deserialize, Serialize};

/// `role` of an administrator.
pub const ROLE_ADMIN: &str = "admin";

/// `role` of a standard user.
pub const ROLE_USER: &str = "users";

/// `expire_time` of an account that never expires.
pub const EXPIRES_NEVER: i64 = -1;

/// `expire_time` of a disabled account.
pub const DISABLED: i64 = 1;

/// A group named in a request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GroupName {
    /// Group name.
    pub name: String,
}

/// An account to create. The password is sent sealed, not as it is here.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct NewUser {
    /// Login name.
    pub username: String,
    /// The password, in the clear. The client seals it before it is sent.
    pub password: String,
    /// Email address, may be empty.
    pub email: String,
    /// Free-text description, may be empty.
    pub description: String,
    /// [`ROLE_ADMIN`] or [`ROLE_USER`].
    pub role: String,
    /// Groups to join.
    pub groups: Vec<String>,
    /// Give the account a home folder.
    pub enable_home_dir: bool,
    /// Forbid the account to change its own password.
    pub deny_change_pwd: bool,
    /// Home folder quota in `unit`s, 0 for none.
    pub dir_quota: i64,
    /// Unit of `dir_quota`: 0 MB, 1 GB, 2 TB.
    pub unit: i64,
}

impl NewUser {
    /// A standard user with the web UI's defaults: a home folder, no quota,
    /// no groups.
    #[must_use]
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
            email: String::new(),
            description: String::new(),
            role: ROLE_USER.to_owned(),
            groups: Vec::new(),
            enable_home_dir: true,
            deny_change_pwd: false,
            dir_quota: 0,
            unit: 1,
        }
    }
}

/// An account as it should be after an edit. `uid` names the account, and
/// everything else is written back as given.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct UserEdit {
    /// Numeric user id.
    pub uid: i64,
    /// Login name. UGOS does not rename accounts, but the request carries it.
    pub username: String,
    /// The email address afterwards. UGOS blanks it when it is left out of
    /// the request, so an edit has to say it, whether or not it changes.
    pub email: String,
    /// Free-text description.
    pub description: String,
    /// [`ROLE_ADMIN`] or [`ROLE_USER`].
    pub role: String,
    /// Groups the account belongs to afterwards.
    pub groups: Vec<String>,
    /// Whether the account has a home folder.
    pub enable_home_dir: bool,
    /// Whether the account may not change its own password.
    pub deny_change_pwd: bool,
    /// Home folder quota in `unit`s, 0 for none.
    pub dir_quota: i64,
    /// Unit of `dir_quota`.
    pub unit: i64,
    /// A new password, in the clear, or `None` to keep the old one.
    pub password: Option<String>,
    /// [`EXPIRES_NEVER`], [`DISABLED`] or a Unix time in seconds.
    pub expire_time: i64,
    /// The expiry date as text, empty unless `expire_time` is a date.
    pub expire_date: String,
}

/// What `user/check/delete` says about the accounts to delete.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct DeleteCheck {
    /// An account has snapshots that would go with it.
    #[serde(default)]
    pub snapshot: bool,
    /// An account has a safe space.
    #[serde(default)]
    pub safespace: bool,
}

/// A group from `group/list`. Only the groups an administrator made are
/// listed; `admin`, `users` and `ughomeusers` are not.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct Group {
    /// Numeric group id, from 1000.
    #[serde(default)]
    pub gid: i64,
    /// Group name.
    #[serde(default)]
    pub groupname: String,
    /// Free-text description.
    #[serde(default)]
    pub description: String,
    /// The members, or `null` for none.
    #[serde(default, deserialize_with = "crate::types::common::null_vec_as_empty")]
    pub users: Vec<GroupMember>,
    /// When the group was last changed, in Unix seconds.
    #[serde(default)]
    pub utime: i64,
}

/// A member of a group.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct GroupMember {
    /// Login name.
    #[serde(default)]
    pub name: String,
    /// Numeric user id.
    #[serde(default)]
    pub uid: i64,
}
