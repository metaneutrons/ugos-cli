//! Groups, share permissions and per-user quotas.
//!
//! Deleting a group that has members, and changing who may use a share, change
//! what people can reach, so the first needs `force`, as the web UI asks a
//! confirmation. Quotas are in whole MB, GB or TB, 1024-based.

use serde_json::{Value, json};

use crate::api::syslog::SysLogApi;
use crate::api::users::UsersApi;
use crate::client::UgosClient;
use crate::error::{Result, UgosError};
use crate::types::access::{GroupInfo, PermissionList, PermissionRow, Quota, QuotaRow};
use crate::types::syslog::User;
use crate::types::users::Group;

fn refused(message: impl Into<String>) -> UgosError {
    UgosError::Refused(message.into())
}

/// A group by name or id.
///
/// # Errors
///
/// Refuses a name that no group has.
pub async fn find_group(client: &UgosClient, name_or_id: &str) -> Result<Group> {
    client
        .group_list()
        .await?
        .into_iter()
        .find(|g| g.groupname == name_or_id || g.gid.to_string() == name_or_id)
        .ok_or_else(|| refused(format!("there is no group {name_or_id:?}")))
}

/// The name rules of the web UI.
fn check_group_name(name: &str) -> Result<()> {
    let lower = name.to_ascii_lowercase();
    if name.is_empty() || name.chars().count() > 64 {
        return Err(refused("a group name is 1 to 64 characters"));
    }
    if name.chars().all(|c| c.is_ascii_digit()) {
        return Err(refused("a group name is not only digits"));
    }
    if ["root", "admin", "administrator", "none"].contains(&lower.as_str()) {
        return Err(refused(format!("{name:?} is taken for the system")));
    }
    if name.starts_with('-') || name != name.trim() {
        return Err(refused(
            "a group name does not start with - and has no space at its ends",
        ));
    }
    Ok(())
}

/// Make a group.
///
/// # Errors
///
/// Refuses a name the web UI refuses or a member that does not exist.
pub async fn create_group(
    client: &UgosClient,
    name: &str,
    description: &str,
    members: &[String],
) -> Result<Group> {
    check_group_name(name)?;
    if description.chars().count() > 64 {
        return Err(refused("a description is up to 64 characters"));
    }
    if find_group(client, name).await.is_ok() {
        return Err(refused(format!("a group {name:?} exists already")));
    }
    check_members(client, members).await?;
    let body = json!({
        "groupname": name, "description": description,
        "users": members.iter().map(|m| json!({ "name": m })).collect::<Vec<_>>(),
    });
    let _: Value = client.post("group/create", &body).await?;
    find_group(client, name).await
}

async fn check_members(client: &UgosClient, members: &[String]) -> Result<()> {
    let users = client.users().await?;
    for m in members {
        if !users.iter().any(|u| &u.username == m) {
            return Err(refused(format!("there is no account {m:?}")));
        }
    }
    Ok(())
}

/// What to change about a group.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GroupChange {
    /// A new name.
    pub name: Option<String>,
    /// A new description.
    pub description: Option<String>,
    /// Accounts to add.
    pub add: Vec<String>,
    /// Accounts to remove.
    pub remove: Vec<String>,
}

