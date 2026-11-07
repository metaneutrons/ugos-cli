//! Discovery services and Time Machine: `file/advanced/config`.
//!
//! The web UI posts all of it at once, with every share and a flag for the
//! ones that are Time Machine targets, and has no stop. Its confirmations
//! become `force` here.

use serde_json::{Value, json};

use super::{Service, config, refused};
use crate::client::UgosClient;
use crate::error::{Result, UgosError};
use crate::types::shares::SmbAdvanced;

/// The advanced configuration.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn get(client: &UgosClient) -> Result<SmbAdvanced> {
    client.get("file/advanced/config").await
}

/// What to change. `None` leaves a setting alone.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AdvancedChange {
    /// WS-Discovery, for Windows.
    pub wsdd2: Option<bool>,
    /// Bonjour, for Macs.
    pub bonjour: Option<bool>,
    /// `UPnP` announcements.
    pub upnp: Option<bool>,
    /// The shares that are Time Machine targets, by name; an empty list
    /// switches Time Machine off.
    pub time_machine: Option<Vec<String>>,
}

/// More Time Machine targets than this make the web UI ask first.
const SOFT_LIMIT: usize = 5;

fn targets(shares: &SmbAdvanced) -> usize {
    shares.shares.iter().filter(|s| s.time_backup).count()
}

/// The change as a copy of the current settings.
fn edited(current: &SmbAdvanced, change: &AdvancedChange) -> Result<SmbAdvanced> {
    let mut next = current.clone();
    next.wsdd2 = change.wsdd2.unwrap_or(next.wsdd2);
    next.bonjour = change.bonjour.unwrap_or(next.bonjour);
    next.upnp = change.upnp.unwrap_or(next.upnp);
    if let Some(names) = &change.time_machine {
        for share in &mut next.shares {
            share.time_backup = false;
        }
        for name in names {
            let share = next
                .shares
                .iter_mut()
                .find(|s| s.name.eq_ignore_ascii_case(name))
                .ok_or_else(|| refused(format!("there is no share {name:?}")))?;
            share.time_backup = true;
        }
    }
    Ok(next)
}

/// What the web UI asks a confirmation for.
fn check_risks(current: &SmbAdvanced, next: &SmbAdvanced, smb_on: bool) -> Result<()> {
    let count = targets(next);
    if count > SOFT_LIMIT && count > targets(current) {
        return Err(UgosError::NeedsForce(format!(
            "{count} Time Machine targets are more than {SOFT_LIMIT}, which UGOS advises against"
        )));
    }
    let discovery_on = (next.bonjour && !current.bonjour) || (next.wsdd2 && !current.wsdd2);
    if discovery_on && !smb_on {
        return Err(UgosError::NeedsForce(
            "Bonjour and WS-Discovery announce SMB shares, and SMB is off".into(),
        ));
    }
    Ok(())
}

/// The body of `file/advanced/start`.
fn body(next: &SmbAdvanced) -> Value {
    json!({
        "enableWsdd2Service": next.wsdd2,
        "enableBonjourService": next.bonjour,
        "enableUPnPService": next.upnp,
        "enableTimeMachine": targets(next) > 0,
        "shareFolders": next.shares.iter().map(|s| json!({
            "fileId": s.id, "fileName": s.name, "select": s.time_backup,
        })).collect::<Vec<Value>>(),
    })
}

/// Apply a change and read it back.
///
/// # Errors
///
/// Returns [`UgosError::NeedsForce`] where the web UI asks a confirmation:
/// discovery services switched on while SMB is off, and more than five
/// Time Machine targets.
pub async fn apply(
    client: &UgosClient,
    change: &AdvancedChange,
    force: bool,
) -> Result<SmbAdvanced> {
    let current = get(client).await?;
    let next = edited(&current, change)?;
    if !force {
        let smb_on = Service::Smb.running(&config(client, Service::Smb).await?);
        check_risks(&current, &next, smb_on)?;
    }
    let _: Value = client.post("file/advanced/start", &body(&next)).await?;
    let after = get(client).await?;
    if after.wsdd2 != next.wsdd2
        || after.bonjour != next.bonjour
        || after.upnp != next.upnp
        || targets(&after) != targets(&next)
    {
        return Err(refused("UGOS said yes but the settings did not change"));
    }
    Ok(after)
}
