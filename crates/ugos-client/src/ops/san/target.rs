//! iSCSI targets.

use serde_json::{Value, json};

use super::setting::{Chap, chap_body};
use super::{check, everything, lun, refused, setting};
use crate::client::UgosClient;
use crate::error::{Result, UgosError};
use crate::types::san::{Connection, Connections, Target};

/// All targets.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn list(client: &UgosClient) -> Result<Vec<Target>> {
    everything(
        client,
        "iscsi/target",
        &[("name", ""), ("sort_by", ""), ("order", "")],
    )
    .await
}

/// A target by name or id.
///
/// # Errors
///
/// Refuses a name that no target has.
pub async fn find(client: &UgosClient, name_or_id: &str) -> Result<Target> {
    list(client)
        .await?
        .into_iter()
        .find(|t| t.name == name_or_id || t.id.to_string() == name_or_id)
        .ok_or_else(|| refused(format!("there is no target {name_or_id:?}")))
}

/// The next free name and IQN.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn predict(client: &UgosClient) -> Result<(String, String)> {
    let v: Value = client.get("iscsi/target/predict").await?;
    let text = |k: &str| {
        v.get(k)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    Ok((text("name"), text("iqn")))
}

/// The initiators connected to a target, or to all of them.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn connections(client: &UgosClient, target_name: &str) -> Result<Vec<Connection>> {
    let answer: Connections = client
        .get_with_params("iscsi/target/conn", &[("target_name", target_name)])
        .await?;
    Ok(answer.results)
}

/// How a target is protected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Protection {
    /// No CHAP.
    None,
    /// The default CHAP of the setting.
    Default,
    /// Credentials of its own.
    Custom(Chap),
}

/// A new target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetSpec {
    /// Name.
    pub name: String,
    /// The IQN, or the predicted one.
    pub iqn: Option<String>,
    /// The LUN to map, by name or id.
    pub lun: String,
    /// CHAP.
    pub protection: Protection,
}

async fn protection_body(client: &UgosClient, p: &Protection) -> Result<(&'static str, Value)> {
    match p {
        Protection::None => Ok(("close", json!({ "type": 0 }))),
        Protection::Default => {
            if setting::get(client).await?.kind == 0 {
                return Err(refused("the default CHAP is off in the setting"));
            }
            Ok(("default", json!({ "type": 0 })))
        }
        Protection::Custom(chap) => Ok(("custom", chap_body(client, chap, &json!({})).await?)),
    }
}

async fn free(client: &UgosClient, key: &str, value: &str) -> Result<bool> {
    let answer: Value = client
        .get_with_params("iscsi/target/exist", &[(key, value)])
        .await?;
    let field = if key == "target_name" {
        "name_exist"
    } else {
        "iqn_exist"
    };
    Ok(answer.get(field).and_then(Value::as_bool) != Some(true))
}

/// Make a target and map a LUN to it.
///
/// # Errors
///
/// Refuses what the web UI refuses, such as a LUN that is mapped already.
pub async fn create(client: &UgosClient, spec: &TargetSpec) -> Result<Target> {
    check::name(&spec.name)?;
    let iqn = match &spec.iqn {
        Some(i) => i.clone(),
        None => predict(client).await?.1,
    };
    check::target_iqn(&iqn)?;
    if !free(client, "target_name", &spec.name).await? {
        return Err(refused(format!("a target {:?} exists already", spec.name)));
    }
    if !free(client, "target_iqn", &iqn).await? {
        return Err(refused(format!("the IQN {iqn} is in use already")));
    }
    let l = lun::find(client, &spec.lun).await?;
    if !l.targets.is_empty() || !matches!(l.status, 0 | 3 | 4) {
        return Err(refused(format!(
            "the LUN {} is mapped already or not ready",
            l.name
        )));
    }
    let (chap_type, chap_info) = protection_body(client, &spec.protection).await?;
    let body = json!({
        "name": spec.name, "iqn": iqn, "lun_id": l.id,
        "chap_type": chap_type, "chap_info": chap_info,
    });
    let _: Value = client.post("iscsi/target", &body).await?;
    find(client, &spec.name).await
}

/// What to change about a target.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TargetChange {
    /// A new name.
    pub name: Option<String>,
    /// A new IQN.
    pub iqn: Option<String>,
    /// Several connections per initiator.
    pub multi_conn: Option<bool>,
    /// New protection.
    pub protection: Option<Protection>,
    /// Header and data digest.
    pub crc: Option<(bool, bool)>,
    /// Apply CHAP and digest changes to the connected initiators now, which
    /// drops their sessions, instead of at their next connection.
    pub apply_now: bool,
}

