//! Guarded operations on user accounts.
//!
//! What `ugos-cli user` and the MCP server both do. UGOS blanks an email
//! address that an edit leaves out, keeps accounts in groups they were asked
//! to leave, and deletes a home folder with its files, so each operation
//! sends what the web UI would, refuses what would lock the administrator
//! out, and reads the account back.

mod expiry;
#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests;

use crate::api::files::FilesApi;
use crate::api::syslog::SysLogApi;
use crate::api::users::UsersApi;
use crate::client::UgosClient;
use crate::crypto::decrypt_email;
use crate::error::{Result, UgosError};
use crate::types::syslog::User;
use crate::types::users::{NewUser, ROLE_ADMIN, UserEdit};

pub use self::expiry::{Expiry, parse_date, today};

/// Names the web UI refuses, whatever the case.
const RESERVED_NAMES: [&str; 3] = ["root", "admin", "administrator"];

/// What creating an account was asked for. The password goes separately.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Default)]
pub struct UserRequest {
    /// Login name.
    pub name: String,
    /// Email address.
    pub email: Option<String>,
    /// Free-text description.
    pub description: Option<String>,
    /// [`ROLE_ADMIN`] or [`ROLE_USER`]; a standard user when empty.
    pub role: String,
    /// Groups to join.
    pub groups: Vec<String>,
    /// Give the account a home folder.
    pub home_dir: bool,
    /// Forbid the account to change its own password.
    pub deny_password_change: bool,
}

/// The request that creating an account sends, without a password.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] for a name the web UI would refuse.
pub fn build_new_user(request: &UserRequest) -> Result<NewUser> {
    check_name(&request.name)?;
    let mut new = NewUser::new(request.name.trim(), "");
    request
        .email
        .as_deref()
        .unwrap_or_default()
        .trim()
        .clone_into(&mut new.email);
    new.description = request.description.clone().unwrap_or_default();
    if !request.role.is_empty() {
        new.role.clone_from(&request.role);
    }
    new.groups.clone_from(&request.groups);
    new.enable_home_dir = request.home_dir;
    new.deny_change_pwd = request.deny_password_change;
    if new.role == ROLE_ADMIN {
        // The web UI clears this for administrators.
        new.deny_change_pwd = false;
    }
    Ok(new)
}

/// Create an account and read it back.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] for a bad request, a name that is taken,
/// or an account UGOS made differently.
pub async fn create_user(
    client: &UgosClient,
    request: &UserRequest,
    password: &str,
) -> Result<User> {
    let mut new = build_new_user(request)?;
    if !client.user_name_free(&new.username).await? {
        refuse!("an account named '{}' already exists", new.username);
    }
    new.password = password.to_owned();
    let _ = client
        .user_create(&new)
        .await
        .map_err(|e| explain(e, !new.groups.is_empty()))?;

    // Read the account back: what UGOS holds is what counts.
    let made = client.user_find(&new.username).await?;
    if let Some(problem) = created_mismatch(&new, &made) {
        refuse!("UGOS created '{}' but {problem}", made.username);
    }
    Ok(made)
}

/// Change an account and check that UGOS kept the change.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] when there is nothing to change, when the
/// change would lock the administrator out, or when UGOS did not apply it.
pub async fn update_user(client: &UgosClient, account: &str, change: &UserChanges) -> Result<User> {
    if change.is_empty() {
        refuse!("nothing to change: give a field to set");
    }
    let before = client.user_find(account).await?;
    refuse_lockout(client, &before, change).await?;
    let edit = change.apply(&before)?;
    client
        .user_edit(&edit)
        .await
        .map_err(|e| explain(e, !change.add_groups.is_empty()))?;

    // UGOS may answer success and keep the old value, so look at what it holds.
    let now = client.user_find(&before.username).await?;
    if let Some(problem) = change.mismatch(&edit, &now) {
        refuse!("UGOS accepted the request but {problem}");
    }
    Ok(now)
}

/// UGOS answers an unknown group with a bare "Failed to operate"; say what
/// the likely cause is when groups were part of the request.
fn explain(error: UgosError, groups_named: bool) -> UgosError {
    match error {
        UgosError::Api { code: 9999, .. } if groups_named => UgosError::Refused(
            "UGOS refused the request; check the group names, only groups that exist can be set"
                .into(),
        ),
        other => other,
    }
}

/// The first respect in which the account differs from the request.
fn created_mismatch(asked: &NewUser, made: &User) -> Option<String> {
    if made.role != asked.role {
        return Some(format!("its role is '{}'", made.role));
    }
    let held = group_names(made);
    let missing: Vec<&str> = asked
        .groups
        .iter()
        .filter(|group| !held.contains(group))
        .map(String::as_str)
        .collect();
    if missing.is_empty() {
        None
    } else {
        Some(format!("it is not in: {}", missing.join(", ")))
    }
}

