//! Guarded operations on shared folders.
//!
//! What `ugos-cli share` and the MCP server both do: each checks its input
//! the way the web UI does, and reads the result back, since UGOS answers
//! success to a request it did not understand and changes nothing. The
//! callers only parse their own input and print.

use crate::api::files::FilesApi;
use crate::api::shares::SharesApi;
use crate::client::UgosClient;
use crate::error::{Result, UgosError};
use crate::types::files::Volume;
use crate::types::shares::{
    Access, AclChange, AclUser, CreatedShare, NewShare, PathAcl, PrincipalKind, QUOTA_UNITS, Share,
    ShareInfo,
};

/// Longest share name the web UI accepts.
const NAME_LIMIT: usize = 80;

/// Names UGOS keeps for its own folders.
const RESERVED_NAMES: [&str; 3] = ["personal_folder", "homes", "home"];

/// What creating a share was asked for.
// One field per switch of the command line; an enum would only restate them.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Default)]
pub struct ShareRequest {
    /// Share name.
    pub name: String,
    /// Volume name or path; the only volume when `None`.
    pub volume: Option<String>,
    /// Quota such as `500m`, `100g` or `2t`.
    pub quota: Option<String>,
    /// Hide the share from network browsing.
    pub hide_in_network: bool,
    /// Show the share to users who cannot open it.
    pub show_unreadable: bool,
    /// Turn the recycle bin off.
    pub no_recycle: bool,
    /// Let everyone empty the recycle bin, not only administrators.
    pub recycle_for_everyone: bool,
    /// Keep data checksums.
    pub data_check: bool,
}

/// The request that creating a share sends.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] for a name or quota the web UI would
/// refuse, or a volume that cannot be told.
pub fn build_new_share(volumes: &[Volume], request: &ShareRequest) -> Result<NewShare> {
    check_name(&request.name)?;
    let mut new = NewShare::new(
        pick_volume(volumes, request.volume.as_deref())?,
        request.name.trim(),
    );
    if let Some(text) = &request.quota {
        (new.quota, new.unit) = parse_quota(text)?;
    }
    new.network_places_hide = request.hide_in_network;
    new.no_permission_hide = !request.show_unreadable;
    new.enable_recycle = !request.no_recycle;
    new.allow_only_admin_recycle = !request.recycle_for_everyone;
    new.data_check = request.data_check;
    Ok(new)
}

/// Create a share, unless one of that name exists.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] for a bad request or a name that is taken,
/// and the appropriate [`UgosError`] on failure.
pub async fn create_share(client: &UgosClient, request: &ShareRequest) -> Result<CreatedShare> {
    let volumes = client.fs_volumes().await?;
    let new = build_new_share(&volumes, request)?;
    let existing = client.share_list().await?;
    if let Some(clash) = existing
        .iter()
        .find(|share| share.name.eq_ignore_ascii_case(&new.name))
    {
        refuse!("a share named '{}' already exists", clash.name);
    }
    client.share_create(&new).await
}

/// The settings of a share, found by name or id.
///
/// # Errors
///
/// Returns [`UgosError::NotFound`] when no share matches.
pub async fn share_settings(client: &UgosClient, name_or_id: &str) -> Result<ShareInfo> {
    let found = client.share_find(name_or_id).await?;
    client.share_info(found.id).await
}

/// Change the settings of a share and check that UGOS kept the change.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] when there is nothing to change or UGOS
/// did not apply it.
pub async fn update_share(
    client: &UgosClient,
    name_or_id: &str,
    change: &SettingsChange,
) -> Result<ShareInfo> {
    if change.is_empty() {
        refuse!("nothing to change: give a quota or a visibility setting");
    }
    let before = share_settings(client, name_or_id).await?;
    let mut after = before.clone();
    change.apply(&mut after);
    client.share_update(&after).await?;

    // UGOS answers success to a request it does not understand and changes
    // nothing, so look at what it holds now.
    let now = client.share_info(before.id).await?;
    if let Some(problem) = change.mismatch(&now) {
        refuse!("UGOS accepted the request but {problem}");
    }
    Ok(now)
}

/// Delete a share and everything in it. A share that holds files is not
/// deleted unless `force` says so.
///
/// # Errors
///
/// Returns [`UgosError::NeedsForce`] for a share that holds files, and
/// [`UgosError::Refused`] when UGOS answered success and the share is still
/// there.
pub async fn delete_share(client: &UgosClient, name_or_id: &str, force: bool) -> Result<Share> {
    let found = client.share_find(name_or_id).await?;
    if !force {
        match client.fs_list(&found.path).await.map(|files| files.len()) {
            Ok(0) => {}
            Ok(count) => {
                return Err(UgosError::NeedsForce(format!(
                    "share '{}' is not empty ({count} {}), and deleting it deletes the content",
                    found.name,
                    if count == 1 { "entry" } else { "entries" }
                )));
            }
            Err(e) => {
                return Err(UgosError::NeedsForce(format!(
                    "cannot tell whether share '{}' is empty ({e})",
                    found.name
                )));
            }
        }
    }
    client.share_delete(&found.path).await?;
    let left = client.share_list().await?;
    if left.iter().any(|remaining| remaining.id == found.id) {
        refuse!(
            "UGOS accepted the request but share '{}' is still there",
            found.name
        );
    }
    Ok(found)
}

