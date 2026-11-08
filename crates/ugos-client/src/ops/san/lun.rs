//! LUNs.

use serde_json::{Value, json};

use super::{check, everything, group, refused};
use crate::api::storage::StorageApi;
use crate::client::UgosClient;
use crate::error::{Result, UgosError};
use crate::types::san::{GroupAccess, Lun};
use crate::types::storage::StorageVolume;

const GIB: i64 = 1 << 30;

/// All LUNs.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn list(client: &UgosClient) -> Result<Vec<Lun>> {
    everything(client, "iscsi/lun", &[]).await
}

/// A LUN by name or id.
///
/// # Errors
///
/// Refuses a name that no LUN has.
pub async fn find(client: &UgosClient, name_or_id: &str) -> Result<Lun> {
    list(client)
        .await?
        .into_iter()
        .find(|l| l.name == name_or_id || l.id.to_string() == name_or_id)
        .ok_or_else(|| refused(format!("there is no LUN {name_or_id:?}")))
}

/// Whether a status is a job in progress, or damage.
const fn blocked(status: i64) -> bool {
    matches!(status, 1 | 5..=8)
}

async fn volume(client: &UgosClient, name_or_label: &str) -> Result<StorageVolume> {
    let all = client.volumes().await?;
    let found = all
        .into_iter()
        .find(|v| v.name == name_or_label || v.label == name_or_label)
        .ok_or_else(|| refused(format!("there is no volume {name_or_label:?}")))?;
    if !found.ready_to_use || found.available < GIB {
        return Err(refused(format!(
            "the volume {} is not ready or has less than 1 GiB free",
            found.label
        )));
    }
    Ok(found)
}

/// A new LUN.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LunSpec {
    /// Name.
    pub name: String,
    /// Description.
    pub desc: String,
    /// Size in whole `unit`s.
    pub size: i64,
    /// `GB` or `TB`.
    pub unit: &'static str,
    /// The volume's name or label.
    pub volume: String,
    /// Thin: space is taken as it is written.
    pub thin: bool,
}

/// Make a LUN.
///
/// # Errors
///
/// Returns [`UgosError::NeedsForce`] for a thin LUN larger than the free
/// space, which can run the volume full.
pub async fn create(client: &UgosClient, spec: &LunSpec, force: bool) -> Result<Lun> {
    check::name(&spec.name)?;
    check::description(&spec.desc)?;
    if list(client).await?.iter().any(|l| l.name == spec.name) {
        return Err(refused(format!("a LUN {:?} exists already", spec.name)));
    }
    let vol = volume(client, &spec.volume).await?;
    let bytes = check::bytes(spec.size, spec.unit);
    if bytes > vol.available {
        if !spec.thin {
            return Err(refused("a thick LUN cannot be larger than the free space"));
        }
        if !force {
            return Err(UgosError::NeedsForce(
                "the thin LUN is larger than the free space; the volume can run full".into(),
            ));
        }
    }
    let body = json!({
        "name": spec.name, "desc": spec.desc, "size": spec.size, "size_type": spec.unit,
        "volume_name": vol.name, "type": if spec.thin { "thin" } else { "thick" },
    });
    let _: Value = client.post("iscsi/lun", &body).await?;
    find(client, &spec.name).await
}

/// Who may use a LUN.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Access {
    /// Every group.
    All,
    /// Only these groups, with `rw` or `ro`.
    Groups(Vec<(String, String)>),
}

/// What to change about a LUN.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LunChange {
    /// A new name.
    pub name: Option<String>,
    /// A new description.
    pub desc: Option<String>,
    /// A larger size, in whole units.
    pub size: Option<(i64, &'static str)>,
    /// Force unit access.
    pub fua: Option<bool>,
    /// Who may use it.
    pub access: Option<Access>,
}

async fn group_access(
    client: &UgosClient,
    groups: &[(String, String)],
) -> Result<Vec<GroupAccess>> {
    let mut out = Vec::new();
    for (name, mode) in groups {
        if mode != "rw" && mode != "ro" {
            return Err(refused("a group gets rw or ro"));
        }
        let g = group::find(client, name).await?;
        out.push(GroupAccess {
            group_id: g.id,
            group_name: g.name,
            access_mode: mode.clone(),
            desc: g.desc,
        });
    }
    if out.is_empty() {
        return Err(refused("custom access names at least one group"));
    }
    Ok(out)
}

