//! The global iSCSI setting: port, IQN prefix and default CHAP.

use serde_json::{Value, json};

use super::{check, refused};
use crate::client::UgosClient;
use crate::error::{Result, UgosError};
use crate::types::san::Setting;

/// The setting.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn get(client: &UgosClient) -> Result<Setting> {
    client.get("iscsi/setting").await
}

/// CHAP credentials, one-way or mutual.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Chap {
    /// None.
    Off,
    /// One-way: the initiator proves itself.
    OneWay {
        /// User.
        user: String,
        /// Password, 12 to 16 letters and digits.
        password: String,
    },
    /// Mutual: the target proves itself as well.
    Mutual {
        /// User.
        user: String,
        /// Password.
        password: String,
        /// The target's user.
        mutual_user: String,
        /// The target's password, different from the first.
        mutual_password: String,
    },
}

/// What to change about the setting.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SettingChange {
    /// The port, 1001 to 65535.
    pub port: Option<i64>,
    /// The IQN prefix.
    pub iqn_prefix: Option<String>,
    /// The default CHAP.
    pub chap: Option<Chap>,
}

/// Check a CHAP choice and seal its passwords into the `chap_info` body.
pub(super) async fn chap_body(client: &UgosClient, chap: &Chap, extra: &Value) -> Result<Value> {
    let mut body = match chap {
        Chap::Off => json!({ "type": 0 }),
        Chap::OneWay { user, password } => {
            check::chap_user(user)?;
            check::chap_password(password)?;
            json!({ "type": 1, "userid": user, "password": client.seal_password(password).await? })
        }
        Chap::Mutual {
            user,
            password,
            mutual_user,
            mutual_password,
        } => {
            check::chap_user(user)?;
            check::chap_user(mutual_user)?;
            check::chap_password(password)?;
            check::chap_password(mutual_password)?;
            if password == mutual_password {
                return Err(refused("the two CHAP passwords must differ"));
            }
            json!({
                "type": 2, "userid": user, "password": client.seal_password(password).await?,
                "mutual_userid": mutual_user,
                "mutual_password": client.seal_password(mutual_password).await?,
            })
        }
    };
    if let (Some(map), Some(more)) = (body.as_object_mut(), extra.as_object()) {
        map.extend(more.clone());
    }
    Ok(body)
}

/// Change the setting. A new port restarts the iSCSI service and new
/// credentials affect connected initiators, so both need `force`.
///
/// # Errors
///
/// Returns [`UgosError::NeedsForce`] without `force`.
pub async fn set(client: &UgosClient, change: &SettingChange, force: bool) -> Result<Setting> {
    let current = get(client).await?;
    let port = change.port.unwrap_or(current.port);
    if !(1001..=65535).contains(&port) {
        return Err(refused("the port is 1001 to 65535"));
    }
    let prefix = match &change.iqn_prefix {
        Some(p) => check::iqn_prefix(p)?,
        None => current.iqn_prefix.clone(),
    };
    if port != current.port {
        let check: Value = client
            .get_with_params("iscsi/setting/portCheck", &[("port", &port.to_string())])
            .await?;
        if check.get("is_available").and_then(Value::as_bool) != Some(true) {
            return Err(refused(format!("port {port} is taken or reserved")));
        }
    }
    if !force && (port != current.port || change.chap.is_some()) {
        return Err(UgosError::NeedsForce(
            "a new port restarts the iSCSI service, and new credentials affect connected initiators"
                .into(),
        ));
    }
    let chap = match &change.chap {
        Some(c) => chap_body(client, c, &json!({})).await?,
        None => json!({ "type": current.kind }),
    };
    let body = json!({ "port": port, "iqn_prefix": prefix, "chap_info": chap });
    let _: Value = client.post("iscsi/setting", &body).await?;
    let after = get(client).await?;
    if after.port != port || after.iqn_prefix != prefix {
        return Err(refused("UGOS said yes but the setting did not change"));
    }
    Ok(after)
}
