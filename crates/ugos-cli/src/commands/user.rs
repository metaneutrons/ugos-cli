//! `user` and `group`: the accounts on the NAS, and their groups.

mod delete;
mod expiry;
mod group;

use std::io::{IsTerminal, Write};

use anyhow::{Context, Result, anyhow, bail};
use ugos_client::UgosClient;
use ugos_client::api::syslog::SysLogApi;
use ugos_client::api::users::{UsersApi, create_body};
use ugos_client::crypto::decrypt_email;
use ugos_client::error::UgosError;
use ugos_client::types::syslog::User;
use ugos_client::types::users::{NewUser, ROLE_ADMIN, ROLE_USER, UserEdit};

use self::expiry::{Expiry, parse_date, today};
pub use self::group::group;
use super::share::flag_pair;
use crate::cli::{OutputFormat, RoleArg, UserAction};
use crate::output;

/// Names the web UI refuses, whatever the case.
const RESERVED_NAMES: [&str; 3] = ["root", "admin", "administrator"];

/// Environment variable that holds a new password.
const PASSWORD_ENV: &str = "UGOS_NEW_PASSWORD";

/// Run a `user` subcommand.
pub async fn user(
    client: &UgosClient,
    action: &UserAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
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
            let options = CreateOptions {
                name,
                email: email.as_deref(),
                description: description.as_deref(),
                role: *role,
                groups,
                home_dir: !*no_home_dir,
                deny_password_change: *deny_password_change,
            };
            create(client, &options, *new_password_stdin, *dry_run, fmt, w).await
        }
        UserAction::Update { account, .. } => {
            let change = UserChanges::from_action(action)?;
            update(client, account, &change, fmt, w).await
        }
        UserAction::Delete { account, force } => {
            delete::delete(client, account, *force, fmt, w).await
        }
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

/// What `user create` was asked for.
struct CreateOptions<'a> {
    name: &'a str,
    email: Option<&'a str>,
    description: Option<&'a str>,
    role: RoleArg,
    groups: &'a [String],
    home_dir: bool,
    deny_password_change: bool,
}

/// The request `user create` sends, without a password.
fn build_new_user(options: &CreateOptions<'_>) -> Result<NewUser> {
    check_name(options.name)?;
    let mut new = NewUser::new(options.name.trim(), "");
    options
        .email
        .unwrap_or_default()
        .trim()
        .clone_into(&mut new.email);
    options
        .description
        .unwrap_or_default()
        .clone_into(&mut new.description);
    role_name(options.role).clone_into(&mut new.role);
    new.groups = options.groups.to_vec();
    new.enable_home_dir = options.home_dir;
    new.deny_change_pwd = options.deny_password_change;
    if new.role == ROLE_ADMIN {
        // The web UI clears this for administrators.
        new.deny_change_pwd = false;
    }
    Ok(new)
}

