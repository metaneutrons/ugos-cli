//! Groups, share permissions and quotas of accounts.

use std::io::Write;

use anyhow::{Result, bail};
use ugos_client::UgosClient;
use ugos_client::api::users::UsersApi;
use ugos_client::ops::access::{self, GroupChange, Level, Owner};

use super::share::with_force_hint;
use crate::cli::{GroupAction, OutputFormat, UserAction};
use crate::output;
use crate::output::access as rows;

fn level(text: &str) -> Result<Level> {
    Ok(match text {
        "none" => Level::None,
        "read" => Level::Read,
        "write" => Level::Write,
        "inherit" => Level::Inherit,
        other => bail!("{other:?} is not none, read, write or inherit"),
    })
}

/// Rules as `share=level`.
fn rules(list: &[String]) -> Result<Vec<(String, Level)>> {
    list.iter()
        .map(|r| {
            let (share, text) = r
                .split_once('=')
                .ok_or_else(|| anyhow::anyhow!("{r:?} is not share=level"))?;
            Ok((share.to_owned(), level(text)?))
        })
        .collect()
}

/// A limit as `500M`, `100G`, `2T` or `none`.
fn limit(text: &str) -> Result<Option<(i64, &'static str)>> {
    if text.eq_ignore_ascii_case("none") {
        return Ok(None);
    }
    let upper = text.trim().to_ascii_uppercase();
    let (digits, unit) = [
        ("TB", "TB"),
        ("T", "TB"),
        ("GB", "GB"),
        ("G", "GB"),
        ("MB", "MB"),
        ("M", "MB"),
    ]
    .iter()
    .find_map(|(suffix, unit)| upper.strip_suffix(suffix).map(|d| (d, *unit)))
    .ok_or_else(|| anyhow::anyhow!("a limit is a whole number of M, G or T, or none"))?;
    let value: i64 = digits.trim().parse()?;
    Ok(Some((value, unit)))
}

fn permissions_table(
    w: &mut impl Write,
    fmt: OutputFormat,
    list: &[ugos_client::types::access::PermissionRow],
) -> Result<()> {
    let table: Vec<rows::PermissionRow> = list.iter().map(Into::into).collect();
    output::print_list(w, &table, fmt)
}

/// The access subcommands of `user`. `None` when the action is not one.
pub async fn user_access(
    client: &UgosClient,
    action: &UserAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<Option<()>> {
    match action {
        UserAction::Permissions { account } => {
            let user = client.user_find(account).await?;
            permissions_table(
                w,
                fmt,
                &access::permissions(client, Owner::User(user.uid)).await?,
            )?;
        }
        UserAction::Permit { account, rules: r } => {
            let user = client.user_find(account).await?;
            let after = access::set_permissions(client, Owner::User(user.uid), &rules(r)?).await?;
            permissions_table(w, fmt, &after)?;
        }
        UserAction::Quota { account } => {
            let user = client.user_find(account).await?;
            let q = access::quota(client, user.uid).await?;
            let table: Vec<rows::QuotaTableRow> =
                q.btrfs.iter().chain(&q.ext4).map(Into::into).collect();
            output::print_list(w, &table, fmt)?;
        }
        UserAction::QuotaSet {
            account,
            volume,
            limit: l,
        } => {
            let user = client.user_find(account).await?;
            let q = access::set_quota(client, user.uid, volume, limit(l)?).await?;
            let table: Vec<rows::QuotaTableRow> =
                q.btrfs.iter().chain(&q.ext4).map(Into::into).collect();
            output::print_list(w, &table, fmt)?;
        }
        _ => return Ok(None),
    }
    Ok(Some(()))
}

async fn group_permissions(
    client: &UgosClient,
    action: &GroupAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    match action {
        GroupAction::Permissions { group } => {
            let g = access::find_group(client, group).await?;
            permissions_table(
                w,
                fmt,
                &access::permissions(client, Owner::Group(g.gid)).await?,
            )?;
        }
        GroupAction::Permit { group, rules: r } => {
            let g = access::find_group(client, group).await?;
            let after = access::set_permissions(client, Owner::Group(g.gid), &rules(r)?).await?;
            permissions_table(w, fmt, &after)?;
        }
        _ => {}
    }
    Ok(())
}

/// The access subcommands of `group`. `None` when the action is not one.
pub async fn group_access(
    client: &UgosClient,
    action: &GroupAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<Option<()>> {
    match action {
        GroupAction::Create {
            name,
            description,
            members,
        } => {
            let g = access::create_group(client, name, description, members).await?;
            output::print_list(w, &[output::GroupRow::from(&g)], fmt)?;
        }
        GroupAction::Edit {
            group,
            new_name,
            description,
            add,
            remove,
        } => {
            let change = GroupChange {
                name: new_name.clone(),
                description: description.clone(),
                add: add.clone(),
                remove: remove.clone(),
            };
            let g = access::edit_group(client, group, &change).await?;
            output::print_list(w, &[output::GroupRow::from(&g)], fmt)?;
        }
        GroupAction::Delete { group, force } => {
            access::delete_group(client, group, *force)
                .await
                .map_err(with_force_hint)?;
            writeln!(w, "The group {group} is deleted")?;
        }
        GroupAction::Permissions { .. } | GroupAction::Permit { .. } => {
            group_permissions(client, action, fmt, w).await?;
        }
        GroupAction::List => return Ok(None),
    }
    Ok(Some(()))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::{Level, level, limit, rules};

    #[test]
    fn levels_rules_and_limits_are_read() {
        assert_eq!(level("write").unwrap(), Level::Write);
        assert!(level("rw").is_err());
        let r = rules(&["docs=read".into(), "media=none".into()]).unwrap();
        assert_eq!(r[1], ("media".to_owned(), Level::None));
        assert!(rules(&["docs".into()]).is_err());
        assert_eq!(limit("500M").unwrap(), Some((500, "MB")));
        assert_eq!(limit("2tb").unwrap(), Some((2, "TB")));
        assert_eq!(limit("none").unwrap(), None);
        assert!(limit("5K").is_err());
        assert!(limit("1.5G").is_err());
    }
}