fn group_names(user: &User) -> Vec<String> {
    user.groups.iter().map(|group| group.name.clone()).collect()
}

/// A login name the web UI would accept.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] for a name the web UI would refuse.
pub fn check_name(name: &str) -> Result<()> {
    if name.trim().is_empty() {
        refuse!("the user name is empty");
    }
    if name != name.trim() {
        refuse!("the user name must not start or end with a space");
    }
    if name.starts_with('-') {
        refuse!("the user name must not start with '-'");
    }
    if RESERVED_NAMES.contains(&name.to_ascii_lowercase().as_str()) {
        refuse!("'{name}' is a name UGOS keeps for itself");
    }
    Ok(())
}

/// What `user update` was asked to change.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct UserChanges {
    /// New email address. The stored one is kept when `None`.
    pub email: Option<String>,
    /// New description.
    pub description: Option<String>,
    /// New role, [`ROLE_ADMIN`] or [`ROLE_USER`].
    pub role: Option<String>,
    /// Groups to join.
    pub add_groups: Vec<String>,
    /// Groups to leave.
    pub remove_groups: Vec<String>,
    /// Give or take away the home folder.
    pub home_dir: Option<bool>,
    /// Forbid or allow the account to change its own password.
    pub deny_password_change: Option<bool>,
    /// New expiry: never, disabled, or a date.
    pub expiry: Option<Expiry>,
    /// New password, in the clear.
    pub password: Option<String>,
}

impl UserChanges {
    /// Whether nothing is to change.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// The account as it should be after the change.
    pub(crate) fn apply(&self, user: &User) -> Result<UserEdit> {
        let (expire_time, expire_date) = self.expiry.as_ref().map_or_else(
            || (user.expire_time, user.expire_date.clone()),
            |expiry| (expiry.time, expiry.date.clone()),
        );
        Ok(UserEdit {
            uid: user.uid,
            username: user.username.clone(),
            email: match &self.email {
                Some(email) => email.trim().to_owned(),
                None => stored_email(user)?,
            },
            description: self
                .description
                .clone()
                .unwrap_or_else(|| user.description.clone()),
            role: self.role.clone().unwrap_or_else(|| user.role.clone()),
            groups: self.next_groups(user),
            enable_home_dir: self.home_dir.unwrap_or(user.enable_home_dir),
            deny_change_pwd: self.deny_password_change.unwrap_or(user.deny_change_pwd),
            dir_quota: user.dir_quota,
            unit: user.unit,
            password: self.password.clone(),
            expire_time,
            expire_date,
        })
    }

    /// Whether `now` shows another expiry than the one asked for. A date is
    /// compared as text: UGOS moves the time to midnight of its own time zone.
    pub(crate) fn expiry_differs(&self, edit: &UserEdit, now: &User) -> bool {
        let Some(asked) = &self.expiry else {
            return false;
        };
        if asked.date.is_empty() {
            now.expire_time != edit.expire_time
        } else {
            now.expire_date != asked.date || now.expire_time <= 1
        }
    }

    /// The groups of `user` with the named ones added and removed.
    fn next_groups(&self, user: &User) -> Vec<String> {
        let mut groups = group_names(user);
        groups.retain(|group| !self.remove_groups.contains(group));
        for group in &self.add_groups {
            if !groups.contains(group) {
                groups.push(group.clone());
            }
        }
        groups
    }

    /// The first change that `now` does not show, as a sentence.
    pub(crate) fn mismatch(&self, edit: &UserEdit, now: &User) -> Option<String> {
        if let Some(problem) =
            group_mismatch(&self.add_groups, &self.remove_groups, &group_names(now))
        {
            return Some(problem);
        }
        let checks = [
            (now.description != edit.description, "the description"),
            (now.role != edit.role, "the role"),
            (
                now.enable_home_dir != edit.enable_home_dir,
                "the home folder",
            ),
            (
                now.deny_change_pwd != edit.deny_change_pwd,
                "the password change rule",
            ),
            (self.expiry_differs(edit, now), "the expiry"),
            (
                now.email.is_empty() != edit.email.is_empty(),
                "the email address",
            ),
        ];
        checks
            .into_iter()
            .find(|(differs, _)| *differs)
            .map(|(_, what)| format!("{what} did not change"))
    }
}

/// The real email address of an account, which an edit must send back or UGOS
/// blanks it.
fn stored_email(user: &User) -> Result<String> {
    if user.email.is_empty() {
        return Ok(String::new());
    }
    if user.email_encrypted.is_empty() {
        refuse!(
            "UGOS shows only a masked address for '{}'; give an email address so that it is not lost",
            user.username
        );
    }
    decrypt_email(&user.email_encrypted).map_err(|e| {
        UgosError::Refused(format!(
            "cannot read the address of '{}' ({e}); give an email address to set it anew",
            user.username
        ))
    })
}

