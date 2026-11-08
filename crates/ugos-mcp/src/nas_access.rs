//! Account creation, groups, share permissions and quotas: the logic behind
//! the `ugos_user_create`, `ugos_group_*`, `ugos_*_permissions`,
//! `ugos_*_permit` and `ugos_user_quota*` tools.
//!
//! The same operations as `ugos-cli`. The tools that take a password take it
//! as a parameter: gate or switch them off where a model should not handle
//! passwords. Deleting a group with members needs `force`.

use rmcp::schemars;
use serde::Serialize;
use ugos_client::UgosClient;
use ugos_client::api::users::UsersApi;
use ugos_client::error::UgosError;
use ugos_client::ops::access::{self, GroupChange, Level, Owner};
use ugos_client::ops::users::{UserRequest, create_user};
use ugos_client::types::users::{ROLE_ADMIN, ROLE_USER};

use crate::manage::failure;

fn json<T: Serialize>(result: Result<T, UgosError>) -> String {
    match result {
        Ok(value) => serde_json::to_string_pretty(&value).unwrap_or_default(),
        Err(e) => failure(&e),
    }
}

fn refused(message: &str) -> UgosError {
    UgosError::Refused(message.into())
}

/// Make an account.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct UserCreateParam {
    /// Login name.
    pub name: String,
    /// The password of the account.
    pub password: String,
    /// Email address.
    #[serde(default)]
    pub email: Option<String>,
    /// A description.
    #[serde(default)]
    pub description: Option<String>,
    /// `user` (default) or `admin`. An administrator can manage the NAS: ask
    /// the user first.
    #[serde(default)]
    pub role: Option<String>,
    /// Groups to join.
    #[serde(default)]
    pub groups: Vec<String>,
    /// Give the account no home folder.
    #[serde(default)]
    pub no_home_dir: bool,
    /// Forbid the account to change its own password.
    #[serde(default)]
    pub deny_password_change: bool,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// Make a group.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AccessGroupCreateParam {
    /// Name: not only digits, not `admin`, no leading `-`, up to 64.
    pub name: String,
    /// A description, up to 64 characters.
    #[serde(default)]
    pub description: String,
    /// Accounts to put in it.
    #[serde(default)]
    pub members: Vec<String>,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// Change a group.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AccessGroupEditParam {
    /// Name or gid.
    pub group: String,
    /// A new name.
    #[serde(default)]
    pub new_name: Option<String>,
    /// A new description.
    #[serde(default)]
    pub description: Option<String>,
    /// Accounts to add.
    #[serde(default)]
    pub add: Vec<String>,
    /// Accounts to remove.
    #[serde(default)]
    pub remove: Vec<String>,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// A group, for reading permissions or deleting.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AccessGroupRefParam {
    /// Name or gid.
    pub group: String,
    /// Deleting: go on although the group has members. Ask the user first.
    #[serde(default)]
    pub force: bool,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// An account or group, for reading permissions.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct OwnerParam {
    /// Login name or uid for an account, name or gid for a group.
    pub name: String,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// Set permissions on shares.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PermitParam {
    /// Login name or uid for an account, name or gid for a group.
    pub name: String,
    /// Share by `none`, `read`, `write` or `inherit`, like `{"docs": "read"}`.
    pub shares: std::collections::BTreeMap<String, String>,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// Set a quota.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct QuotaSetParam {
    /// Login name or uid.
    pub account: String,
    /// The volume, like `volume1`.
    pub volume: String,
    /// A whole number of MB, GB or TB, like `500M`, `100G` or `2T`; `none`
    /// removes the limit.
    pub limit: String,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

pub async fn user_create(c: &UgosClient, p: &UserCreateParam) -> String {
    let run = async {
        let role = match p.role.as_deref() {
            None | Some("user") => ROLE_USER,
            Some("admin") => ROLE_ADMIN,
            Some(_) => return Err(refused("the role is admin or user")),
        };
        let request = UserRequest {
            name: p.name.clone(),
            email: p.email.clone(),
            description: p.description.clone(),
            role: role.to_owned(),
            groups: p.groups.clone(),
            home_dir: !p.no_home_dir,
            deny_password_change: p.deny_password_change,
        };
        create_user(c, &request, &p.password).await
    };
    json(run.await)
}

pub async fn group_create(c: &UgosClient, p: &AccessGroupCreateParam) -> String {
    json(access::create_group(c, &p.name, &p.description, &p.members).await)
}

pub async fn group_edit(c: &UgosClient, p: &AccessGroupEditParam) -> String {
    let change = GroupChange {
        name: p.new_name.clone(),
        description: p.description.clone(),
        add: p.add.clone(),
        remove: p.remove.clone(),
    };
    json(access::edit_group(c, &p.group, &change).await)
}

pub async fn group_delete(c: &UgosClient, p: &AccessGroupRefParam) -> String {
    access::delete_group(c, &p.group, p.force)
        .await
        .map_or_else(|e| failure(&e), |()| "The group is deleted".into())
}

async fn owner(c: &UgosClient, name: &str, group: bool) -> Result<Owner, UgosError> {
    if group {
        Ok(Owner::Group(access::find_group(c, name).await?.gid))
    } else {
        Ok(Owner::User(c.user_find(name).await?.uid))
    }
}

pub async fn permissions(c: &UgosClient, p: &OwnerParam, group: bool) -> String {
    let run = async { access::permissions(c, owner(c, &p.name, group).await?).await };
    json(run.await)
}

pub async fn permit(c: &UgosClient, p: &PermitParam, group: bool) -> String {
    let run = async {
        let mut changes = Vec::new();
        for (share, text) in &p.shares {
            let level = match text.as_str() {
                "none" => Level::None,
                "read" => Level::Read,
                "write" => Level::Write,
                "inherit" => Level::Inherit,
                _ => return Err(refused("the level is none, read, write or inherit")),
            };
            changes.push((share.clone(), level));
        }
        access::set_permissions(c, owner(c, &p.name, group).await?, &changes).await
    };
    json(run.await)
}

pub async fn quota(c: &UgosClient, p: &OwnerParam) -> String {
    let run = async { access::quota(c, c.user_find(&p.name).await?.uid).await };
    json(run.await)
}

fn limit(text: &str) -> Result<Option<(i64, &'static str)>, UgosError> {
    if text.eq_ignore_ascii_case("none") {
        return Ok(None);
    }
    let upper = text.trim().to_ascii_uppercase();
    let (digits, unit) = [
        ("TB", "TB"),
        ("T", "TB"),
        ("GB", "GB"),
        ("G", "GB"),
        ("MB", "MB"),
        ("M", "MB"),
    ]
    .iter()
    .find_map(|(suffix, unit)| upper.strip_suffix(suffix).map(|d| (d, *unit)))
    .ok_or_else(|| refused("a limit is a whole number of M, G or T, or none"))?;
    let value = digits
        .trim()
        .parse()
        .map_err(|_| refused("a limit is a whole number of M, G or T, or none"))?;
    Ok(Some((value, unit)))
}

pub async fn quota_set(c: &UgosClient, p: &QuotaSetParam) -> String {
    let run = async {
        let user = c.user_find(&p.account).await?;
        access::set_quota(c, user.uid, &p.volume, limit(&p.limit)?).await
    };
    json(run.await)
}
