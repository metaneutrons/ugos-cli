//! `share` and `smb`: shared folders and the SMB file service.

use std::io::Write;

use anyhow::{Context, Result, bail};
use ugos_client::UgosClient;
use ugos_client::api::files::FilesApi;
use ugos_client::api::shares::SharesApi;
use ugos_client::types::files::Volume;
use ugos_client::types::shares::{
    Access, AclChange, AclUser, NewShare, PathAcl, PrincipalKind, QUOTA_UNITS, Share, ShareInfo,
};

use crate::cli::{OutputFormat, ShareAclAction, ShareAction, SmbAction};
use crate::output;

/// Longest share name the web UI accepts.
const NAME_LIMIT: usize = 80;

/// Names UGOS keeps for its own folders.
const RESERVED_NAMES: [&str; 3] = ["personal_folder", "homes", "home"];

/// Run a `share` subcommand.
pub async fn share(
    client: &UgosClient,
    action: &ShareAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    match action {
        ShareAction::List => list(client, fmt, w).await,
        ShareAction::Show { share } => show(client, share, fmt, w).await,
        ShareAction::Create {
            name,
            volume,
            quota,
            hide_in_network,
            show_unreadable,
            no_recycle,
            recycle_for_everyone,
            data_check,
            dry_run,
        } => {
            let options = CreateOptions {
                name,
                volume: volume.as_deref(),
                quota: quota.as_deref(),
                hide_in_network: *hide_in_network,
                show_unreadable: *show_unreadable,
                no_recycle: *no_recycle,
                recycle_for_everyone: *recycle_for_everyone,
                data_check: *data_check,
            };
            create(client, &options, *dry_run, fmt, w).await
        }
        ShareAction::Update {
            share,
            quota,
            no_quota,
            hide_in_network,
            show_in_network,
            hide_unreadable,
            show_unreadable,
        } => {
            let change = SettingsChange {
                quota: quota_change(quota.as_deref(), *no_quota)?,
                network_places_hide: flag_pair(*hide_in_network, *show_in_network),
                no_permission_hide: flag_pair(*hide_unreadable, *show_unreadable),
            };
            update(client, share, &change, fmt, w).await
        }
        ShareAction::Delete { share, force } => delete(client, share, *force, fmt, w).await,
        ShareAction::Acl { action } => acl(client, action, fmt, w).await,
    }
}

async fn list(client: &UgosClient, fmt: OutputFormat, w: &mut impl Write) -> Result<()> {
    let shares = client.share_list().await?;
    match fmt {
        OutputFormat::Table => {
            let rows: Vec<output::ShareRow> = shares.iter().map(Into::into).collect();
            output::print_list(w, &rows, fmt)
        }
        OutputFormat::Json => output::print_json(w, &shares),
    }
}

async fn show(
    client: &UgosClient,
    share: &str,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    let info = settings_of(client, share).await?;
    match fmt {
        OutputFormat::Table => output::print_list(w, &output::share_info_rows(&info), fmt),
        OutputFormat::Json => output::print_json(w, &info),
    }
}

/// The quota change that `--quota` and `--no-quota` ask for, if any.
fn quota_change(quota: Option<&str>, no_quota: bool) -> Result<Option<(i64, i64)>> {
    match (quota, no_quota) {
        (Some(text), _) => parse_quota(text).map(Some),
        (None, true) => Ok(Some((0, 1))),
        (None, false) => Ok(None),
    }
}

/// What `share create` was asked for.
// One field per switch of the command line; an enum would only restate them.
#[allow(clippy::struct_excessive_bools)]
struct CreateOptions<'a> {
    name: &'a str,
    volume: Option<&'a str>,
    quota: Option<&'a str>,
    hide_in_network: bool,
    show_unreadable: bool,
    no_recycle: bool,
    recycle_for_everyone: bool,
    data_check: bool,
}

