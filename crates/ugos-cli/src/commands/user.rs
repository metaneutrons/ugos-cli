//! `user` and `group`: the accounts on the NAS, and their groups.
//!
//! The checks and the read-back live in `ugos_client::ops::users`, which the
//! MCP server uses too. This file parses the command line, asks for the
//! password, and prints.

mod group;

use std::io::{IsTerminal, Write};

use anyhow::{Result, bail};
use ugos_client::UgosClient;
use ugos_client::api::syslog::SysLogApi;
use ugos_client::api::users::{UsersApi, create_body};
use ugos_client::ops::users::{
    Expiry, UserChanges, UserRequest, build_new_user, create_user, delete_user, parse_date, today,
    update_user,
};
use ugos_client::types::syslog::User;
use ugos_client::types::users::{ROLE_ADMIN, ROLE_USER};

pub use self::group::group;
use super::share::{flag_pair, with_force_hint};
use crate::cli::{OutputFormat, RoleArg, UserAction};
use crate::output;

/// Environment variable that holds a new password.
const PASSWORD_ENV: &str = "UGOS_NEW_PASSWORD";

/// Run a `user` subcommand.
pub async fn user(
    client: &UgosClient,
    action: &UserAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    if super::access::user_access(client, action, fmt, w)
        .await?
        .is_some()
    {
        return Ok(());
    }
    match action {
        UserAction::List => list(client, fmt, w).await,
        UserAction::Me => show_account(&client.current_user().await?, fmt, w),
        UserAction::Show { account } => show_account(&client.user_find(account).await?, fmt, w),
        UserAction::Create {
            name,
            email,
            description,
            role,
            groups,
            no_home_dir,
            deny_password_change,
            new_password_stdin,
            dry_run,
        } => {
            let request = UserRequest {
                name: name.clone(),
                email: email.clone(),
                description: description.clone(),
                role: role_name(*role).to_owned(),
                groups: groups.clone(),
                home_dir: !*no_home_dir,
                deny_password_change: *deny_password_change,
            };
            create(client, &request, *new_password_stdin, *dry_run, fmt, w).await
        }
        UserAction::Update { account, .. } => {
            let change = changes_from(action)?;
            update(client, account, &change, fmt, w).await
        }
        UserAction::Delete { account, force } => {
            let found = delete_user(client, account, *force)
                .await
                .map_err(with_force_hint)?;
            output::print_success(w, &format!("Deleted account '{}'", found.username), fmt)
        }
        // Handled above, by `user_access`.
        _ => Ok(()),
    }
}

async fn list(client: &UgosClient, fmt: OutputFormat, w: &mut impl Write) -> Result<()> {
    let users = client.users().await?;
    match fmt {
        OutputFormat::Table => {
            let rows: Vec<output::UserRow> = users.iter().map(Into::into).collect();
            output::print_list(w, &rows, fmt)
        }
        OutputFormat::Json => output::print_json(w, &users),
    }
}

fn show_account(account: &User, fmt: OutputFormat, w: &mut impl Write) -> Result<()> {
    match fmt {
        OutputFormat::Table => output::print_list(w, &output::user_rows(account), fmt),
        OutputFormat::Json => output::print_json(w, account),
    }
}

const fn role_name(role: RoleArg) -> &'static str {
    match role {
        RoleArg::User => ROLE_USER,
        RoleArg::Admin => ROLE_ADMIN,
    }
}

// ── Passwords ───────────────────────────────────────────────────────

/// A new password: from stdin when asked to, else from `UGOS_NEW_PASSWORD`,
/// else typed on the terminal without echo.
fn new_password_from(from_stdin: bool) -> Result<String> {
    if from_stdin {
        return crate::password_from_reader(&mut std::io::stdin().lock(), "--new-password-stdin");
    }
    if let Some(password) = std::env::var(PASSWORD_ENV).ok().filter(|v| !v.is_empty()) {
        return Ok(password);
    }
    if !std::io::stdin().is_terminal() {
        bail!("no new password: set {PASSWORD_ENV} or pass --new-password-stdin");
    }
    let first = rpassword::prompt_password("New password: ")?;
    let again = rpassword::prompt_password("Repeat the password: ")?;
    if first != again {
        bail!("the two passwords differ");
    }
    if first.is_empty() {
        bail!("the password is empty");
    }
    Ok(first)
}

// ── Create ──────────────────────────────────────────────────────────

async fn create(
    client: &UgosClient,
    request: &UserRequest,
    password_from_stdin: bool,
    dry_run: bool,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    // Check the request before asking for a password.
    let new = build_new_user(request)?;
    if dry_run {
        return output::print_json(w, &create_body(&new, "<password>"));
    }
    let password = new_password_from(password_from_stdin)?;
    let made = create_user(client, request, &password).await?;
    match fmt {
        OutputFormat::Table => {
            writeln!(w, "Created account '{}' (uid {})", made.username, made.uid)?;
            output::print_list(w, &output::user_rows(&made), fmt)
        }
        OutputFormat::Json => output::print_json(w, &made),
    }
}

// ── Update ──────────────────────────────────────────────────────────

/// The expiry that `--disable` or `--enable` and `--no-expiry` ask for.
const fn expiry_change(disable: bool, lift: bool) -> Option<Expiry> {
    match (disable, lift) {
        (true, _) => Some(Expiry::DISABLED),
        (false, true) => Some(Expiry::NEVER),
        (false, false) => None,
    }
}

/// What an `update` command line asks for. Reads the password here, so that a
/// missing one stops the command before anything is sent.
fn changes_from(action: &UserAction) -> Result<UserChanges> {
    let UserAction::Update {
        email,
        description,
        role,
        add_groups,
        remove_groups,
        home_dir,
        no_home_dir,
        deny_password_change,
        allow_password_change,
        disable,
        enable,
        no_expiry,
        expires,
        new_password_stdin,
        new_password,
        ..
    } = action
    else {
        bail!("not an update command");
    };
    let password = if *new_password_stdin || *new_password {
        Some(new_password_from(*new_password_stdin)?)
    } else {
        None
    };
    Ok(UserChanges {
        email: email.clone(),
        description: description.clone(),
        role: role.map(|role| role_name(role).to_owned()),
        add_groups: add_groups.clone(),
        remove_groups: remove_groups.clone(),
        home_dir: flag_pair(*home_dir, *no_home_dir),
        deny_password_change: flag_pair(*deny_password_change, *allow_password_change),
        expiry: match expires {
            Some(text) => Some(parse_date(text, today())?),
            None => expiry_change(*disable, *enable || *no_expiry),
        },
        password,
    })
}

async fn update(
    client: &UgosClient,
    account: &str,
    change: &UserChanges,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    if change.is_empty() {
        bail!("nothing to change: give a field to set, see `user update --help`");
    }
    let now = update_user(client, account, change).await?;
    match fmt {
        OutputFormat::Table => {
            writeln!(w, "Updated account '{}'", now.username)?;
            output::print_list(w, &output::user_rows(&now), fmt)
        }
        OutputFormat::Json => output::print_json(w, &now),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disable_and_enable_pick_the_expiry() {
        assert_eq!(expiry_change(true, false), Some(Expiry::DISABLED));
        assert_eq!(expiry_change(true, true), Some(Expiry::DISABLED));
        assert_eq!(expiry_change(false, true), Some(Expiry::NEVER));
        assert_eq!(expiry_change(false, false), None);
    }

    #[test]
    fn roles_map_to_what_ugos_calls_them() {
        assert_eq!(role_name(RoleArg::User), ROLE_USER);
        assert_eq!(role_name(RoleArg::Admin), ROLE_ADMIN);
    }
}