/// Change a group. UGOS takes the full member list, so this reads it, edits
/// it and sends it back.
///
/// # Errors
///
/// Refuses what the web UI refuses.
pub async fn edit_group(
    client: &UgosClient,
    name_or_id: &str,
    change: &GroupChange,
) -> Result<Group> {
    let group = find_group(client, name_or_id).await?;
    let info: GroupInfo = client.get(&format!("group/info/{}", group.gid)).await?;
    let name = change
        .name
        .clone()
        .unwrap_or_else(|| info.groupname.clone());
    let description = change
        .description
        .clone()
        .unwrap_or_else(|| info.description.clone());
    check_group_name(&name)?;
    if name != info.groupname && find_group(client, &name).await.is_ok() {
        return Err(refused(format!("a group {name:?} exists already")));
    }
    check_members(client, &change.add).await?;
    let mut members: Vec<String> = info.users.iter().map(|u| u.name.clone()).collect();
    members.retain(|m| !change.remove.contains(m));
    for m in &change.add {
        if !members.contains(m) {
            members.push(m.clone());
        }
    }
    let body = json!({
        "gid": group.gid, "groupname": name, "description": description,
        "users": members.iter().map(|m| json!({ "name": m })).collect::<Vec<_>>(),
    });
    let _: Value = client.post("group/edit", &body).await?;
    let after = find_group(client, &name).await?;
    let mut got: Vec<&str> = after.users.iter().map(|u| u.name.as_str()).collect();
    let mut want: Vec<&str> = members.iter().map(String::as_str).collect();
    got.sort_unstable();
    want.sort_unstable();
    if got != want || after.description != description {
        return Err(refused("UGOS said yes but the group did not change"));
    }
    Ok(after)
}

/// Delete a group.
///
/// # Errors
///
/// Returns [`UgosError::NeedsForce`] when the group has members.
pub async fn delete_group(client: &UgosClient, name_or_id: &str, force: bool) -> Result<()> {
    let group = find_group(client, name_or_id).await?;
    if !group.users.is_empty() && !force {
        return Err(UgosError::NeedsForce(format!(
            "the group has {} members, who lose what it gave them",
            group.users.len()
        )));
    }
    let _: Value = client
        .delete_with_body("group/delete", &[group.gid])
        .await?;
    if find_group(client, &group.groupname).await.is_ok() {
        return Err(refused("UGOS said yes but the group is still there"));
    }
    Ok(())
}

/// Whose permissions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Owner {
    /// An account.
    User(i64),
    /// A group.
    Group(i64),
}

impl Owner {
    fn path(self) -> String {
        match self {
            Self::User(uid) => format!("user/permission/{uid}"),
            Self::Group(gid) => format!("group/permission/{gid}"),
        }
    }
}

/// What an account or a group may do on each share.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn permissions(client: &UgosClient, owner: Owner) -> Result<Vec<PermissionRow>> {
    let list: PermissionList = client.get(&owner.path()).await?;
    Ok(list.result)
}

/// What to allow on a share.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    /// No access.
    None,
    /// Read only.
    Read,
    /// Read and write.
    Write,
    /// Nothing set, so the groups decide.
    Inherit,
}

impl Level {
    /// The code UGOS takes.
    #[must_use]
    pub const fn code(self) -> i64 {
        match self {
            Self::None => 0,
            Self::Read => 5,
            Self::Write => 7,
            Self::Inherit => -1,
        }
    }
}

/// Set what an account or a group may do on shares, and read it back.
///
/// # Errors
///
/// Refuses a share that does not exist, and read-only for an administrator,
/// who always gets read and write.
pub async fn set_permissions(
    client: &UgosClient,
    owner: Owner,
    changes: &[(String, Level)],
) -> Result<Vec<PermissionRow>> {
    if changes.is_empty() {
        return Err(refused("name at least one share and a level"));
    }
    let rows = permissions(client, owner).await?;
    if let Owner::User(uid) = owner {
        let user: User = client.user_find(&uid.to_string()).await?;
        if user.role == "admin" && changes.iter().any(|(_, l)| *l == Level::Read) {
            return Err(refused("an administrator gets read and write: use write"));
        }
    }
    let mut list = Vec::new();
    for (share, level) in changes {
        let row = rows
            .iter()
            .find(|r| &r.name == share || &r.path == share)
            .ok_or_else(|| refused(format!("there is no share {share:?}")))?;
        list.push(json!({ "path": row.path, "permission": level.code() }));
    }
    let _: Value = client.post(&owner.path(), &json!({ "list": list })).await?;
    let after = permissions(client, owner).await?;
    for (share, level) in changes {
        let got = after.iter().find(|r| &r.name == share || &r.path == share);
        let same = got.is_some_and(|r| match level {
            Level::Read => r.permission == 5 || r.permission == 4,
            Level::Write => r.permission == 7 || r.permission == 6,
            other => r.permission == other.code(),
        });
        if !same {
            return Err(refused(format!("UGOS said yes but {share} did not change")));
        }
    }
    Ok(after)
}

