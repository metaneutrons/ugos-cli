//! Password policies, two-factor authentication, avatars, invitations and
//! imports, as `user` subcommands.

use std::io::Write;

use anyhow::Result;
use ugos_client::UgosClient;
use ugos_client::api::syslog::SysLogApi;
use ugos_client::api::users::UsersApi;
use ugos_client::ops::policy::{self, Approval, Avatar, ExpiryChange, RulesChange};

use super::hw::show;
use super::share::with_force_hint;
use crate::cli::{
    ApproveArgs, ImportArgs, InviteAction, OnOff, OutputFormat, PasswordExpiryArgs,
    PasswordRulesArgs, UserAction,
};
use crate::output;
use crate::output::policy as rows;

async fn rules_cmd(
    client: &UgosClient,
    a: &PasswordRulesArgs,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    let change = RulesChange {
        min_length: a.min_length,
        no_username: a.no_username.map(OnOff::flag),
        no_common: a.no_common.map(OnOff::flag),
        mixed_case: a.mixed_case.map(OnOff::flag),
        digit: a.digit.map(OnOff::flag),
        special: a.special.map(OnOff::flag),
    };
    let got = if change == RulesChange::default() {
        policy::password_rules(client).await?
    } else {
        policy::set_password_rules(client, &change).await?
    };
    show(w, fmt, &got, &rows::rules_rows(&got))
}

async fn expiry_cmd(
    client: &UgosClient,
    a: &PasswordExpiryArgs,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    let change = ExpiryChange {
        enable: a.enable.map(OnOff::flag),
        max_days: a.max_days,
        warn_days: a.warn_days,
        force_change: a.force_change.map(OnOff::flag),
        exempt: (!a.exempt.is_empty()).then(|| a.exempt.clone()),
    };
    let got = if change == ExpiryChange::default() {
        policy::password_expiry(client).await?
    } else {
        policy::set_password_expiry(client, &change, a.force)
            .await
            .map_err(with_force_hint)?
    };
    show(w, fmt, &got, &rows::expiry_rows(&got))
}

async fn invite_cmd(
    client: &UgosClient,
    action: &InviteAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    let me = client.current_user().await?.uid;
    match action {
        InviteAction::Status => {
            let code = policy::invite_code(client, me).await?;
            if code.is_empty() {
                writeln!(w, "Invitations are off")?;
            } else {
                let base = policy::invite_link_base(client).await?;
                writeln!(w, "Invitations are on, code {code}")?;
                writeln!(w, "Addresses: {base}")?;
            }
            Ok(())
        }
        InviteAction::Enable { force } => {
            let code = policy::set_invites(client, me, true, *force)
                .await
                .map_err(with_force_hint)?;
            writeln!(w, "Invitations are on, code {code}")?;
            Ok(())
        }
        InviteAction::Disable => {
            let _ = policy::set_invites(client, me, false, false).await?;
            writeln!(w, "Invitations are off")?;
            Ok(())
        }
        InviteAction::Requests | InviteAction::Processed => {
            let list =
                policy::invitations(client, matches!(action, InviteAction::Processed)).await?;
            let table: Vec<rows::InvitationRow> = list.iter().map(Into::into).collect();
            output::print_list(w, &table, fmt)
        }
        InviteAction::Approve(a) => approve_cmd(client, a, w).await,
        InviteAction::Reject { id } => {
            policy::reject(client, *id).await?;
            writeln!(w, "Application {id} is rejected")?;
            Ok(())
        }
    }
}

async fn approve_cmd(client: &UgosClient, a: &ApproveArgs, w: &mut impl Write) -> Result<()> {
    let how = Approval {
        role: a.role.clone(),
        groups: a.groups.clone(),
        description: a.description.clone(),
        home_dir: !a.no_home_dir,
    };
    policy::approve(client, a.id, &how, a.force)
        .await
        .map_err(with_force_hint)?;
    writeln!(w, "Application {} is approved", a.id)?;
    Ok(())
}

async fn import_cmd(
    client: &UgosClient,
    a: &ImportArgs,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    let list = policy::read_import(client, &a.file).await?;
    let table: Vec<rows::ImportRowView> = list.iter().map(Into::into).collect();
    output::print_list(w, &table, fmt)?;
    if a.commit {
        policy::commit_import(client, &list, a.change_password, a.force)
            .await
            .map_err(with_force_hint)?;
        wait_import(client, list.len(), w).await?;
    }
    Ok(())
}

/// Wait for UGOS to finish an import: back off, and give up after two
/// minutes.
async fn wait_import(client: &UgosClient, count: usize, w: &mut impl Write) -> Result<()> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_mins(2);
    let mut pause = std::time::Duration::from_secs(2);
    while std::time::Instant::now() < deadline {
        tokio::time::sleep(pause).await;
        pause = (pause * 2).min(std::time::Duration::from_secs(10));
        if !policy::import_running(client).await? {
            writeln!(w, "Imported {count} accounts")?;
            return Ok(());
        }
    }
    anyhow::bail!("the import was still running after two minutes; check `user list`")
}

/// The policy subcommands of `user`. `None` when the action is not one.
pub async fn user_policy(
    client: &UgosClient,
    action: &UserAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<Option<()>> {
    match action {
        UserAction::PasswordRules(a) => rules_cmd(client, a, fmt, w).await?,
        UserAction::PasswordExpiry(a) => expiry_cmd(client, a, fmt, w).await?,
        UserAction::PasswordChanged { account } => {
            let user = client.user_find(account).await?;
            let when = policy::last_password_change(client, user.uid).await?;
            writeln!(w, "{}", rows::time_text(when))?;
        }
        UserAction::OtpDisable { account, force } => {
            let user = client.user_find(account).await?;
            policy::disable_otp(client, user.uid, *force)
                .await
                .map_err(with_force_hint)?;
            writeln!(w, "The second factor of {} is off", user.username)?;
        }
        UserAction::Avatars => {
            for name in policy::avatars(client).await? {
                writeln!(w, "{name}")?;
            }
        }
        UserAction::Avatar(a) => {
            let user = client.user_find(&a.account).await?;
            let avatar = match (&a.builtin, &a.file) {
                (Some(name), None) => Avatar::Builtin(name.clone()),
                (None, Some(path)) => Avatar::File(path.clone()),
                _ => anyhow::bail!("give --builtin NAME or --file PICTURE"),
            };
            let id = policy::set_avatar(client, user.uid, &avatar).await?;
            writeln!(w, "Avatar set ({} bytes of picture)", id.len())?;
        }
        UserAction::Import(a) => import_cmd(client, a, fmt, w).await?,
        UserAction::Invite { action } => invite_cmd(client, action, fmt, w).await?,
        _ => return Ok(None),
    }
    Ok(Some(()))
}