async fn create(
    client: &UgosClient,
    options: &CreateOptions<'_>,
    password_from_stdin: bool,
    dry_run: bool,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    let mut new = build_new_user(options)?;
    if dry_run {
        return output::print_json(w, &create_body(&new, "<password>"));
    }
    if !client.user_name_free(&new.username).await? {
        bail!("an account named '{}' already exists", new.username);
    }
    new.password = new_password_from(password_from_stdin)?;
    let _ = client
        .user_create(&new)
        .await
        .map_err(|e| explain(e, !new.groups.is_empty()))?;

    // Read the account back: what UGOS holds is what counts.
    let made = client.user_find(&new.username).await?;
    if let Some(problem) = created_mismatch(&new, &made) {
        bail!("UGOS created '{}' but {problem}", made.username);
    }
    match fmt {
        OutputFormat::Table => {
            writeln!(w, "Created account '{}' (uid {})", made.username, made.uid)?;
            output::print_list(w, &output::user_rows(&made), fmt)
        }
        OutputFormat::Json => output::print_json(w, &made),
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

/// UGOS answers an unknown group with a bare "Failed to operate"; say what
/// the likely cause is when groups were part of the request.
fn explain(error: UgosError, groups_named: bool) -> anyhow::Error {
    match error {
        UgosError::Api { code: 9999, .. } if groups_named => anyhow!(
            "UGOS refused the request; check the group names, only groups that exist can be set"
        ),
        other => other.into(),
    }
}

/// A login name the web UI would accept.
fn check_name(name: &str) -> Result<()> {
    if name.trim().is_empty() {
        bail!("the user name is empty");
    }
    if name != name.trim() {
        bail!("the user name must not start or end with a space");
    }
    if name.starts_with('-') {
        bail!("the user name must not start with '-'");
    }
    if RESERVED_NAMES.contains(&name.to_ascii_lowercase().as_str()) {
        bail!("'{name}' is a name UGOS keeps for itself");
    }
    Ok(())
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

/// What `user update` was asked to change.
#[derive(Debug, Default, PartialEq, Eq)]
struct UserChanges {
    email: Option<String>,
    description: Option<String>,
    role: Option<&'static str>,
    add_groups: Vec<String>,
    remove_groups: Vec<String>,
    home_dir: Option<bool>,
    deny_password_change: Option<bool>,
    /// New expiry: never, disabled, or a date.
    expiry: Option<Expiry>,
    password: Option<String>,
}

impl UserChanges {
    /// What an `update` command line asks for. Reads the password here, so
    /// that a missing one stops the command before anything is sent.
    fn from_action(action: &UserAction) -> Result<Self> {
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
        Ok(Self {
            email: email.clone(),
            description: description.clone(),
            role: role.map(role_name),
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

    fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// The account as it should be after the change.
    fn apply(&self, user: &User) -> Result<UserEdit> {
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
            role: self.role.map_or_else(|| user.role.clone(), str::to_owned),
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
    fn expiry_differs(&self, edit: &UserEdit, now: &User) -> bool {
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
    fn mismatch(&self, edit: &UserEdit, now: &User) -> Option<String> {
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
        bail!(
            "UGOS shows only a masked address for '{}'; pass --email so that it is not lost",
            user.username
        );
    }
    decrypt_email(&user.email_encrypted).with_context(|| {
        format!(
            "reading the address of '{}'; pass --email to set it anew",
            user.username
        )
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

async fn update(
    client: &UgosClient,
    user: &str,
    change: &UserChanges,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    if change.is_empty() {
        bail!("nothing to change: give a field to set, see `user update --help`");
    }
    let before = client.user_find(user).await?;
    refuse_lockout(client, &before, change).await?;
    let edit = change.apply(&before)?;
    client
        .user_edit(&edit)
        .await
        .map_err(|e| explain(e, !change.add_groups.is_empty()))?;

    // UGOS may answer success and keep the old value, so look at what it holds.
    let now = client.user_find(&before.username).await?;
    if let Some(problem) = change.mismatch(&edit, &now) {
        bail!("UGOS accepted the request but {problem}");
    }
    match fmt {
        OutputFormat::Table => {
            writeln!(w, "Updated account '{}'", now.username)?;
            output::print_list(w, &output::user_rows(&now), fmt)
        }
        OutputFormat::Json => output::print_json(w, &now),
    }
}

/// Refuse a change that would lock the administrator out.
async fn refuse_lockout(client: &UgosClient, target: &User, change: &UserChanges) -> Result<()> {
    let demotes = change.role.is_some_and(|role| role != ROLE_ADMIN) && target.role == ROLE_ADMIN;
    let disables = change.expiry.as_ref().is_some_and(Expiry::is_disabled);
    if !demotes && !disables {
        return Ok(());
    }
    if client.current_user().await?.uid == target.uid {
        bail!("this is the account the CLI is logged in with; it would lock you out");
    }
    if demotes && admins(&client.users().await?) <= 1 {
        bail!("'{}' is the only administrator", target.username);
    }
    Ok(())
}

fn admins(users: &[User]) -> usize {
    users.iter().filter(|user| user.role == ROLE_ADMIN).count()
}

// ── Delete ──────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests;