/// The request `share create` sends.
fn build_new_share(volumes: &[Volume], options: &CreateOptions<'_>) -> Result<NewShare> {
    check_name(options.name)?;
    let mut new = NewShare::new(pick_volume(volumes, options.volume)?, options.name.trim());
    if let Some(text) = options.quota {
        (new.quota, new.unit) = parse_quota(text)?;
    }
    new.network_places_hide = options.hide_in_network;
    new.no_permission_hide = !options.show_unreadable;
    new.enable_recycle = !options.no_recycle;
    new.allow_only_admin_recycle = !options.recycle_for_everyone;
    new.data_check = options.data_check;
    Ok(new)
}

async fn create(
    client: &UgosClient,
    options: &CreateOptions<'_>,
    dry_run: bool,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    let volumes = client.fs_volumes().await?;
    let new = build_new_share(&volumes, options)?;
    if dry_run {
        return output::print_json(w, &new);
    }
    let existing = client.share_list().await?;
    if let Some(clash) = existing
        .iter()
        .find(|share| share.name.eq_ignore_ascii_case(&new.name))
    {
        bail!("a share named '{}' already exists", clash.name);
    }
    let created = client.share_create(&new).await?;
    match fmt {
        OutputFormat::Table => {
            writeln!(w, "Created share '{}' ({})", new.name, created.path)?;
        }
        OutputFormat::Json => output::print_json(w, &created)?,
    }
    Ok(())
}

async fn update(
    client: &UgosClient,
    share: &str,
    change: &SettingsChange,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    if change.is_empty() {
        bail!("nothing to change: give --quota, --no-quota or a visibility flag");
    }
    let before = settings_of(client, share).await?;
    let mut after = before.clone();
    change.apply(&mut after);
    client.share_update(&after).await?;

    // UGOS answers success to a request it does not understand and changes
    // nothing, so look at what it holds now.
    let now = client.share_info(before.id).await?;
    if let Some(problem) = change.mismatch(&now) {
        bail!("UGOS accepted the request but {problem}");
    }
    match fmt {
        OutputFormat::Table => {
            writeln!(w, "Updated share '{}'", now.name)?;
            output::print_list(w, &output::share_info_rows(&now), fmt)?;
        }
        OutputFormat::Json => output::print_json(w, &now)?,
    }
    Ok(())
}

async fn delete(
    client: &UgosClient,
    share: &str,
    force: bool,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    let found = client.share_find(share).await?;
    if !force {
        match client.fs_list(&found.path).await.map(|files| files.len()) {
            Ok(0) => {}
            Ok(count) => bail!(
                "share '{}' holds {count} entries; pass --force to delete it anyway",
                found.name
            ),
            Err(e) => bail!(
                "cannot tell whether share '{}' is empty ({e}); pass --force to delete it anyway",
                found.name
            ),
        }
    }
    client.share_delete(&found.path).await?;
    let left = client.share_list().await?;
    if left.iter().any(|remaining| remaining.id == found.id) {
        bail!(
            "UGOS accepted the request but share '{}' is still there",
            found.name
        );
    }
    output::print_success(w, &format!("Deleted share '{}'", found.name), fmt)
}

/// Run a `share acl` subcommand.
async fn acl(
    client: &UgosClient,
    action: &ShareAclAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    match action {
        ShareAclAction::Show { share } => acl_show(client, share, fmt, w).await,
        ShareAclAction::Set { share, rules } => acl_set(client, share, rules, fmt, w).await,
    }
}

async fn acl_show(
    client: &UgosClient,
    share: &str,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    let found = client.share_find(share).await?;
    let acl = client.share_acl(&found.path).await?;
    match fmt {
        OutputFormat::Table => output::print_list(w, &output::acl_rows(&acl), fmt),
        OutputFormat::Json => output::print_json(w, &acl),
    }
}

