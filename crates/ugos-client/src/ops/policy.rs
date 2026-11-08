//! Password policies, two-factor authentication, avatars, invitations and
//! imports of accounts.
//!
//! What turns off an account's second factor, opens a sign-up link, locks
//! accounts out until they change their password or creates many accounts at
//! once says so first and needs `force`.

use std::path::Path;

use serde_json::{Value, json};

use crate::api::syslog::SysLogApi;
use crate::client::UgosClient;
use crate::error::{Result, UgosError};
use crate::types::policy::{ImportRow, Invitation, PasswordExpiry, PasswordRules};

fn refused(message: impl Into<String>) -> UgosError {
    UgosError::Refused(message.into())
}

/// The strength rules of passwords.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn password_rules(client: &UgosClient) -> Result<PasswordRules> {
    client.get("user/config/global").await
}

/// What to change about the strength rules.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RulesChange {
    /// The shortest password, 6 to 127.
    pub min_length: Option<i64>,
    /// No user name in the password.
    pub no_username: Option<bool>,
    /// No common password.
    pub no_common: Option<bool>,
    /// Upper and lower case.
    pub mixed_case: Option<bool>,
    /// A digit.
    pub digit: Option<bool>,
    /// A special character.
    pub special: Option<bool>,
}

/// Change the strength rules and read them back.
///
/// # Errors
///
/// Refuses a length outside 6 to 127.
pub async fn set_password_rules(
    client: &UgosClient,
    change: &RulesChange,
) -> Result<PasswordRules> {
    let mut next = password_rules(client).await?;
    if let Some(n) = change.min_length {
        if !(6..=127).contains(&n) {
            return Err(refused("the shortest password is 6 to 127 characters"));
        }
        next.min_length = n;
    }
    next.disable_contain_key = change.no_username.unwrap_or(next.disable_contain_key);
    next.disable_common_pwd = change.no_common.unwrap_or(next.disable_common_pwd);
    next.must_contain_case = change.mixed_case.unwrap_or(next.must_contain_case);
    next.must_contain_num = change.digit.unwrap_or(next.must_contain_num);
    next.must_contain_special_char = change.special.unwrap_or(next.must_contain_special_char);
    let _: Value = client.post("user/config/global", &next).await?;
    let after = password_rules(client).await?;
    if after != next {
        return Err(refused(
            "UGOS said yes but the password rules did not change",
        ));
    }
    Ok(after)
}

/// The expiry of passwords.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn password_expiry(client: &UgosClient) -> Result<PasswordExpiry> {
    client.get("user/config/password").await
}

/// What to change about the expiry of passwords.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExpiryChange {
    /// Whether passwords expire.
    pub enable: Option<bool>,
    /// Days a password is valid, 1 to 999.
    pub max_days: Option<i64>,
    /// Days of warning, 0 to 99, below the validity.
    pub warn_days: Option<i64>,
    /// Force the change at expiry.
    pub force_change: Option<bool>,
    /// The accounts whose password does not expire, replacing the list.
    pub exempt: Option<Vec<String>>,
}

/// Change the expiry of passwords and read it back.
///
/// # Errors
///
/// Returns [`UgosError::NeedsForce`] when expiry is on with a forced change
/// afterwards: accounts are locked out until they change their password.
pub async fn set_password_expiry(
    client: &UgosClient,
    change: &ExpiryChange,
    force: bool,
) -> Result<PasswordExpiry> {
    let mut next = password_expiry(client).await?;
    next.enable = change.enable.unwrap_or(next.enable);
    next.force_change = change.force_change.unwrap_or(next.force_change);
    if let Some(n) = change.max_days {
        if !(1..=999).contains(&n) {
            return Err(refused("a password is valid for 1 to 999 days"));
        }
        next.max_days = n;
    }
    if let Some(n) = change.warn_days {
        if !(0..=99).contains(&n) {
            return Err(refused("the warning is 0 to 99 days"));
        }
        next.warn_age = n;
    }
    if next.warn_age > 0 && next.max_days <= next.warn_age {
        return Err(refused("the validity must be longer than the warning"));
    }
    if let Some(names) = &change.exempt {
        let users = client.users().await?;
        if let Some(bad) = names
            .iter()
            .find(|n| !users.iter().any(|u| &&u.username == n))
        {
            return Err(refused(format!("there is no account {bad:?}")));
        }
        next.white_list.clone_from(names);
    }
    if next.enable && next.force_change && !force {
        return Err(UgosError::NeedsForce(
            "accounts whose password expires are locked out until they change it".into(),
        ));
    }
    let _: Value = client.post("user/config/password", &next).await?;
    let after = password_expiry(client).await?;
    if after.enable != next.enable
        || after.max_days != next.max_days
        || after.force_change != next.force_change
    {
        return Err(refused(
            "UGOS said yes but the password expiry did not change",
        ));
    }
    Ok(after)
}

