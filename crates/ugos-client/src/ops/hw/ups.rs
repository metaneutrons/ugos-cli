//! The UPS: USB, SNMP or a network UPS (NUT client).
//!
//! The web UI picks the call from the old and new setup: it stops a running
//! monitoring first when the kind changes or for SNMP, then starts or applies.
//! This follows the same decision.

use serde_json::{Value, json};

use crate::client::UgosClient;
use crate::error::{Result, UgosError};
use crate::types::hw::Ups;

fn refused(message: impl Into<String>) -> UgosError {
    UgosError::Refused(message.into())
}

/// How long to run on battery before acting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wait {
    /// Until the battery is low.
    BatteryLow,
    /// This long: a number and a unit, `1` minutes (1 to 1200) or `2` hours
    /// (1 to 20).
    Custom(i64, i64),
    /// Act at once.
    Immediately,
}

/// What kind of UPS and how it reaches the NAS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// A UPS on a USB port. `nut_server` shares it with other NAS.
    Usb {
        /// Serve the UPS over NUT.
        nut_server: bool,
    },
    /// A UPS read by SNMP.
    Snmp {
        /// Address, IPv4 or IPv6.
        ip: String,
        /// Community.
        community: String,
        /// Version, `1` or `2`.
        version: i64,
        /// MIB, `auto` by default.
        mib: String,
    },
    /// A UPS that another NAS serves over NUT.
    Network {
        /// Address of the NUT master.
        master_ip: String,
    },
}

/// What to set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpsSetting {
    /// The kind of UPS.
    pub source: Source,
    /// `0` go to standby, `1` shut down.
    pub protect_type: i64,
    /// When to act.
    pub wait: Wait,
}

/// The UPS setting.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn ups(client: &UgosClient) -> Result<Ups> {
    client.get("hardware/ups/config").await
}

fn check(setting: &UpsSetting) -> Result<()> {
    if ![0, 1].contains(&setting.protect_type) {
        return Err(refused("the UPS action is 0 (standby) or 1 (shut down)"));
    }
    match (&setting.wait, &setting.source) {
        (Wait::Custom(n, 1), _) if (1..=1200).contains(n) => {}
        (Wait::Custom(n, 2), _) if (1..=20).contains(n) => {}
        (Wait::Custom(..), _) => {
            return Err(refused("a wait is 1 to 1200 minutes or 1 to 20 hours"));
        }
        (Wait::Immediately, Source::Snmp { .. }) => {
            return Err(refused("an SNMP UPS cannot shut the NAS down at once"));
        }
        (Wait::BatteryLow, Source::Network { .. }) => {
            return Err(refused("a network UPS waits a set time or acts at once"));
        }
        _ => {}
    }
    match &setting.source {
        Source::Snmp {
            ip,
            community,
            version,
            ..
        } => {
            if ip.is_empty() || community.is_empty() || ![1, 2].contains(version) {
                return Err(refused(
                    "SNMP needs an address, a community and a version of 1 or 2",
                ));
            }
        }
        Source::Network { master_ip } => {
            let _ = crate::ops::net::validate::ipv4(master_ip, "the NUT master")?;
        }
        Source::Usb { .. } => {}
    }
    Ok(())
}

/// The body of `ups/start` and `ups/apply`, masked by kind as the web UI does.
#[must_use]
pub fn body(setting: &UpsSetting) -> Value {
    let (standby_time, unit, right_now) = match setting.wait {
        Wait::BatteryLow => (-1, 1, false),
        Wait::Custom(n, unit) => (n, unit, false),
        Wait::Immediately => (-1, 1, true),
    };
    let (kind, nut, ip, community, version, mib, master) = match &setting.source {
        Source::Usb { nut_server } => (0, *nut_server, "", "", 1, "", ""),
        Source::Snmp {
            ip,
            community,
            version,
            mib,
        } => (
            1,
            false,
            ip.as_str(),
            community.as_str(),
            *version,
            mib.as_str(),
            "",
        ),
        Source::Network { master_ip } => (2, false, "", "", 1, "", master_ip.as_str()),
    };
    json!({
        "type": kind, "snmp_ip": ip, "snmp_mib": mib, "snmp_ver": version,
        "snmp_community": community, "ups_master_ip": master,
        "nut_server_enable": nut, "right_now": right_now,
        "protect_type": setting.protect_type, "standby_time": standby_time,
        "standby_time_unit": unit,
    })
}

/// The body that connects a USB UPS with the defaults.
fn usb_defaults() -> Value {
    body(&UpsSetting {
        source: Source::Usb { nut_server: false },
        protect_type: 0,
        wait: Wait::BatteryLow,
    })
}

/// Disconnect the UPS.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn disconnect(client: &UgosClient) -> Result<()> {
    let _: Value = client.post("hardware/ups/stop", &Value::Null).await?;
    Ok(())
}

const fn kind_of(source: &Source) -> i64 {
    match source {
        Source::Usb { .. } => 0,
        Source::Snmp { .. } => 1,
        Source::Network { .. } => 2,
    }
}

/// Whether a network UPS answers at this address.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] when it does not.
pub async fn check_master(client: &UgosClient, ip: &str) -> Result<()> {
    let reply: Value = client
        .get_with_params("hardware/ups/connection/check", &[("ups_master_ip", ip)])
        .await
        .map_err(|_| refused(format!("no NUT server answers at {ip}")))?;
    if reply
        .get("result")
        .is_some_and(|v| v.as_bool().unwrap_or_else(|| v.as_i64().unwrap_or(0) != 0))
    {
        Ok(())
    } else {
        Err(refused(format!("no NUT server answers at {ip}")))
    }
}

/// Set the UPS as the web UI would, and read the setting back.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] for input the web UI would refuse, a
/// network UPS that does not answer, or a change UGOS did not keep.
pub async fn set_ups(client: &UgosClient, setting: &UpsSetting) -> Result<Ups> {
    check(setting)?;
    if let Source::Network { master_ip } = &setting.source {
        check_master(client, master_ip).await?;
    }
    let current = ups(client).await?;
    let kind = kind_of(&setting.source);
    let snmp = kind == 1;
    if current.status && (snmp || current.kind != kind) {
        disconnect(client).await?;
    }
    let same_usb = kind == 0 && current.kind == 0;
    let (path, request) = if snmp || (kind == 0 && !same_usb) {
        (
            "hardware/ups/start",
            if snmp { body(setting) } else { usb_defaults() },
        )
    } else {
        ("hardware/ups/apply", body(setting))
    };
    let _: Value = client.post(path, &request).await?;
    let now = ups(client).await?;
    if now.kind == kind {
        Ok(now)
    } else {
        Err(refused(
            "UGOS accepted the request but the UPS kind did not change",
        ))
    }
}

/// The MIBs an SNMP UPS can use.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn mibs(client: &UgosClient) -> Result<Vec<String>> {
    let reply: Value = client.get("hardware/ups/snmp/mibs").await?;
    Ok(reply
        .get("result")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default())
}

/// The UPS firmware state: version, update progress and the last failure.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn firmware(client: &UgosClient) -> Result<Value> {
    client.get("hardware/ups/firmware/info").await
}
