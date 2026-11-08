//! Password policies, two-factor authentication, avatars, invitations and
//! imports of accounts: the logic behind the `ugos_user_*` tools of that
//! kind.
//!
//! The same operations as `ugos-cli`. What turns off a second factor, opens a
//! sign-up link, locks accounts out or creates many accounts needs `force`,
//! and the model is told to ask the user first.

use std::path::{Path, PathBuf};

use rmcp::schemars;
use serde::Serialize;
use ugos_client::UgosClient;
use ugos_client::api::syslog::SysLogApi;
use ugos_client::api::users::UsersApi;
use ugos_client::error::UgosError;
use ugos_client::ops::policy::{self, Approval, Avatar, ExpiryChange, RulesChange};

use crate::manage::failure;

fn json<T: Serialize>(result: Result<T, UgosError>) -> String {
    match result {
        Ok(value) => serde_json::to_string_pretty(&value).unwrap_or_default(),
        Err(e) => failure(&e),
    }
}

fn done(result: Result<(), UgosError>, text: &str) -> String {
    result.map_or_else(|e| failure(&e), |()| text.to_owned())
}

fn refused(message: &str) -> UgosError {
    UgosError::Refused(message.into())
}

/// Strength rules of passwords. Without a field, that part stays.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct RulesParam {
    /// The shortest password, 6 to 127.
    #[serde(default)]
    pub min_length: Option<i64>,
    /// No user name in the password.
    #[serde(default)]
    pub no_username: Option<bool>,
    /// No common password.
    #[serde(default)]
    pub no_common: Option<bool>,
    /// Upper and lower case.
    #[serde(default)]
    pub mixed_case: Option<bool>,
    /// A digit.
    #[serde(default)]
    pub digit: Option<bool>,
    /// A special character.
    #[serde(default)]
    pub special: Option<bool>,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// Expiry of passwords. Without a field, that part stays.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ExpiryParam {
    /// Whether passwords expire.
    #[serde(default)]
    pub enable: Option<bool>,
    /// Days a password is valid, 1 to 999.
    #[serde(default)]
    pub max_days: Option<i64>,
    /// Days of warning, 0 to 99.
    #[serde(default)]
    pub warn_days: Option<i64>,
    /// Force the change at expiry.
    #[serde(default)]
    pub force_change: Option<bool>,
    /// Accounts whose password does not expire; replaces the list.
    #[serde(default)]
    pub exempt: Option<Vec<String>>,
    /// Go on although accounts are locked out until they change their
    /// password. Ask the user first.
    #[serde(default)]
    pub force: bool,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// One account.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AccountRefParam {
    /// Login name or uid.
    pub account: String,
    /// Switching off the second factor: go on, the account then has none.
    /// Ask the user first.
    #[serde(default)]
    pub force: bool,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// An avatar.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AvatarParam {
    /// Login name or uid.
    pub account: String,
    /// One of the offered avatars, like `05.png`.
    #[serde(default)]
    pub builtin: Option<String>,
    /// Absolute path of a picture on this machine, under 8 MB.
    #[serde(default)]
    pub file: Option<String>,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// An import of accounts.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ImportParam {
    /// Absolute path of the `.xls` or `.xlsx` file on this machine.
    pub file: String,
    /// Create the accounts. Without this the rows are only read.
    #[serde(default)]
    pub commit: bool,
    /// The accounts have to change their password at the first login.
    #[serde(default)]
    pub change_password: bool,
    /// Go on although this creates accounts whose passwords are in the file.
    /// Ask the user first.
    #[serde(default)]
    pub force: bool,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// Invitations.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct InviteParam {
    /// `status`, `enable`, `disable`, `requests`, `processed`, `approve` or
    /// `reject`.
    pub action: String,
    /// The application id, for `approve` and `reject`.
    #[serde(default)]
    pub id: Option<i64>,
    /// Approving: `users` (default) or `admin`.
    #[serde(default)]
    pub role: Option<String>,
    /// Approving: groups to join.
    #[serde(default)]
    pub groups: Vec<String>,
    /// Approving: a description.
    #[serde(default)]
    pub description: Option<String>,
    /// Approving: give no home folder.
    #[serde(default)]
    pub no_home_dir: bool,
    /// Go on for `enable` (anyone with the link can apply) and for approving
    /// an administrator. Ask the user first.
    #[serde(default)]
    pub force: bool,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

