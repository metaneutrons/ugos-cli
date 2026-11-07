//! `user delete`, and what it must not take with it.

use std::io::Write;

use anyhow::{Result, bail};
use ugos_client::UgosClient;
use ugos_client::api::files::FilesApi;
use ugos_client::api::syslog::SysLogApi;
use ugos_client::api::users::UsersApi;
use ugos_client::error::UgosError;
use ugos_client::types::syslog::User;
use ugos_client::types::users::ROLE_ADMIN;

use super::admins;
use crate::cli::OutputFormat;
use crate::output;

pub(super) async fn delete(
    client: &UgosClient,
    user: &str,
    force: bool,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    let found = client.user_find(user).await?;
    refuse_lockout_by_delete(client, &found).await?;
    refuse_losses(client, &found, force).await?;
    client.user_delete(&[found.uid]).await?;
    let left = client.users().await?;
    if left.iter().any(|remaining| remaining.uid == found.uid) {
        bail!(
            "UGOS accepted the request but '{}' is still there",
            found.username
        );
    }
    output::print_success(w, &format!("Deleted account '{}'", found.username), fmt)
}

/// Refuse to delete the account the CLI is logged in with, or the only
/// administrator.
async fn refuse_lockout_by_delete(client: &UgosClient, found: &User) -> Result<()> {
    if client.current_user().await?.uid == found.uid {
        bail!("this is the account the CLI is logged in with");
    }
    if found.role == ROLE_ADMIN && admins(&client.users().await?) <= 1 {
        bail!("'{}' is the only administrator", found.username);
    }
    Ok(())
}

/// Refuse a delete that would take data with it, unless `--force` says so.
/// A safe space is refused whatever `--force` says.
async fn refuse_losses(client: &UgosClient, found: &User, force: bool) -> Result<()> {
    let check = client.user_check_delete(&[found.uid]).await?;
    if check.safespace {
        bail!(
            "'{}' has a safe space; delete that first in the web UI",
            found.username
        );
    }
    if force {
        return Ok(());
    }
    if check.snapshot {
        bail!(
            "'{}' has snapshots that would be deleted with the account; pass --force to go on",
            found.username
        );
    }
    if found.enable_home_dir {
        match home_entries(client, found).await {
            Ok(0) => {}
            Ok(count) => bail!(
                "the home folder of '{}' holds {count} {}, which are deleted with the \
                 account; pass --force to go on",
                found.username,
                if count == 1 { "entry" } else { "entries" }
            ),
            Err(e) => bail!(
                "cannot tell whether the home folder of '{}' is empty ({e}); pass --force to go on",
                found.username
            ),
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
        Err(e) => Err(e.into()),
    }
}