/// When an account last changed its password, Unix seconds.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn last_password_change(client: &UgosClient, uid: i64) -> Result<i64> {
    let v: Value = client
        .get(&format!("user/passwd/lastChangeTime/{uid}"))
        .await?;
    Ok(v.get("result").and_then(Value::as_i64).unwrap_or(0))
}

/// Switch off the second factor of an account.
///
/// # Errors
///
/// Returns [`UgosError::NeedsForce`] without `force`.
pub async fn disable_otp(client: &UgosClient, uid: i64, force: bool) -> Result<()> {
    if uid <= 0 {
        return Err(refused("a uid is above 0"));
    }
    if !force {
        return Err(UgosError::NeedsForce(
            "the account then has no second factor".into(),
        ));
    }
    let _: Value = client.post("otp/disable", &json!({ "uid": uid })).await?;
    Ok(())
}

/// The avatars UGOS offers.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn avatars(client: &UgosClient) -> Result<Vec<String>> {
    let v: Value = client.get("user/headpic/list").await?;
    Ok(serde_json::from_value(v["list"].clone()).unwrap_or_default())
}

/// Which avatar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Avatar {
    /// One of the offered ones, by name.
    Builtin(String),
    /// A picture on this machine: `.jpg`, `.png`, `.gif`, `.bmp` or `.webp`,
    /// under 8 MB.
    File(std::path::PathBuf),
}

/// Set the avatar of an account. Returns the new avatar id.
///
/// # Errors
///
/// Refuses an avatar that is not offered and a picture the web UI refuses.
pub async fn set_avatar(client: &UgosClient, uid: i64, avatar: &Avatar) -> Result<String> {
    let mut form = reqwest::multipart::Form::new();
    match avatar {
        Avatar::Builtin(name) => {
            if !avatars(client).await?.contains(name) {
                return Err(refused(format!(
                    "{name:?} is not one of the offered avatars"
                )));
            }
            form = form.text("icon", name.clone()).text("file", "");
        }
        Avatar::File(path) => {
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            if !["jpg", "jpeg", "png", "gif", "bmp", "webp"].contains(&ext.as_str()) {
                return Err(refused(
                    "a picture is .jpg, .jpeg, .png, .gif, .bmp or .webp",
                ));
            }
            let bytes = std::fs::read(path)
                .map_err(|e| UgosError::io(format!("reading '{}'", path.display()), e))?;
            if bytes.is_empty() || bytes.len() >= 8 * 1024 * 1024 {
                return Err(refused("a picture is under 8 MB"));
            }
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("avatar")
                .to_owned();
            form = form.text("icon", "").part(
                "file",
                reqwest::multipart::Part::bytes(bytes).file_name(name),
            );
        }
    }
    let v: Value = client
        .post_multipart(&format!("user/change/headpic/{uid}"), form)
        .await?;
    Ok(v.get("icon_value")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned())
}

/// The code of the invitation link, empty when invitations are off.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn invite_code(client: &UgosClient, admin_uid: i64) -> Result<String> {
    let v: Value = client
        .get(&format!("user/invite/state/{admin_uid}"))
        .await?;
    Ok(v.get("result")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned())
}

/// The addresses a link can be built from: ports, host names and addresses.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn invite_link_base(client: &UgosClient) -> Result<Value> {
    client.get("user/priority_link").await
}

/// Switch invitations on or off.
///
/// # Errors
///
/// Returns [`UgosError::NeedsForce`] for switching them on: anyone who has
/// the link can then apply for an account.
pub async fn set_invites(
    client: &UgosClient,
    admin_uid: i64,
    on: bool,
    force: bool,
) -> Result<String> {
    if on && !force {
        return Err(UgosError::NeedsForce(
            "anyone who has the link can then apply for an account".into(),
        ));
    }
    let body = json!({ "uid": admin_uid, "enabled": on });
    let _: Value = client.post("user/invite/switch", &body).await?;
    let code = invite_code(client, admin_uid).await?;
    if code.is_empty() == on {
        return Err(refused("UGOS said yes but the invitations did not change"));
    }
    Ok(code)
}