/// A sentence naming the groups that are not as asked, if there are any.
///
/// Only the groups that were named count. UGOS adds and drops `admin` and
/// `ughomeusers` itself, with the role and the home folder, so the whole list
/// is not what was sent.
fn group_mismatch(add: &[String], remove: &[String], held: &[String]) -> Option<String> {
    let missing: Vec<&str> = add
        .iter()
        .filter(|group| !held.contains(group))
        .map(String::as_str)
        .collect();
    if !missing.is_empty() {
        return Some(format!("the account did not join: {}", missing.join(", ")));
    }
    let kept: Vec<&str> = remove
        .iter()
        .filter(|group| held.contains(group))
        .map(String::as_str)
        .collect();
    if kept.is_empty() {
        None
    } else {
        Some(format!(
            "the account stays in: {} (the built-in groups cannot be left)",
            kept.join(", ")
        ))
    }
}

/// Refuse a change that would lock the administrator out.
async fn refuse_lockout(client: &UgosClient, target: &User, change: &UserChanges) -> Result<()> {
    let demotes = change
        .role
        .as_deref()
        .is_some_and(|role| role != ROLE_ADMIN)
        && target.role == ROLE_ADMIN;
    let disables = change.expiry.as_ref().is_some_and(Expiry::is_disabled);
    if !demotes && !disables {
        return Ok(());
    }
    if client.current_user().await?.uid == target.uid {
        refuse!("this is the account this session is logged in with; the change would lock it out");
    }
    if demotes && admins(&client.users().await?) <= 1 {
        refuse!("'{}' is the only administrator", target.username);
    }
    Ok(())
}

fn admins(users: &[User]) -> usize {
    users.iter().filter(|user| user.role == ROLE_ADMIN).count()
}

/// Delete an account. Its home folder and the files in it go with it, so an
/// account with files or snapshots is not deleted unless `force` says so.
///
/// # Errors
///
/// Returns [`UgosError::NeedsForce`] for files or snapshots that would be
/// lost, and [`UgosError::Refused`] for the account this session is logged in
/// with, the only administrator, or a safe space.
pub async fn delete_user(client: &UgosClient, account: &str, force: bool) -> Result<User> {
    let found = client.user_find(account).await?;
    refuse_lockout_by_delete(client, &found).await?;
    refuse_losses(client, &found, force).await?;
    client.user_delete(&[found.uid]).await?;
    let left = client.users().await?;
    if left.iter().any(|remaining| remaining.uid == found.uid) {
        refuse!(
            "UGOS accepted the request but '{}' is still there",
            found.username
        );
    }
    Ok(found)
}

/// Refuse to delete the account the CLI is logged in with, or the only
/// administrator.
async fn refuse_lockout_by_delete(client: &UgosClient, found: &User) -> Result<()> {
    if client.current_user().await?.uid == found.uid {
        refuse!("this is the account this session is logged in with");
    }
    if found.role == ROLE_ADMIN && admins(&client.users().await?) <= 1 {
        refuse!("'{}' is the only administrator", found.username);
    }
    Ok(())
}

/// Refuse a delete that would take data with it, unless `--force` says so.
/// A safe space is refused whatever `--force` says.
async fn refuse_losses(client: &UgosClient, found: &User, force: bool) -> Result<()> {
    let check = client.user_check_delete(&[found.uid]).await?;
    if check.safespace {
        refuse!(
            "'{}' has a safe space; delete that first in the web UI",
            found.username
        );
    }
    if force {
        return Ok(());
    }
    if check.snapshot {
        return Err(UgosError::NeedsForce(format!(
            "'{}' has snapshots that would be deleted with the account",
            found.username
        )));
    }
    if found.enable_home_dir {
        match home_entries(client, found).await {
            Ok(0) => {}
            Ok(count) => {
                return Err(UgosError::NeedsForce(format!(
                    "the home folder of '{}' is not empty ({count} {}), and deleting the account deletes it",
                    found.username,
                    if count == 1 { "entry" } else { "entries" }
                )));
            }
            Err(e) => {
                return Err(UgosError::NeedsForce(format!(
                    "cannot tell whether the home folder of '{}' is empty ({e})",
                    found.username
                )));
            }
        }
    }
    Ok(())
}

/// How many entries the home folder of an account holds. A folder that does
/// not exist holds none.
async fn home_entries(client: &UgosClient, user: &User) -> Result<usize> {
    match client.fs_list(&format!("/home/{}", user.username)).await {
        Ok(files) => Ok(files.len()),
        Err(UgosError::Api { code: 1302, .. }) => Ok(0),
        Err(e) => Err(e),
    }
}
