//! Shared folders, user accounts and groups: the logic behind their tools.
//!
//! The tools that change something go through `ugos_client::ops`, as
//! `ugos-cli` does: the same checks, the same refusals, and a read-back that
//! catches UGOS answering success to a request it did not carry out.
//!
//! The tools that delete take `force`, as `--force` does on the command line.
//! A refusal that `force` overrides says what would be lost, and the model is
//! told to ask the user before it repeats the call with `force`. Whether a
//! model may call a tool at all is for the MCP client to gate.

use rmcp::schemars;
use serde::Serialize;
use ugos_client::UgosClient;
use ugos_client::api::shares::SharesApi;
use ugos_client::api::syslog::SysLogApi;
use ugos_client::api::users::UsersApi;
use ugos_client::error::UgosError;
use ugos_client::ops::shares::{
    SettingsChange, ShareRequest, create_share, delete_share, parse_quota, set_share_acl,
    share_settings, update_share,
};
use ugos_client::ops::users::{Expiry, UserChanges, delete_user, parse_date, today, update_user};
use ugos_client::types::users::{ROLE_ADMIN, ROLE_USER};

// ── Parameter types ─────────────────────────────────────────────────

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ShareParam {
    /// Share name or numeric id.
    pub share: String,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
// One field per setting of the web UI; an enum would only restate them.
#[allow(clippy::struct_excessive_bools)]
pub struct ShareCreateParam {
    /// Share name. Not `home`, `homes` or `personal_folder`, no slash, at most 80 characters.
    pub name: String,
    /// Volume name or path, e.g. `/volume1`. Optional when the NAS has one volume.
    #[serde(default)]
    pub volume: Option<String>,
    /// Quota such as `500m`, `100g` or `2t`. No quota when left out.
    #[serde(default)]
    pub quota: Option<String>,
    /// Hide the share from network browsing.
    #[serde(default)]
    pub hide_in_network: bool,
    /// Show the share to users who cannot open it (default: hidden from them).
    #[serde(default)]
    pub show_unreadable: bool,
    /// Turn the recycle bin off.
    #[serde(default)]
    pub no_recycle: bool,
    /// Let everyone empty the recycle bin, not only administrators.
    #[serde(default)]
    pub recycle_for_everyone: bool,
    /// Keep data checksums.
    #[serde(default)]
    pub data_check: bool,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ShareUpdateParam {
    /// Share name or numeric id.
    pub share: String,
    /// New quota such as `500m`, `100g` or `2t`.
    #[serde(default)]
    pub quota: Option<String>,
    /// Lift the quota.
    #[serde(default)]
    pub no_quota: bool,
    /// Hide the share from network browsing (`true`) or show it (`false`).
    #[serde(default)]
    pub hide_in_network: Option<bool>,
    /// Hide the share from users who cannot open it (`true`) or show it (`false`).
    #[serde(default)]
    pub hide_unreadable: Option<bool>,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ShareAclSetParam {
    /// Share name or numeric id.
    pub share: String,
    /// Rules like `user:anna=rw` or `group:family=read`. Access is `none`,
    /// `read`, `rw` or `inherit`. Names must be in the share's access list.
    pub rules: Vec<String>,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ShareDeleteParam {
    /// Share name or numeric id.
    pub share: String,
    /// Delete although the share holds files, which go with it.
    #[serde(default)]
    pub force: bool,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AccountDeleteParam {
    /// Login name or numeric uid.
    pub account: String,
    /// Delete although the account has snapshots or files in its home folder,
    /// which go with it.
    #[serde(default)]
    pub force: bool,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AccountParam {
    /// Login name or numeric uid.
    pub account: String,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
// One field per setting of the web UI; an enum would only restate them.
#[allow(clippy::struct_excessive_bools)]
pub struct UserUpdateParam {
    /// Login name or numeric uid.
    pub account: String,
    /// New email address. The stored one is kept when left out.
    #[serde(default)]
    pub email: Option<String>,
    /// New description.
    #[serde(default)]
    pub description: Option<String>,
    /// Groups to join.
    #[serde(default)]
    pub add_groups: Vec<String>,
    /// Groups to leave. The built-in groups cannot be left.
    #[serde(default)]
    pub remove_groups: Vec<String>,
    /// Give (`true`) or take away (`false`) the home folder.
    #[serde(default)]
    pub home_dir: Option<bool>,
    /// Forbid (`true`) or allow (`false`) the account to change its own password.
    #[serde(default)]
    pub deny_password_change: Option<bool>,
    /// Disable the account.
    #[serde(default)]
    pub disable: bool,
    /// Enable a disabled account, or lift an expiry.
    #[serde(default)]
    pub enable: bool,
    /// Let the account expire on a date, `YYYY-MM-DD`.
    #[serde(default)]
    pub expires: Option<String>,
    /// A new role, `admin` or `user`. Giving or taking away administrator
    /// rights changes who can manage the NAS: ask the user first.
    #[serde(default)]
    pub role: Option<String>,
    /// A new password for the account. The tool takes it as a parameter, so
    /// gate or switch off the tool where a model should not handle passwords.
    #[serde(default)]
    pub password: Option<String>,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

// ── Output ──────────────────────────────────────────────────────────

/// Pretty JSON, or the error as the tools word it.
fn json<T: Serialize>(result: Result<T, UgosError>) -> String {
    match result {
        Ok(value) => serde_json::to_string_pretty(&value).unwrap_or_default(),
        Err(e) => failure(&e),
    }
}

/// An error as a tool answers it. A refusal that `ugos-cli --force` could
/// override is not overridden here, and the message says so.
pub fn failure(error: &UgosError) -> String {
    match error {
        UgosError::NeedsForce(message) => format!(
            "refused: {message}. Ask the user; repeat the call with force set only if they agree."
        ),
        other => format!("error: {other}"),
    }
}

// ── Reading ─────────────────────────────────────────────────────────

pub async fn share_list(client: &UgosClient) -> String {
    json(client.share_list().await)
}

pub async fn share_show(client: &UgosClient, share: &str) -> String {
    json(share_settings(client, share).await)
}

pub async fn share_acl_show(client: &UgosClient, share: &str) -> String {
    let acl = async {
        let found = client.share_find(share).await?;
        client.share_acl(&found.path).await
    };
    json(acl.await)
}

pub async fn smb_status(client: &UgosClient) -> String {
    let status = async {
        let config = client.smb_config().await?;
        let advanced = client.smb_advanced().await?;
        Ok(serde_json::json!({ "config": config, "advanced": advanced }))
    };
    json(status.await)
}

pub async fn user_list(client: &UgosClient) -> String {
    json(client.users().await)
}

pub async fn user_show(client: &UgosClient, account: &str) -> String {
    json(client.user_find(account).await)
}

pub async fn group_list(client: &UgosClient) -> String {
    json(client.group_list().await)
}

// ── Changing shares ─────────────────────────────────────────────────

pub async fn share_create(client: &UgosClient, p: &ShareCreateParam) -> String {
    let request = ShareRequest {
        name: p.name.clone(),
        volume: p.volume.clone(),
        quota: p.quota.clone(),
        hide_in_network: p.hide_in_network,
        show_unreadable: p.show_unreadable,
        no_recycle: p.no_recycle,
        recycle_for_everyone: p.recycle_for_everyone,
        data_check: p.data_check,
    };
    json(create_share(client, &request).await)
}

pub async fn share_update(client: &UgosClient, p: &ShareUpdateParam) -> String {
    let quota = match (&p.quota, p.no_quota) {
        (Some(_), true) => return "error: give a quota or no_quota, not both".into(),
        (Some(text), false) => match parse_quota(text) {
            Ok(quota) => Some(quota),
            Err(e) => return failure(&e),
        },
        (None, true) => Some((0, 1)),
        (None, false) => None,
    };
    let change = SettingsChange {
        quota,
        network_places_hide: p.hide_in_network,
        no_permission_hide: p.hide_unreadable,
    };
    json(update_share(client, &p.share, &change).await)
}

pub async fn share_acl_set(client: &UgosClient, p: &ShareAclSetParam) -> String {
    json(
        set_share_acl(client, &p.share, &p.rules)
            .await
            .map(|(_, acl)| acl),
    )
}

pub async fn share_delete(client: &UgosClient, p: &ShareDeleteParam) -> String {
    match delete_share(client, &p.share, p.force).await {
        Ok(found) => format!("Deleted share {}", found.name),
        Err(e) => failure(&e),
    }
}

// ── Changing accounts ───────────────────────────────────────────────

pub async fn user_update(client: &UgosClient, p: &UserUpdateParam) -> String {
    let expiry = match (&p.expires, p.disable, p.enable) {
        (Some(_), true, _) | (Some(_), _, true) | (None, true, true) => {
            return "error: give one of expires, disable and enable".into();
        }
        (Some(text), false, false) => match parse_date(text, today()) {
            Ok(expiry) => Some(expiry),
            Err(e) => return failure(&e),
        },
        (None, true, false) => Some(Expiry::DISABLED),
        (None, false, true) => Some(Expiry::NEVER),
        (None, false, false) => None,
    };
    let change = UserChanges {
        email: p.email.clone(),
        description: p.description.clone(),
        role: match p.role.as_deref() {
            None => None,
            Some("admin") => Some(ROLE_ADMIN.to_owned()),
            Some("user") => Some(ROLE_USER.to_owned()),
            Some(_) => return "error: the role is admin or user".into(),
        },
        add_groups: p.add_groups.clone(),
        remove_groups: p.remove_groups.clone(),
        home_dir: p.home_dir,
        deny_password_change: p.deny_password_change,
        expiry,
        password: p.password.clone().filter(|v| !v.is_empty()),
    };
    json(update_user(client, &p.account, &change).await)
}

pub async fn user_delete(client: &UgosClient, p: &AccountDeleteParam) -> String {
    match delete_user(client, &p.account, p.force).await {
        Ok(found) => format!("Deleted account {}", found.username),
        Err(e) => failure(&e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_refusal_that_force_would_override_tells_the_model_to_ask() {
        let said = failure(&UgosError::NeedsForce("share 'x' holds 3 entries".into()));
        assert!(
            said.starts_with("refused: share 'x' holds 3 entries"),
            "{said}"
        );
        assert!(said.contains("force"), "{said}");
        assert!(said.contains("Ask the user"), "{said}");
    }

    #[test]
    fn other_errors_are_plain() {
        assert_eq!(failure(&UgosError::Refused("nope".into())), "error: nope");
    }
}
