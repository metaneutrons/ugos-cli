//! Initiator groups.

use serde_json::{Value, json};

use super::{check, everything, refused};
use crate::client::UgosClient;
use crate::error::{Result, UgosError};
use crate::types::san::Group;

/// The most initiators a group holds.
const MOST_IQNS: usize = 256;

/// All groups.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn list(client: &UgosClient) -> Result<Vec<Group>> {
    everything(client, "iscsi/group", &[]).await
}

/// A group by name or id.
///
/// # Errors
///
/// Refuses a name that no group has.
pub async fn find(client: &UgosClient, name_or_id: &str) -> Result<Group> {
    let all = list(client).await?;
    all.into_iter()
        .find(|g| g.name == name_or_id || g.id.to_string() == name_or_id)
        .ok_or_else(|| refused(format!("there is no group {name_or_id:?}")))
}

/// The LUNs a group is bound to.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn bound_luns(client: &UgosClient, id: i64) -> Result<Vec<String>> {
    let answer: Value = client
        .get_with_params("iscsi/group/bind", &[("group_id", &id.to_string())])
        .await?;
    Ok(answer
        .get("result")
        .and_then(Value::as_array)
        .map(|l| {
            l.iter()
                .filter_map(|e| e.get("name").and_then(Value::as_str).map(str::to_owned))
                .collect()
        })
        .unwrap_or_default())
}

/// Check a list of initiator IQNs the way the web UI does, and say which
/// belong to another group already.
async fn check_iqns(client: &UgosClient, iqns: &[String], own: Option<&str>) -> Result<()> {
    if iqns.is_empty() || iqns.len() > MOST_IQNS {
        return Err(refused(format!(
            "a group holds 1 to {MOST_IQNS} initiators"
        )));
    }
    let mut seen = Vec::new();
    for iqn in iqns {
        check::initiator_iqn(iqn)?;
        let lower = iqn.to_ascii_lowercase();
        if seen.contains(&lower) {
            return Err(refused(format!("{iqn} is in the list twice")));
        }
        seen.push(lower);
    }
    let answer: Value = client
        .post("iscsi/group/check", &json!({ "iqns": iqns }))
        .await?;
    for entry in answer
        .get("result")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let taken = entry.get("is_exist").and_then(Value::as_bool) == Some(true);
        let group = entry
            .get("group_name")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if taken && Some(group) != own {
            let iqn = entry.get("iqn").and_then(Value::as_str).unwrap_or_default();
            return Err(refused(format!(
                "{iqn} belongs to the group {group:?} already"
            )));
        }
    }
    Ok(())
}

/// A group, once UGOS is done creating or editing it.
async fn settled(client: &UgosClient, name: &str) -> Result<Group> {
    let mut pause = std::time::Duration::from_millis(300);
    let mut group = find(client, name).await?;
    for _ in 0..8 {
        if group.status == 0 {
            break;
        }
        tokio::time::sleep(pause).await;
        pause = (pause * 2).min(std::time::Duration::from_secs(3));
        group = find(client, name).await?;
    }
    Ok(group)
}

/// Make a group.
///
/// # Errors
///
/// Refuses what the web UI refuses.
pub async fn create(client: &UgosClient, name: &str, desc: &str, iqns: &[String]) -> Result<Group> {
    check::name(name)?;
    check::description(desc)?;
    if list(client).await?.iter().any(|g| g.name == name) {
        return Err(refused(format!("a group {name:?} exists already")));
    }
    check_iqns(client, iqns, None).await?;
    let body = json!({ "name": name, "desc": desc, "iqns": iqns });
    let _: Value = client.post("iscsi/group", &body).await?;
    settled(client, name).await
}

/// What to change about a group.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GroupChange {
    /// A new name.
    pub name: Option<String>,
    /// A new description.
    pub desc: Option<String>,
    /// Initiators to add.
    pub add: Vec<String>,
    /// Initiators to remove.
    pub remove: Vec<String>,
}

/// Change a group.
///
/// # Errors
///
/// Returns [`UgosError::NeedsForce`] when initiators change on a group that
/// LUNs are bound to.
pub async fn edit(
    client: &UgosClient,
    name_or_id: &str,
    change: &GroupChange,
    force: bool,
) -> Result<Group> {
    let group = find(client, name_or_id).await?;
    let name = change.name.clone().unwrap_or_else(|| group.name.clone());
    let desc = change.desc.clone().unwrap_or_else(|| group.desc.clone());
    check::name(&name)?;
    check::description(&desc)?;
    let mut iqns = group.initiator_iqns.clone();
    iqns.retain(|i| !change.remove.iter().any(|r| r.eq_ignore_ascii_case(i)));
    iqns.extend(change.add.iter().cloned());
    let changed = iqns != group.initiator_iqns;
    if changed {
        check_iqns(client, &iqns, Some(&group.name)).await?;
        let luns = bound_luns(client, group.id).await?;
        if !luns.is_empty() && !force {
            return Err(UgosError::NeedsForce(format!(
                "the group is bound to the LUNs {}; changed initiators lose or gain access to them",
                luns.join(", ")
            )));
        }
    }
    let body = json!({ "name": name, "desc": desc, "iqns": iqns });
    let _: Value = client
        .put(&format!("iscsi/group?id={}", group.id), &body)
        .await?;
    let after = settled(client, &name).await?;
    if after.initiator_iqns.len() != iqns.len() || after.desc != desc {
        return Err(refused("UGOS said yes but the group did not change"));
    }
    Ok(after)
}

/// Delete a group.
///
/// # Errors
///
/// Returns [`UgosError::NeedsForce`] when LUNs are bound to it.
pub async fn delete(client: &UgosClient, name_or_id: &str, force: bool) -> Result<()> {
    let group = find(client, name_or_id).await?;
    let luns = bound_luns(client, group.id).await?;
    if !luns.is_empty() && !force {
        return Err(UgosError::NeedsForce(format!(
            "the group is bound to the LUNs {}, which lose it as an initiator group",
            luns.join(", ")
        )));
    }
    let body = json!({ "group_ids": [group.id], "exclude": false });
    let _: Value = client.delete_with_body("iscsi/group", &body).await?;
    let id = group.id;
    let listed = || async move { Ok(list(client).await?.iter().any(|g| g.id == id)) };
    if !super::gone(listed).await? {
        return Err(refused("UGOS said yes but the group is still there"));
    }
    Ok(())
}