async fn acl_set(
    client: &UgosClient,
    share: &str,
    rules: &[String],
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    let found = client.share_find(share).await?;
    let rules = rules
        .iter()
        .map(|text| parse_rule(text))
        .collect::<Result<Vec<_>>>()?;
    let before = client.share_acl(&found.path).await?;
    let changes = plan_changes(&before, &rules)?;
    client.share_acl_set(&found.path, &changes).await?;

    // The administrators group is kept at read and write whatever is
    // asked, and UGOS says nothing, so read the result back.
    let after = client.share_acl(&found.path).await?;
    let refused = unapplied(&after, &rules);
    if !refused.is_empty() {
        bail!("UGOS did not apply: {}", refused.join("; "));
    }
    match fmt {
        OutputFormat::Table => {
            writeln!(w, "Updated access on '{}'", found.name)?;
            output::print_list(w, &output::acl_rows(&after), fmt)
        }
        OutputFormat::Json => output::print_json(w, &after),
    }
}

/// Run an `smb` subcommand.
pub async fn smb(
    client: &UgosClient,
    action: &SmbAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    match action {
        SmbAction::Status => {
            let config = client.smb_config().await?;
            let advanced = client.smb_advanced().await?;
            match fmt {
                OutputFormat::Table => {
                    output::print_list(w, &output::smb_rows(&config, &advanced), fmt)?;
                }
                OutputFormat::Json => {
                    output::print_json(
                        w,
                        &serde_json::json!({ "config": config, "advanced": advanced }),
                    )?;
                }
            }
        }
    }
    Ok(())
}

/// The settings of a share, found by name or id.
async fn settings_of(client: &UgosClient, name_or_id: &str) -> Result<ShareInfo> {
    let found: Share = client.share_find(name_or_id).await?;
    client
        .share_info(found.id)
        .await
        .with_context(|| format!("reading the settings of '{}'", found.name))
}

// ── Pure helpers ────────────────────────────────────────────────────

/// A share name the web UI would accept.
fn check_name(name: &str) -> Result<()> {
    let name = name.trim();
    if name.is_empty() {
        bail!("the share name is empty");
    }
    if name.chars().count() > NAME_LIMIT {
        bail!("the share name is longer than {NAME_LIMIT} characters");
    }
    if name.contains('/') || name.contains('\\') {
        bail!("the share name must not contain a slash");
    }
    if name.starts_with('@') {
        bail!("the share name must not start with '@'");
    }
    if RESERVED_NAMES.contains(&name.to_ascii_lowercase().as_str()) {
        bail!("'{name}' is a name UGOS keeps for its own folders");
    }
    Ok(())
}

/// The volume to create a share on: the one asked for, or the only one.
fn pick_volume(volumes: &[Volume], wanted: Option<&str>) -> Result<String> {
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
                .with_context(|| format!("no volume '{text}'; the NAS has: {}", names()))
        }
        None => match volumes {
            [only] => Ok(only.path.clone()),
            [] => bail!("the NAS reports no volume"),
            _ => bail!(
                "the NAS has several volumes ({}); pick one with --volume",
                names()
            ),
        },
    }
}

/// A quota such as `500m`, `100g` or `2t`: the value and the unit code.
fn parse_quota(text: &str) -> Result<(i64, i64)> {
    let lower = text.trim().to_ascii_lowercase();
    let lower = lower.strip_suffix('b').unwrap_or(&lower);
    let split = lower
        .find(|c: char| !c.is_ascii_digit())
        .with_context(|| format!("quota '{text}' has no unit; use a number and m, g or t"))?;
    let (digits, unit) = lower.split_at(split);
    let value: i64 = digits
        .parse()
        .with_context(|| format!("quota '{text}' does not start with a whole number"))?;
    if value == 0 {
        bail!("a quota of 0 means no limit; use --no-quota to remove one");
    }
    let code = match unit {
        "m" => 0,
        "g" => 1,
        "t" => 2,
        _ => bail!(
            "quota '{text}' has the unit '{unit}'; use {}",
            QUOTA_UNITS.map(str::to_ascii_lowercase).join(", ")
        ),
    };
    Ok((value, code))
}

