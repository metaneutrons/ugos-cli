//! Arguments of the `user` commands for password policies, two-factor
//! authentication, avatars, invitations and imports.

use std::path::PathBuf;

use clap::{Args, Subcommand};

use super::OnOff;

/// The strength rules of passwords.
#[derive(Debug, Args)]
pub struct PasswordRulesArgs {
    /// The shortest password, 6 to 127.
    #[arg(long)]
    pub min_length: Option<i64>,
    /// No user name in the password.
    #[arg(long, value_enum)]
    pub no_username: Option<OnOff>,
    /// No common password.
    #[arg(long, value_enum)]
    pub no_common: Option<OnOff>,
    /// Upper and lower case.
    #[arg(long, value_enum)]
    pub mixed_case: Option<OnOff>,
    /// A digit.
    #[arg(long, value_enum)]
    pub digit: Option<OnOff>,
    /// A special character.
    #[arg(long, value_enum)]
    pub special: Option<OnOff>,
}

/// The expiry of passwords.
#[derive(Debug, Args)]
pub struct PasswordExpiryArgs {
    /// Whether passwords expire.
    #[arg(long, value_enum)]
    pub enable: Option<OnOff>,
    /// Days a password is valid, 1 to 999.
    #[arg(long)]
    pub max_days: Option<i64>,
    /// Days of warning before it ends, 0 to 99.
    #[arg(long)]
    pub warn_days: Option<i64>,
    /// Force the change when it expires.
    #[arg(long, value_enum)]
    pub force_change: Option<OnOff>,
    /// An account whose password does not expire. Replaces the list; repeat
    /// for several.
    #[arg(long = "exempt")]
    pub exempt: Vec<String>,
    /// Go on although accounts are locked out until they change their
    /// password.
    #[arg(long)]
    pub force: bool,
}

/// An avatar.
#[derive(Debug, Args)]
pub struct AvatarArgs {
    /// Login name or numeric uid.
    pub account: String,
    /// One of the offered avatars, like `05.png`.
    #[arg(long, conflicts_with = "file")]
    pub builtin: Option<String>,
    /// A picture on this machine, under 8 MB.
    #[arg(long)]
    pub file: Option<PathBuf>,
}

/// An import of accounts from a spreadsheet.
#[derive(Debug, Args)]
pub struct ImportArgs {
    /// The `.xls` or `.xlsx` file, from the template of the web UI.
    pub file: PathBuf,
    /// Create the accounts. Without this the rows are only read and shown.
    #[arg(long)]
    pub commit: bool,
    /// The accounts have to change their password at the first login.
    #[arg(long)]
    pub change_password: bool,
    /// Go on although this creates accounts whose passwords are in the file.
    #[arg(long)]
    pub force: bool,
}

/// How an applicant is set up.
#[derive(Debug, Args)]
pub struct ApproveArgs {
    /// The application id, from `requests`.
    pub id: i64,
    /// `users` or `admin`.
    #[arg(long, default_value = "users")]
    pub role: String,
    /// A group to join; repeat for several.
    #[arg(long = "group")]
    pub groups: Vec<String>,
    /// A description.
    #[arg(long, default_value = "")]
    pub description: String,
    /// Give no home folder.
    #[arg(long)]
    pub no_home_dir: bool,
    /// Go on although this gives administrator rights.
    #[arg(long)]
    pub force: bool,
}

/// `user invite` subcommands.
#[derive(Debug, Subcommand)]
pub enum InviteAction {
    /// Whether invitations are on, and the link.
    Status,
    /// Switch invitations on.
    Enable {
        /// Go on although anyone with the link can apply for an account.
        #[arg(long)]
        force: bool,
    },
    /// Switch invitations off.
    Disable,
    /// The applications waiting.
    Requests,
    /// The applications that were processed.
    Processed,
    /// Approve an application.
    Approve(ApproveArgs),
    /// Reject an application.
    Reject {
        /// The application id.
        id: i64,
    },
}
