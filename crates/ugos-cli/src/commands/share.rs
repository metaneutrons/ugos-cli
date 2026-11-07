//! `share` and `smb`: shared folders and the SMB file service.

use std::io::Write;

use anyhow::{Result, anyhow, bail};
use ugos_client::UgosClient;
use ugos_client::api::files::FilesApi;
use ugos_client::api::shares::SharesApi;
use ugos_client::error::UgosError;
use ugos_client::ops::shares::{
    SettingsChange, ShareRequest, build_new_share, create_share, delete_share, parse_quota,
    set_share_acl, share_settings, update_share,
};

use crate::cli::{OutputFormat, ShareAclAction, ShareAction, SmbAction};
use crate::output;

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
            let request = ShareRequest {
                name: name.clone(),
                volume: volume.clone(),
                quota: quota.clone(),
                hide_in_network: *hide_in_network,
                show_unreadable: *show_unreadable,
                no_recycle: *no_recycle,
                recycle_for_everyone: *recycle_for_everyone,
                data_check: *data_check,
            };
            create(client, &request, *dry_run, fmt, w).await
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
        (Some(text), _) => Ok(Some(parse_quota(text)?)),
        (None, true) => Ok(Some((0, 1))),
        (None, false) => Ok(None),
    }
}

/// `Some(true)` for the first flag, `Some(false)` for the second, else `None`.
pub(super) const fn flag_pair(on: bool, off: bool) -> Option<bool> {
    match (on, off) {
        (true, _) => Some(true),
        (false, true) => Some(false),
        (false, false) => None,
    }
}

/// Say how to go on when a refusal is one that `--force` overrides.
pub(super) fn with_force_hint(error: UgosError) -> anyhow::Error {
    match error {
        UgosError::NeedsForce(message) => anyhow!("{message}; pass --force to go on"),
        other => other.into(),
    }
}

async fn create(
    client: &UgosClient,
    request: &ShareRequest,
    dry_run: bool,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    if dry_run {
        let volumes = client.fs_volumes().await?;
        return output::print_json(w, &build_new_share(&volumes, request)?);
    }
    let created = create_share(client, request).await?;
    match fmt {
        OutputFormat::Table => {
            writeln!(
                w,
                "Created share '{}' ({})",
                request.name.trim(),
                created.path
            )?;
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
    let now = update_share(client, share, change).await?;
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
    let found = delete_share(client, share, force)
        .await
        .map_err(with_force_hint)?;
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
    let (found, after) = set_share_acl(client, share, rules).await?;
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
async fn settings_of(
    client: &UgosClient,
    name_or_id: &str,
) -> Result<ugos_client::types::shares::ShareInfo> {
    Ok(share_settings(client, name_or_id).await?)
}

#[cfg(test)]
mod tests {
    use super::flag_pair;

    #[test]
    fn a_flag_pair_says_on_off_or_nothing() {
        assert_eq!(flag_pair(true, false), Some(true));
        assert_eq!(flag_pair(false, true), Some(false));
        assert_eq!(flag_pair(false, false), None);
    }
}