/// `Some(true)` for the first flag, `Some(false)` for the second, else `None`.
const fn flag_pair(on: bool, off: bool) -> Option<bool> {
    match (on, off) {
        (true, _) => Some(true),
        (false, true) => Some(false),
        (false, false) => None,
    }
}

/// What `share update` was asked to change.
#[derive(Debug, Default, PartialEq, Eq)]
struct SettingsChange {
    /// New quota and unit code.
    quota: Option<(i64, i64)>,
    network_places_hide: Option<bool>,
    no_permission_hide: Option<bool>,
}

impl SettingsChange {
    const fn is_empty(&self) -> bool {
        self.quota.is_none()
            && self.network_places_hide.is_none()
            && self.no_permission_hide.is_none()
    }

    const fn apply(&self, info: &mut ShareInfo) {
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
    fn mismatch(&self, now: &ShareInfo) -> Option<String> {
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
struct Rule {
    kind: PrincipalKind,
    name: String,
    access: Access,
}

/// Parse `user:NAME=ACCESS` or `group:NAME=ACCESS`.
fn parse_rule(text: &str) -> Result<Rule> {
    let (who, access) = text
        .split_once('=')
        .with_context(|| format!("rule '{text}' has no '='; write user:NAME=ACCESS"))?;
    let (kind, name) = who.split_once(':').with_context(|| {
        format!("rule '{text}' does not say user or group; write user:NAME=ACCESS")
    })?;
    let kind = match kind.trim().to_ascii_lowercase().as_str() {
        "user" | "u" => PrincipalKind::User,
        "group" | "g" => PrincipalKind::Group,
        other => bail!("rule '{text}': '{other}' is not 'user' or 'group'"),
    };
    let name = name.trim();
    if name.is_empty() {
        bail!("rule '{text}' names nobody");
    }
    let access = Access::parse(access).map_err(|bad| {
        anyhow::anyhow!("rule '{text}': '{bad}' is not none, read, rw or inherit")
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
            bail!(
                "no group '{}' in this share's access list; known groups: {}",
                rule.name,
                known(acl.acls.iter().map(|group| group.name.as_str()))
            )
        }
        PrincipalKind::User => match users_of(acl).find(|user| user.username == rule.name) {
            Some(user) => Ok(user.is_admin),
            None => bail!(
                "no user '{}' in this share's access list; known users: {}",
                rule.name,
                known(users_of(acl).map(|user| user.username.as_str()))
            ),
        },
    }
}

/// Turn rules into the changes to send, checking each name against the list.
fn plan_changes(acl: &PathAcl, rules: &[Rule]) -> Result<Vec<AclChange>> {
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
fn unapplied(acl: &PathAcl, rules: &[Rule]) -> Vec<String> {
    rules
        .iter()
        .filter_map(|rule| unapplied_rule(acl, rule))
        .collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn volume(name: &str, path: &str) -> Volume {
        Volume {
            name: name.into(),
            path: path.into(),
            ..Volume::default()
        }
    }

    fn acl() -> PathAcl {
        serde_json::from_str(
            r#"{"acls": [
                {"name": "admin", "gid": 10, "permission_mask": 7, "select_permissions": 7,
                 "users": [{"username": "fabian", "uid": 1000, "is_admin": true,
                            "permission_mask": 7, "select_permissions": -1}]},
                {"name": "family", "gid": 101, "permission_mask": 4, "select_permissions": 5,
                 "users": [{"username": "anna", "uid": 1001, "is_admin": false,
                            "permission_mask": 5, "select_permissions": -1}]}
            ], "total_users": 2}"#,
        )
        .unwrap()
    }

    #[test]
    fn names_the_web_ui_would_refuse_are_refused() {
        assert!(check_name("media").is_ok());
        assert!(check_name("  media  ").is_ok());
        assert!(check_name("").is_err());
        assert!(check_name("   ").is_err());
        assert!(check_name(&"x".repeat(81)).is_err());
        assert!(check_name(&"x".repeat(80)).is_ok());
        assert!(check_name("a/b").is_err());
        assert!(check_name("a\\b").is_err());
        assert!(check_name("@recycle").is_err());
        assert!(check_name("Home").is_err());
        assert!(check_name("personal_folder").is_err());
    }

    #[test]
    fn the_only_volume_is_picked_without_being_asked() {
        let one = [volume("Volume 1", "/volume1")];
        assert_eq!(pick_volume(&one, None).unwrap(), "/volume1");
        assert_eq!(pick_volume(&one, Some("/volume1/")).unwrap(), "/volume1");
        assert_eq!(pick_volume(&one, Some("Volume 1")).unwrap(), "/volume1");
    }

    #[test]
    fn several_volumes_need_a_choice_and_an_unknown_one_lists_them() {
        let two = [
            volume("Volume 1", "/volume1"),
            volume("Volume 2", "/volume2"),
        ];
        let err = pick_volume(&two, None).unwrap_err().to_string();
        assert!(err.contains("/volume1, /volume2"), "{err}");
        assert_eq!(pick_volume(&two, Some("/volume2")).unwrap(), "/volume2");
        let err = pick_volume(&two, Some("/volume9")).unwrap_err().to_string();
        assert!(err.contains("/volume1, /volume2"), "{err}");
        assert!(pick_volume(&[], None).is_err());
    }

    #[test]
    fn quotas_are_a_number_and_a_unit() {
        assert_eq!(parse_quota("500m").unwrap(), (500, 0));
        assert_eq!(parse_quota("100g").unwrap(), (100, 1));
        assert_eq!(parse_quota("2T").unwrap(), (2, 2));
        assert!(parse_quota("5 GB").is_err());
        assert_eq!(parse_quota("5GB").unwrap(), (5, 1));
        assert_eq!(parse_quota("5mb").unwrap(), (5, 0));
    }

    #[test]
    fn a_quota_without_a_unit_or_with_nonsense_is_refused() {
        assert!(
            parse_quota("100")
                .unwrap_err()
                .to_string()
                .contains("no unit")
        );
        assert!(parse_quota("g").is_err());
        assert!(parse_quota("1.5g").is_err());
        assert!(parse_quota("-3g").is_err());
        assert!(parse_quota("5k").unwrap_err().to_string().contains("'k'"));
        assert!(
            parse_quota("0g")
                .unwrap_err()
                .to_string()
                .contains("--no-quota")
        );
        assert!(parse_quota("").is_err());
    }

    #[test]
    fn a_flag_pair_says_on_off_or_nothing() {
        assert_eq!(flag_pair(true, false), Some(true));
        assert_eq!(flag_pair(false, true), Some(false));
        assert_eq!(flag_pair(false, false), None);
    }

    #[test]
    fn a_settings_change_applies_and_is_checked() {
        let mut info = ShareInfo {
            quota: 0,
            unit: 2,
            ..ShareInfo::default()
        };
        let change = SettingsChange {
            quota: Some((5, 1)),
            network_places_hide: Some(true),
            no_permission_hide: None,
        };
        assert!(!change.is_empty());
        change.apply(&mut info);
        assert_eq!((info.quota, info.unit), (5, 1));
        assert!(info.network_places_hide);
        assert!(change.mismatch(&info).is_none());

        // A request UGOS ignored shows up as a mismatch.
        let untouched = ShareInfo::default();
        assert!(change.mismatch(&untouched).unwrap().contains("quota"));
    }

    #[test]
    fn removing_a_quota_keeps_the_unit_it_had() {
        let mut info = ShareInfo {
            quota: 5,
            unit: 2,
            ..ShareInfo::default()
        };
        let change = SettingsChange {
            quota: Some((0, 1)),
            ..SettingsChange::default()
        };
        change.apply(&mut info);
        assert_eq!((info.quota, info.unit), (0, 2));
        assert!(change.mismatch(&info).is_none());
    }

    #[test]
    fn an_empty_change_is_empty() {
        assert!(SettingsChange::default().is_empty());
    }

    #[test]
    fn rules_parse_with_their_aliases() {
        assert_eq!(
            parse_rule("user:anna=rw").unwrap(),
            Rule {
                kind: PrincipalKind::User,
                name: "anna".into(),
                access: Access::ReadWrite
            }
        );
        let group = parse_rule("Group: family = READ").unwrap();
        assert_eq!(group.kind, PrincipalKind::Group);
        assert_eq!(group.name, "family");
        assert_eq!(group.access, Access::Read);
        assert_eq!(parse_rule("g:x=inherit").unwrap().access, Access::Inherit);
        assert_eq!(parse_rule("u:x=none").unwrap().access, Access::None);
    }

    #[test]
    fn malformed_rules_say_what_is_wrong() {
        assert!(
            parse_rule("anna")
                .unwrap_err()
                .to_string()
                .contains("no '='")
        );
        assert!(
            parse_rule("anna=rw")
                .unwrap_err()
                .to_string()
                .contains("user or group")
        );
        assert!(
            parse_rule("team:anna=rw")
                .unwrap_err()
                .to_string()
                .contains("'team'")
        );
        assert!(
            parse_rule("user:=rw")
                .unwrap_err()
                .to_string()
                .contains("nobody")
        );
        assert!(
            parse_rule("user:anna=sometimes")
                .unwrap_err()
                .to_string()
                .contains("sometimes")
        );
    }

    #[test]
    fn a_plan_resolves_names_and_marks_administrators() {
        let rules = [
            parse_rule("user:fabian=read").unwrap(),
            parse_rule("group:family=rw").unwrap(),
            parse_rule("user:anna=inherit").unwrap(),
        ];
        let changes = plan_changes(&acl(), &rules).unwrap();
        assert_eq!(changes.len(), 3);
        assert_eq!(changes[0].kind, PrincipalKind::User);
        assert_eq!(changes[0].permission_mask, 4);
        assert!(changes[0].is_admin);
        assert_eq!(changes[1].kind, PrincipalKind::Group);
        assert_eq!(changes[1].permission_mask, 7);
        assert!(!changes[1].is_admin);
        assert_eq!(changes[2].permission_mask, -1);
        assert!(!changes[2].is_admin);
    }

    #[test]
    fn an_unknown_name_is_refused_with_the_known_ones() {
        let err = plan_changes(&acl(), &[parse_rule("user:bob=rw").unwrap()])
            .unwrap_err()
            .to_string();
        assert!(err.contains("no user 'bob'"), "{err}");
        assert!(err.contains("fabian, anna"), "{err}");
        let err = plan_changes(&acl(), &[parse_rule("group:ghosts=rw").unwrap()])
            .unwrap_err()
            .to_string();
        assert!(err.contains("no group 'ghosts'"), "{err}");
        assert!(err.contains("admin, family"), "{err}");
    }

    #[test]
    fn a_group_is_not_looked_up_among_the_users() {
        assert!(plan_changes(&acl(), &[parse_rule("user:family=rw").unwrap()]).is_err());
        assert!(plan_changes(&acl(), &[parse_rule("group:anna=rw").unwrap()]).is_err());
    }

    #[test]
    fn rules_the_list_shows_are_applied_and_the_others_are_named() {
        // `read` is stored as 5, and still counts as read.
        let applied = [parse_rule("group:family=read").unwrap()];
        assert_eq!(unapplied(&acl(), &applied), Vec::<String>::new());

        // The administrators group stays at read and write.
        let refused = unapplied(&acl(), &[parse_rule("group:admin=read").unwrap()]);
        assert_eq!(refused, vec!["group 'admin' stays at rw instead of read"]);

        let refused = unapplied(&acl(), &[parse_rule("user:anna=rw").unwrap()]);
        assert_eq!(refused, vec!["user 'anna' stays at inherit instead of rw"]);

        let gone = unapplied(&acl(), &[parse_rule("user:bob=rw").unwrap()]);
        assert_eq!(gone, vec!["user 'bob' is gone from the list"]);
    }
}