/// The applications waiting, or the ones that were processed.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn invitations(client: &UgosClient, processed: bool) -> Result<Vec<Invitation>> {
    let path = if processed {
        "user/invite/agree/list"
    } else {
        "user/invite/list"
    };
    let v: Value = client.get(path).await?;
    match v.get("result") {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(list) => Ok(serde_json::from_value(list.clone())?),
    }
}

/// How an approved applicant gets set up.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Approval {
    /// `admin` or `users`.
    pub role: String,
    /// Groups to join.
    pub groups: Vec<String>,
    /// A description.
    pub description: String,
    /// Give a home folder.
    pub home_dir: bool,
}

/// Approve an application: UGOS makes the account with the applicant's own
/// password.
///
/// # Errors
///
/// Returns [`UgosError::NeedsForce`] for the administrator role.
pub async fn approve(client: &UgosClient, id: i64, how: &Approval, force: bool) -> Result<()> {
    if how.role == "admin" && !force {
        return Err(UgosError::NeedsForce(
            "an administrator can manage the NAS".into(),
        ));
    }
    if how.role != "admin" && how.role != "users" {
        return Err(refused("the role is admin or users"));
    }
    if !invitations(client, false).await?.iter().any(|i| i.id == id) {
        return Err(refused(format!("there is no application {id}")));
    }
    let body = json!({
        "id": id,
        "account": {
            "role": how.role, "description": how.description, "email": "",
            "groups": how.groups.iter().map(|g| json!({ "name": g })).collect::<Vec<_>>(),
            "enable_home_dir": how.home_dir, "dir_quota": 0, "unit": 1,
        },
    });
    let _: Value = client.post("user/invite/agree", &body).await?;
    Ok(())
}

/// Reject or delete an application.
///
/// # Errors
///
/// Refuses an id that no application has.
pub async fn reject(client: &UgosClient, id: i64) -> Result<()> {
    let all = invitations(client, false).await?;
    if !all.iter().any(|i| i.id == id) {
        return Err(refused(format!("there is no application {id}")));
    }
    let _: Value = client
        .delete_with_body(&format!("user/invite/{id}"), &json!({}))
        .await?;
    Ok(())
}

/// Read an import file (`.xls` or `.xlsx`, under 8 MB) and get the rows UGOS
/// makes of it, valid or not. Nothing is created.
///
/// # Errors
///
/// Refuses a file the web UI refuses, and an import with no rows.
pub async fn read_import(client: &UgosClient, file: &Path) -> Result<Vec<ImportRow>> {
    let ext = file
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if ext != "xls" && ext != "xlsx" {
        return Err(refused("an import file is .xls or .xlsx"));
    }
    let bytes = std::fs::read(file)
        .map_err(|e| UgosError::io(format!("reading '{}'", file.display()), e))?;
    if bytes.is_empty() || bytes.len() >= 8 * 1024 * 1024 {
        return Err(refused("an import file is under 8 MB"));
    }
    let name = file
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("users.xlsx")
        .to_owned();
    let form = reqwest::multipart::Form::new().part(
        "file",
        reqwest::multipart::Part::bytes(bytes).file_name(name),
    );
    let v: Value = client.post_multipart("user/import", form).await?;
    let rows: Vec<ImportRow> = match v.get("result") {
        None | Some(Value::Null) => Vec::new(),
        Some(list) => serde_json::from_value(list.clone())?,
    };
    if rows.is_empty() {
        return Err(refused("UGOS found no accounts in that file"));
    }
    Ok(rows)
}

/// Create the accounts of an import. Every row has to be valid.
///
/// # Errors
///
/// Returns [`UgosError::NeedsForce`] without `force`, and refuses an import
/// with an invalid row.
pub async fn commit_import(
    client: &UgosClient,
    rows: &[ImportRow],
    change_password: bool,
    force: bool,
) -> Result<()> {
    if let Some(bad) = rows.iter().find(|r| !r.valid) {
        return Err(refused(format!("{}: {}", bad.username, bad.err_tip)));
    }
    if !force {
        return Err(UgosError::NeedsForce(format!(
            "this creates {} accounts, whose passwords are in the file",
            rows.len()
        )));
    }
    let body = json!({ "force_change_pwd": change_password, "overwrite": false, "list": rows });
    let _: Value = client.post("user/import/commit", &body).await?;
    Ok(())
}

/// Whether an import is running.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn import_running(client: &UgosClient) -> Result<bool> {
    let v: Value = client.get("user/import/state").await?;
    Ok(v.get("is_running")
        .and_then(Value::as_bool)
        .unwrap_or(false))
}