/// Change access rules, written `user:NAME=ACCESS` or `group:NAME=ACCESS`,
/// and check that UGOS applied each. It keeps the administrators group at
/// read and write whatever is asked, and says nothing.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] for a malformed rule, a name the list does
/// not show, or a rule UGOS did not apply.
pub async fn set_share_acl(
    client: &UgosClient,
    name_or_id: &str,
    rules: &[String],
) -> Result<(Share, PathAcl)> {
    let found = client.share_find(name_or_id).await?;
    let rules = rules
        .iter()
        .map(|text| parse_rule(text))
        .collect::<Result<Vec<_>>>()?;
    let before = client.share_acl(&found.path).await?;
    let changes = plan_changes(&before, &rules)?;
    client.share_acl_set(&found.path, &changes).await?;
    let after = client.share_acl(&found.path).await?;
    let refused = unapplied(&after, &rules);
    if !refused.is_empty() {
        refuse!("UGOS did not apply: {}", refused.join("; "));
    }
    Ok((found, after))
}

/// A share name the web UI would accept.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] for a name the web UI would refuse.
pub fn check_name(name: &str) -> Result<()> {
    let name = name.trim();
    if name.is_empty() {
        refuse!("the share name is empty");
    }
    if name.chars().count() > NAME_LIMIT {
        refuse!("the share name is longer than {NAME_LIMIT} characters");
    }
    if name.contains('/') || name.contains('\\') {
        refuse!("the share name must not contain a slash");
    }
    if name.starts_with('@') {
        refuse!("the share name must not start with '@'");
    }
    if RESERVED_NAMES.contains(&name.to_ascii_lowercase().as_str()) {
        refuse!("'{name}' is a name UGOS keeps for its own folders");
    }
    Ok(())
}

/// The volume to create a share on: the one asked for, or the only one.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] for an unknown volume, or when none is
/// named and the NAS has none or several.
pub fn pick_volume(volumes: &[Volume], wanted: Option<&str>) -> Result<String> {
    let names = || {
        volumes
            .iter()
            .map(|volume| volume.path.clone())
            .collect::<Vec<_>>()
            .join(", ")
    };
    match wanted {
        Some(text) => {
            let wanted = text.trim_end_matches('/');
            volumes
                .iter()
                .find(|volume| volume.path == wanted || volume.name == wanted)
                .map(|volume| volume.path.clone())
                .ok_or_else(|| {
                    UgosError::Refused(format!("no volume '{text}'; the NAS has: {}", names()))
                })
        }
        None => match volumes {
            [only] => Ok(only.path.clone()),
            [] => refuse!("the NAS reports no volume"),
            _ => refuse!("the NAS has several volumes ({}); name one", names()),
        },
    }
}

/// A quota such as `500m`, `100g` or `2t`: the value and the unit code.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] for a quota without a unit, with an
/// unknown one, or of 0.
pub fn parse_quota(text: &str) -> Result<(i64, i64)> {
    let lower = text.trim().to_ascii_lowercase();
    let lower = lower.strip_suffix('b').unwrap_or(&lower);
    let split = lower.find(|c: char| !c.is_ascii_digit()).ok_or_else(|| {
        UgosError::Refused(format!(
            "quota '{text}' has no unit; use a number and m, g or t"
        ))
    })?;
    let (digits, unit) = lower.split_at(split);
    let value: i64 = digits.parse().map_err(|_| {
        UgosError::Refused(format!("quota '{text}' does not start with a whole number"))
    })?;
    if value == 0 {
        refuse!("a quota of 0 means no limit; remove the quota instead");
    }
    let code = match unit {
        "m" => 0,
        "g" => 1,
        "t" => 2,
        _ => refuse!(
            "quota '{text}' has the unit '{unit}'; use {}",
            QUOTA_UNITS.map(str::to_ascii_lowercase).join(", ")
        ),
    };
    Ok((value, code))
}

/// What `share update` was asked to change.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct SettingsChange {
    /// New quota and unit code; `(0, _)` lifts the quota.
    pub quota: Option<(i64, i64)>,
    /// Hide the share from network browsing.
    pub network_places_hide: Option<bool>,
    /// Hide the share from users who cannot open it.
    pub no_permission_hide: Option<bool>,
}

impl SettingsChange {
    /// Whether nothing is to change.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.quota.is_none()
            && self.network_places_hide.is_none()
            && self.no_permission_hide.is_none()
    }

    /// Write the change into `info`.
    pub const fn apply(&self, info: &mut ShareInfo) {
        if let Some((quota, unit)) = self.quota {
            info.quota = quota;
            // Without a limit the unit is not shown; keep the one it had.
            if quota > 0 {
                info.unit = unit;
            }
        }
        if let Some(hide) = self.network_places_hide {
            info.network_places_hide = hide;
        }
        if let Some(hide) = self.no_permission_hide {
            info.no_permission_hide = hide;
        }
    }

    /// The first requested setting that `now` does not show, as a sentence.
    #[must_use]
    pub fn mismatch(&self, now: &ShareInfo) -> Option<String> {
        if let Some((quota, unit)) = self.quota
            && (now.quota != quota || (quota > 0 && now.unit != unit))
        {
            return Some("the quota is not what was asked for".into());
        }
        if let Some(hide) = self.network_places_hide
            && now.network_places_hide != hide
        {
            return Some("the network visibility did not change".into());
        }
        if let Some(hide) = self.no_permission_hide
            && now.no_permission_hide != hide
        {
            return Some("the visibility for users without access did not change".into());
        }
        None
    }
}