/// Change a target. A new IQN, CHAP or digest changes what connected
/// initiators see, so it needs `force`.
///
/// # Errors
///
/// Returns [`UgosError::NeedsForce`] without `force`.
pub async fn edit(
    client: &UgosClient,
    name_or_id: &str,
    change: &TargetChange,
    force: bool,
) -> Result<Target> {
    let t = find(client, name_or_id).await?;
    if (3..=7).contains(&t.status) {
        return Err(refused("the target is busy: wait for it"));
    }
    let name = change.name.clone().unwrap_or_else(|| t.name.clone());
    let iqn = change.iqn.clone().unwrap_or_else(|| t.iqn.clone());
    check::name(&name)?;
    check::target_iqn(&iqn)?;
    let risky = iqn != t.iqn || change.protection.is_some() || change.crc.is_some();
    if risky && !force && !connections(client, &t.name).await?.is_empty() {
        return Err(UgosError::NeedsForce(
            "initiators are connected, and a new IQN, CHAP or digest changes what they see".into(),
        ));
    }
    let (chap_type, chap_info) = match &change.protection {
        Some(p) => protection_body(client, p).await?,
        None => (leak(&t.chap_type), json!({ "type": t.chap_info.kind })),
    };
    let (head, data) = change.crc.unwrap_or((t.enable_head_crc, t.enable_data_crc));
    let mut chap_info = chap_info;
    chap_info["apply_now"] = json!(change.apply_now);
    let body = json!({
        "name": name, "iqn": iqn, "multi_conn": change.multi_conn.unwrap_or(t.multi_conn),
        "chap_type": chap_type, "chap_info": chap_info,
        "crc_info": { "enable_head_crc": head, "enable_data_crc": data, "apply_now": change.apply_now },
    });
    let _: Value = client
        .put(&format!("iscsi/target?id={}", t.id), &body)
        .await?;
    find(client, &name).await
}

/// The three values of `chap_type` as static text.
fn leak(chap_type: &str) -> &'static str {
    match chap_type {
        "default" => "default",
        "custom" => "custom",
        _ => "close",
    }
}

/// Switch a target on.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn enable(client: &UgosClient, name_or_id: &str) -> Result<Target> {
    let t = find(client, name_or_id).await?;
    let _: Value = client
        .post(&format!("iscsi/target/enable?id={}", t.id), &json!({}))
        .await?;
    find(client, &t.name).await
}

/// Switch a target off. Connected initiators lose it.
///
/// # Errors
///
/// Returns [`UgosError::NeedsForce`] when initiators are connected.
pub async fn disable(client: &UgosClient, name_or_id: &str, force: bool) -> Result<Target> {
    let t = find(client, name_or_id).await?;
    if !force && !connections(client, &t.name).await?.is_empty() {
        return Err(UgosError::NeedsForce(
            "initiators are connected, and lose the target".into(),
        ));
    }
    let _: Value = client
        .post(&format!("iscsi/target/disable?id={}", t.id), &json!({}))
        .await?;
    find(client, &t.name).await
}

/// Delete a target, and with `with_lun` the LUN mapped to it and its data.
/// That asks for the account's password, which this client sends sealed.
///
/// # Errors
///
/// Returns [`UgosError::NeedsForce`] when initiators are connected or the LUN
/// goes with it.
pub async fn delete(
    client: &UgosClient,
    name_or_id: &str,
    with_lun: bool,
    force: bool,
) -> Result<()> {
    let t = find(client, name_or_id).await?;
    if !force {
        if with_lun {
            return Err(UgosError::NeedsForce(format!(
                "deleting the target {} also destroys its LUN and the data on it",
                t.name
            )));
        }
        if !connections(client, &t.name).await?.is_empty() {
            return Err(UgosError::NeedsForce("initiators are connected".into()));
        }
    }
    let body = json!({ "target_ids": [t.id], "exclude": false });
    if with_lun {
        let _: Value = client
            .post_confirmed("iscsi/target/deleteWithLun", &body)
            .await?;
    } else {
        let _: Value = client.delete_with_body("iscsi/target", &body).await?;
    }
    let id = t.id;
    let listed = || async move { Ok(list(client).await?.iter().any(|x| x.id == id)) };
    if !super::gone(listed).await? {
        return Err(refused("UGOS said yes but the target is still there"));
    }
    Ok(())
}