/// Whether this NAS offers quotas per account.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn quota_available(client: &UgosClient) -> Result<bool> {
    let v: Value = client.get("user/check/quota").await?;
    Ok(v.get("result").and_then(Value::as_bool).unwrap_or(true))
}

/// The limits of an account, per volume.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn quota(client: &UgosClient, uid: i64) -> Result<Quota> {
    client.get(&format!("user/quota/{uid}")).await
}

/// The KiB for a number of MB, GB or TB.
///
/// # Errors
///
/// Refuses a unit that is none of those.
pub fn kib(value: i64, unit: &str) -> Result<i64> {
    let factor: i64 = match unit {
        "MB" => 1 << 10,
        "GB" => 1 << 20,
        "TB" => 1 << 30,
        _ => return Err(refused("the unit is MB, GB or TB")),
    };
    Ok(value.saturating_mul(factor))
}

/// Set the limit of an account on a volume, in whole MB, GB or TB, or take it
/// away with `None`.
///
/// # Errors
///
/// Refuses when the NAS offers no quotas, a volume the account has no row
/// for, and a limit above what fits.
pub async fn set_quota(
    client: &UgosClient,
    uid: i64,
    volume: &str,
    limit: Option<(i64, &str)>,
) -> Result<Quota> {
    if !quota_available(client).await? {
        return Err(refused("this NAS offers no quotas per account"));
    }
    let current = quota(client, uid).await?;
    let row: &QuotaRow = current
        .btrfs
        .iter()
        .chain(&current.ext4)
        .find(|r| r.path == volume || r.path.rsplit('/').next() == Some(volume))
        .ok_or_else(|| refused(format!("there is no volume {volume:?} to limit")))?;
    let hard = match limit {
        None => 0,
        Some((value, unit)) => {
            if value <= 0 {
                return Err(refused("a limit is above 0; leave it out to remove one"));
            }
            let k = kib(value, unit)?;
            if row.max_value > 0 && k.saturating_mul(1024) > row.max_value {
                return Err(refused("that is more than the volume holds"));
            }
            k
        }
    };
    let body = json!([{ "path": row.path, "hard": hard }]);
    let _: Value = client.post(&format!("user/quota/{uid}"), &body).await?;
    let after = quota(client, uid).await?;
    let got = after
        .btrfs
        .iter()
        .chain(&after.ext4)
        .find(|r| r.path == row.path);
    if got.is_none_or(|r| r.hard != hard) {
        return Err(refused("UGOS said yes but the limit did not change"));
    }
    Ok(after)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::{Level, check_group_name, kib};

    #[test]
    fn group_names_follow_the_web_ui() {
        assert!(check_group_name("team-a").is_ok());
        assert!(check_group_name("1234").is_err());
        assert!(check_group_name("Admin").is_err());
        assert!(check_group_name("None").is_err());
        assert!(check_group_name("-x").is_err());
        assert!(check_group_name(" x").is_err());
        assert!(check_group_name("").is_err());
        assert!(check_group_name(&"a".repeat(65)).is_err());
    }

    #[test]
    fn levels_and_sizes_have_their_codes() {
        assert_eq!(
            [Level::None, Level::Read, Level::Write, Level::Inherit].map(Level::code),
            [0, 5, 7, -1]
        );
        assert_eq!(kib(1, "MB").unwrap(), 1024);
        assert_eq!(kib(2, "GB").unwrap(), 2 << 20);
        assert_eq!(kib(1, "TB").unwrap(), 1 << 30);
        assert!(kib(1, "KB").is_err());
    }
}
