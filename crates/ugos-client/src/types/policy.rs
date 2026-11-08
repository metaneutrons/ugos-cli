//! Types for password policies, invitations and imports of accounts.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::common::null_vec_as_empty;

/// The strength rules every password has to meet: `user/config/global`.
// One flag per field of the API's flat shape; an enum would only restate them.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PasswordRules {
    /// The password may not contain the user name.
    #[serde(default)]
    pub disable_contain_key: bool,
    /// The password may not be a common one.
    #[serde(default)]
    pub disable_common_pwd: bool,
    /// Upper and lower case are needed.
    #[serde(default)]
    pub must_contain_case: bool,
    /// A digit is needed.
    #[serde(default)]
    pub must_contain_num: bool,
    /// A special character is needed.
    #[serde(default)]
    pub must_contain_special_char: bool,
    /// The shortest password, 6 to 127.
    #[serde(default)]
    pub min_length: i64,
}

/// The expiry of passwords: `user/config/password`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PasswordExpiry {
    /// Whether passwords expire.
    #[serde(default)]
    pub enable: bool,
    /// Days a password is valid.
    #[serde(default)]
    pub max_days: i64,
    /// Days before the end at which a warning appears.
    #[serde(default)]
    pub warn_age: i64,
    /// Whether the change is forced when it expires.
    #[serde(default)]
    pub force_change: bool,
    /// Accounts whose password does not expire, by name.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub white_list: Vec<String>,
}

/// An application for an account, made through the invitation link.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Invitation {
    /// Id.
    #[serde(default, rename = "ID")]
    pub id: i64,
    /// The wanted name.
    #[serde(default)]
    pub username: String,
    /// The email address.
    #[serde(default)]
    pub email: String,
    /// The reason given.
    #[serde(default)]
    pub reason: String,
    /// When, Unix seconds.
    #[serde(default, rename = "CreatedAt")]
    pub created_at: i64,
    /// Who invited, for processed ones.
    #[serde(default)]
    pub invite_name: String,
    /// Who approved, for processed ones.
    #[serde(default)]
    pub agree_name: String,
}

/// A row of an import file, as UGOS reads it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportRow {
    /// The name.
    #[serde(default)]
    pub username: String,
    /// The password, in the clear.
    #[serde(default)]
    pub password: String,
    /// A description.
    #[serde(default)]
    pub description: String,
    /// The email address.
    #[serde(default)]
    pub email: String,
    /// `admin` or `users`.
    #[serde(default)]
    pub role: String,
    /// The groups.
    #[serde(default)]
    pub groups: Vec<Value>,
    /// Whether the row can be imported.
    #[serde(default)]
    pub valid: bool,
    /// Why not.
    #[serde(default)]
    pub err_tip: String,
}