pub async fn rules(c: &UgosClient, p: &RulesParam) -> String {
    let change = RulesChange {
        min_length: p.min_length,
        no_username: p.no_username,
        no_common: p.no_common,
        mixed_case: p.mixed_case,
        digit: p.digit,
        special: p.special,
    };
    if change == RulesChange::default() {
        return json(policy::password_rules(c).await);
    }
    json(policy::set_password_rules(c, &change).await)
}

pub async fn expiry(c: &UgosClient, p: &ExpiryParam) -> String {
    let change = ExpiryChange {
        enable: p.enable,
        max_days: p.max_days,
        warn_days: p.warn_days,
        force_change: p.force_change,
        exempt: p.exempt.clone(),
    };
    if change == ExpiryChange::default() {
        return json(policy::password_expiry(c).await);
    }
    json(policy::set_password_expiry(c, &change, p.force).await)
}

pub async fn password_changed(c: &UgosClient, p: &AccountRefParam) -> String {
    let run = async { policy::last_password_change(c, c.user_find(&p.account).await?.uid).await };
    json(run.await)
}

pub async fn otp_disable(c: &UgosClient, p: &AccountRefParam) -> String {
    let run = async { policy::disable_otp(c, c.user_find(&p.account).await?.uid, p.force).await };
    done(run.await, "The second factor is off")
}

pub async fn avatars(c: &UgosClient) -> String {
    json(policy::avatars(c).await)
}

pub async fn avatar(c: &UgosClient, p: &AvatarParam) -> String {
    let run = async {
        let which = match (&p.builtin, &p.file) {
            (Some(name), None) => Avatar::Builtin(name.clone()),
            (None, Some(path)) => Avatar::File(PathBuf::from(path)),
            _ => return Err(refused("give builtin or file")),
        };
        policy::set_avatar(c, c.user_find(&p.account).await?.uid, &which)
            .await
            .map(|_| ())
    };
    done(run.await, "The avatar is set")
}

pub async fn import(c: &UgosClient, p: &ImportParam) -> String {
    let run = async {
        let mut rows = policy::read_import(c, Path::new(&p.file)).await?;
        if p.commit {
            policy::commit_import(c, &rows, p.change_password, p.force).await?;
        }
        // The passwords are in the file, not in the answer.
        for row in &mut rows {
            row.password.clear();
        }
        Ok(rows)
    };
    json(run.await)
}

pub async fn invite(c: &UgosClient, p: &InviteParam) -> String {
    let run = async {
        let me = c.current_user().await?.uid;
        let id = || p.id.ok_or_else(|| refused("name the application: id"));
        match p.action.as_str() {
            "status" => Ok(serde_json::json!({
                "code": policy::invite_code(c, me).await?,
                "addresses": policy::invite_link_base(c).await?,
            })),
            "enable" => {
                Ok(serde_json::json!({ "code": policy::set_invites(c, me, true, p.force).await? }))
            }
            "disable" => {
                Ok(serde_json::json!({ "code": policy::set_invites(c, me, false, false).await? }))
            }
            "requests" => Ok(serde_json::to_value(policy::invitations(c, false).await?)?),
            "processed" => Ok(serde_json::to_value(policy::invitations(c, true).await?)?),
            "approve" => {
                let how = Approval {
                    role: p.role.clone().unwrap_or_else(|| "users".into()),
                    groups: p.groups.clone(),
                    description: p.description.clone().unwrap_or_default(),
                    home_dir: !p.no_home_dir,
                };
                policy::approve(c, id()?, &how, p.force).await?;
                Ok(serde_json::json!({ "approved": id()? }))
            }
            "reject" => {
                policy::reject(c, id()?).await?;
                Ok(serde_json::json!({ "rejected": id()? }))
            }
            _ => Err(refused(
                "the action is status, enable, disable, requests, processed, approve or reject",
            )),
        }
    };
    json(run.await)
}