/// The size a LUN would get, checked: it only grows, and a thick LUN only by
/// what the volume has free.
async fn grown(client: &UgosClient, lun: &Lun, size: (i64, &'static str)) -> Result<i64> {
    let bytes = check::bytes(size.0, size.1);
    if bytes < lun.size {
        return Err(refused("a LUN cannot be made smaller"));
    }
    if lun.kind == "thick" && bytes - lun.size > volume(client, &lun.volume_name).await?.available {
        return Err(refused("the volume has not enough free space for that"));
    }
    Ok(bytes)
}

/// Change a LUN. Growing it cannot be undone and a new access rule changes
/// what initiators see, so both need `force`.
///
/// # Errors
///
/// Returns [`UgosError::NeedsForce`] without `force`.
pub async fn edit(
    client: &UgosClient,
    name_or_id: &str,
    change: &LunChange,
    force: bool,
) -> Result<Lun> {
    let lun = find(client, name_or_id).await?;
    if blocked(lun.status) {
        return Err(refused("the LUN is busy or damaged: wait for it"));
    }
    let name = change.name.clone().unwrap_or_else(|| lun.name.clone());
    let desc = change.desc.clone().unwrap_or_else(|| lun.desc.clone());
    check::name(&name)?;
    check::description(&desc)?;
    let size = change.size.unwrap_or_else(|| shown_size(&lun));
    if change.size.is_some() {
        let _ = grown(client, &lun, size).await?;
    }
    if (change.size.is_some() || change.access.is_some()) && !force {
        return Err(UgosError::NeedsForce(
            "a larger LUN cannot be made smaller again, and a new access rule changes what initiators see"
                .into(),
        ));
    }
    let (access_type, groups) = match &change.access {
        Some(Access::All) => ("default", Vec::new()),
        Some(Access::Groups(g)) => ("custom", group_access(client, g).await?),
        None => (lun.access_type.as_str(), lun.group_access.clone()),
    };
    let access_type = if access_type.is_empty() {
        "default"
    } else {
        access_type
    };
    let body = json!({
        "name": name, "desc": desc, "size": size.0, "size_type": size.1,
        "enable_fua": change.fua.unwrap_or(lun.enable_fua),
        "access_type": access_type, "group_access": groups,
    });
    let _: Value = client
        .put(&format!("iscsi/lun?id={}", lun.id), &body)
        .await?;
    find(client, &name).await
}

/// The size of a LUN in whole units of the unit it was made in.
fn shown_size(lun: &Lun) -> (i64, &'static str) {
    let unit: &'static str = if lun.size_type == "TB" { "TB" } else { "GB" };
    let per = check::bytes(1, unit);
    (lun.size / per, unit)
}

/// Delete a LUN and the data on it. UGOS asks for the account's password,
/// which this client sends sealed.
///
/// # Errors
///
/// Returns [`UgosError::NeedsForce`] without `force`.
pub async fn delete(client: &UgosClient, name_or_id: &str, force: bool) -> Result<()> {
    let lun = find(client, name_or_id).await?;
    if blocked(lun.status) && lun.status != 1 {
        return Err(refused("the LUN is busy: wait for it"));
    }
    if !force {
        let mapped = lun.targets.first().map_or(String::new(), |t| {
            format!(" and is mapped to {}", t.target_name)
        });
        return Err(UgosError::NeedsForce(format!(
            "deleting the LUN {} destroys its data{mapped}",
            lun.name
        )));
    }
    let body = json!({ "lun_ids": [lun.id], "exclude": false });
    let _: Value = client
        .delete_with_body_confirmed("iscsi/lun", &body)
        .await?;
    let id = lun.id;
    let listed = || async move { Ok(list(client).await?.iter().any(|l| l.id == id)) };
    if !super::gone(listed).await? {
        return Err(refused("UGOS said yes but the LUN is still there"));
    }
    Ok(())
}
