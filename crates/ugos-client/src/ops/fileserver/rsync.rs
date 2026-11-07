//! The rsync service: one port, one account, one password.

use serde_json::Value;

use super::fields::{Field, Kind};
use super::{Config, PortUse, number, refused};
use crate::api::users::UsersApi;
use crate::client::UgosClient;
use crate::error::Result;

/// The settings. The password is not one of them, it travels sealed.
pub const FIELDS: &[Field] = &[
    Field::of(
        "port",
        "port",
        Kind::Int(1, 65535),
        "port, 873 is the usual one",
    ),
    Field::of(
        "account",
        "account",
        Kind::Text(64),
        "the account that rsync logs in as",
    ),
];

/// The longest password that fits one RSA block. The web UI allows 127
/// characters by splitting; how UGOS reads that is not known, so this
/// client stops at one block.
const LONGEST: usize = 117;

fn text<'a>(config: &'a Config, key: &str) -> &'a str {
    config.get(key).and_then(Value::as_str).unwrap_or_default()
}

pub(super) async fn finish(
    client: &UgosClient,
    current: &Config,
    next: &mut Config,
    enable: bool,
    password: Option<&str>,
) -> Result<Vec<PortUse>> {
    let account = text(next, "account").to_owned();
    if enable && account.is_empty() {
        return Err(refused("rsync needs an account: give account=NAME"));
    }
    if !account.is_empty() {
        if account.chars().any(char::is_whitespace) {
            return Err(refused("rsync cannot use an account name with spaces"));
        }
        let _ = client.user_find(&account).await?;
    }
    let changed = account != text(current, "account");
    if enable && changed && password.is_none_or(str::is_empty) {
        return Err(refused("a new account needs its password"));
    }
    let sealed = match password.filter(|p| !p.is_empty()) {
        Some(p) if p.chars().count() > LONGEST => {
            return Err(refused(format!(
                "the password is at most {LONGEST} characters"
            )));
        }
        Some(p) if p.chars().any(char::is_whitespace) => {
            return Err(refused("the password has no spaces"));
        }
        Some(p) => client.seal_password(p).await?,
        // UGOS keeps the old password when it gets an empty one.
        None => String::new(),
    };
    let _ = next.insert("password".into(), Value::String(sealed));
    let _ = next.insert("user_not_found".into(), Value::Bool(false));
    let port = match number(next, "port") {
        0 => 873,
        p => p,
    };
    let _ = next.insert("port".into(), Value::from(port));
    Ok(vec![PortUse {
        module: "rsync",
        key: "port",
        name: None,
        port,
    }])
}