/// One `share acl set` rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Rule {
    pub(crate) kind: PrincipalKind,
    pub(crate) name: String,
    pub(crate) access: Access,
}

/// Parse `user:NAME=ACCESS` or `group:NAME=ACCESS`.
pub(crate) fn parse_rule(text: &str) -> Result<Rule> {
    let (who, access) = text.split_once('=').ok_or_else(|| {
        UgosError::Refused(format!("rule '{text}' has no '='; write user:NAME=ACCESS"))
    })?;
    let (kind, name) = who.split_once(':').ok_or_else(|| {
        UgosError::Refused(format!(
            "rule '{text}' does not say user or group; write user:NAME=ACCESS"
        ))
    })?;
    let kind = match kind.trim().to_ascii_lowercase().as_str() {
        "user" | "u" => PrincipalKind::User,
        "group" | "g" => PrincipalKind::Group,
        other => refuse!("rule '{text}': '{other}' is not 'user' or 'group'"),
    };
    let name = name.trim();
    if name.is_empty() {
        refuse!("rule '{text}' names nobody");
    }
    let access = Access::parse(access).map_err(|bad| {
        UgosError::Refused(format!(
            "rule '{text}': '{bad}' is not none, read, rw or inherit"
        ))
    })?;
    Ok(Rule {
        kind,
        name: name.to_owned(),
        access,
    })
}

/// The users of an access list, whatever group lists them.
fn users_of(acl: &PathAcl) -> impl Iterator<Item = &AclUser> {
    acl.acls.iter().flat_map(|group| &group.users)
}

/// Whether the rule's principal is an administrator, or an error that names
/// the principals the list does show.
fn resolve_admin(acl: &PathAcl, rule: &Rule) -> Result<bool> {
    match rule.kind {
        PrincipalKind::Group => {
            if acl.acls.iter().any(|group| group.name == rule.name) {
                return Ok(false);
            }
            refuse!(
                "no group '{}' in this share's access list; known groups: {}",
                rule.name,
                known(acl.acls.iter().map(|group| group.name.as_str()))
            )
        }
        PrincipalKind::User => match users_of(acl).find(|user| user.username == rule.name) {
            Some(user) => Ok(user.is_admin),
            None => refuse!(
                "no user '{}' in this share's access list; known users: {}",
                rule.name,
                known(users_of(acl).map(|user| user.username.as_str()))
            ),
        },
    }
}

/// Turn rules into the changes to send, checking each name against the list.
pub(crate) fn plan_changes(acl: &PathAcl, rules: &[Rule]) -> Result<Vec<AclChange>> {
    rules
        .iter()
        .map(|rule| {
            Ok(AclChange {
                kind: rule.kind,
                name: rule.name.clone(),
                permission_mask: rule.access.mask(),
                is_admin: resolve_admin(acl, rule)?,
            })
        })
        .collect()
}

fn known<'a>(names: impl Iterator<Item = &'a str>) -> String {
    let mut seen: Vec<&str> = Vec::new();
    for name in names {
        if !seen.contains(&name) {
            seen.push(name);
        }
    }
    if seen.is_empty() {
        "none".into()
    } else {
        seen.join(", ")
    }
}

/// What the list holds for the rule's principal, as the mask set on it.
fn mask_set(acl: &PathAcl, rule: &Rule) -> Option<i64> {
    match rule.kind {
        PrincipalKind::Group => acl
            .acls
            .iter()
            .find(|group| group.name == rule.name)
            .map(|group| group.select_permissions),
        PrincipalKind::User => users_of(acl)
            .find(|user| user.username == rule.name)
            .map(|user| user.select_permissions),
    }
}

/// A sentence if the list does not show the rule as asked.
fn unapplied_rule(acl: &PathAcl, rule: &Rule) -> Option<String> {
    let kind = match rule.kind {
        PrincipalKind::User => "user",
        PrincipalKind::Group => "group",
    };
    match mask_set(acl, rule).map(Access::from_mask) {
        Some(now) if now == rule.access => None,
        Some(now) => Some(format!(
            "{kind} '{}' stays at {} instead of {}",
            rule.name,
            now.label(),
            rule.access.label()
        )),
        None => Some(format!("{kind} '{}' is gone from the list", rule.name)),
    }
}

/// The rules that the access list does not show after the change.
pub(crate) fn unapplied(acl: &PathAcl, rules: &[Rule]) -> Vec<String> {
    rules
        .iter()
        .filter_map(|rule| unapplied_rule(acl, rule))
        .collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests;
