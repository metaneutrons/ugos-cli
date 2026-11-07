//! Bonds and bridges.
//!
//! Joining or leaving ports re-creates the interface, which can move the
//! address this session talks to, so these ask first when a port carries it.
//! UGOS does the work in the background, and the status calls say when it
//! is done.

use std::time::Duration;

use serde_json::json;

use super::validate;
use crate::api::net::{NetApi, NetFlowApi, NetLinksApi, NetWriteApi};
use crate::client::UgosClient;
use crate::error::{Result, UgosError};
use crate::types::net::{Iface, NewAddress};

/// Bond modes the web UI offers.
pub const BOND_MODES: [&str; 5] = [
    "balance-alb",
    "active-backup",
    "balance-rr",
    "balance-xor",
    "802.3ad",
];

/// The IPv4 setup of a new bond.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BondIpv4 {
    /// `dhcp` or `static`.
    pub protocol: String,
    /// Static address.
    pub address: String,
    /// Static netmask.
    pub netmask: String,
    /// Static gateway.
    pub gateway: String,
    /// Primary and alternate DNS server; either may be empty.
    pub dns: [String; 2],
    /// Make it the default route.
    pub route: bool,
}

fn refused(message: impl Into<String>) -> UgosError {
    UgosError::Refused(message.into())
}

/// Whether any of these interfaces carries an address this session uses.
async fn session_on(client: &UgosClient, names: &[String]) -> Result<bool> {
    let session = client.session_addresses().await;
    let all = client.net_ifaces().await?;
    Ok(all
        .iter()
        .filter(|i: &&Iface| names.contains(&i.interface))
        .flat_map(|i| {
            std::iter::once(i.ipv4.ipaddr.clone()).chain(
                i.ipv6
                    .ip6addr
                    .iter()
                    .map(|a| a.split('/').next().unwrap_or_default().to_owned()),
            )
        })
        .filter_map(|text| text.parse::<std::net::IpAddr>().ok())
        .any(|ip| session.contains(&ip)))
}

/// The body of `network/bond/create`.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] for a mode or an address the web UI would
/// refuse.
pub fn bond_body(slaves: &[String], mode: &str, ipv4: &BondIpv4) -> Result<serde_json::Value> {
    if !BOND_MODES.contains(&mode) {
        return Err(refused(format!(
            "the bond mode '{mode}' is none of {}",
            BOND_MODES.join(", ")
        )));
    }
    if ipv4.protocol == "static" {
        validate::static_ipv4(&ipv4.address, &ipv4.netmask, Some(&ipv4.gateway))?;
        if ipv4.gateway.is_empty() {
            return Err(refused("a static address needs a gateway"));
        }
    } else if ipv4.protocol != "dhcp" {
        return Err(refused("the IPv4 protocol is dhcp or static"));
    }
    for server in ipv4.dns.iter().filter(|s| !s.is_empty()) {
        let _ = validate::dns_server(server)?;
    }
    Ok(json!({
        "slaves": slaves, "bond_mode": mode,
        "ipv4": {
            "protocol": ipv4.protocol, "ipaddr": ipv4.address, "netmask": ipv4.netmask,
            "gateway": ipv4.gateway, "dns": ipv4.dns, "route": i64::from(ipv4.route),
        },
    }))
}

/// Wait for a bond or bridge job. UGOS answers `0` or `1` while it works.
async fn wait(client: &UgosClient, bond: bool) -> Result<()> {
    for _ in 0..20 {
        tokio::time::sleep(Duration::from_secs(3)).await;
        let status = if bond {
            client.net_bond_status().await
        } else {
            client.net_bridge_status().await
        };
        match status {
            Ok(2) | Err(_) => return Ok(()),
            Ok(3) => return Err(refused("UGOS reports that the job failed")),
            Ok(_) => {}
        }
    }
    Err(refused("the job did not finish within a minute"))
}

/// Create a bond from these ports.
///
/// # Errors
///
/// Returns [`UgosError::NeedsForce`] when a port carries this session's
/// address or the ports are in different networks and `force` is not set, and
/// [`UgosError::Refused`] for input the web UI would refuse.
pub async fn create_bond(
    client: &UgosClient,
    slaves: &[String],
    mode: &str,
    ipv4: &BondIpv4,
    force: bool,
) -> Result<Vec<NewAddress>> {
    if slaves.len() < 2 {
        return Err(refused("a bond needs two or more ports"));
    }
    let free = client.net_bond_candidates().await?;
    if let Some(bad) = slaves
        .iter()
        .find(|s| !free.iter().any(|f| f.interface == **s))
    {
        return Err(refused(format!("'{bad}' cannot join a bond")));
    }
    let body = bond_body(slaves, mode, ipv4)?;
    if !force {
        if session_on(client, slaves).await? {
            return Err(UgosError::NeedsForce(
                "a port carries the address this session talks through; it moves with the bond"
                    .into(),
            ));
        }
        if client.net_same_network(slaves).await? == json!(false) {
            return Err(UgosError::NeedsForce(
                "the ports are in different networks".into(),
            ));
        }
    }
    let moved = client.net_bond_create(&body).await?;
    wait(client, true).await?;
    Ok(moved)
}

/// Delete a bond. A bond that firewall or traffic rules name is not deleted
/// unless `force` says so.
///
/// # Errors
///
/// Returns [`UgosError::NeedsForce`] for rules that name it, or this
/// session's address, and [`UgosError::Refused`] when the job fails.
pub async fn delete_bond(client: &UgosClient, bond: &str, force: bool) -> Result<Vec<NewAddress>> {
    if !force {
        guard_removal(client, bond).await?;
    }
    let moved = client.net_bond_delete(bond).await?;
    wait(client, true).await?;
    Ok(moved)
}

async fn guard_removal(client: &UgosClient, name: &str) -> Result<()> {
    let ruled = client.net_has_firewall(name).await?
        || client
            .net_flow()
            .await?
            .list
            .iter()
            .any(|g| g.interface == name);
    if ruled {
        return Err(UgosError::NeedsForce(format!(
            "firewall or traffic rules name '{name}' and would go with it"
        )));
    }
    if session_on(client, &[name.to_owned()]).await? {
        return Err(UgosError::NeedsForce(format!(
            "'{name}' carries the address this session talks through"
        )));
    }
    Ok(())
}

/// Create a bridge from these ports. Wi-Fi cannot be bridged.
///
/// # Errors
///
/// Returns [`UgosError::NeedsForce`] when a port carries this session's
/// address and `force` is not set, and [`UgosError::Refused`] for fewer than
/// two ports, a Wi-Fi port or one that cannot join.
pub async fn create_bridge(
    client: &UgosClient,
    slaves: &[String],
    force: bool,
) -> Result<Vec<NewAddress>> {
    if slaves.len() < 2 {
        return Err(refused("a bridge needs two or more ports"));
    }
    if let Some(wifi) = slaves.iter().find(|s| s.starts_with("wlan")) {
        return Err(refused(format!(
            "'{wifi}' is Wi-Fi, which cannot be bridged"
        )));
    }
    let free = client.net_bridge_candidates().await?;
    if let Some(bad) = slaves
        .iter()
        .find(|s| !free.iter().any(|f| f.interface == **s))
    {
        return Err(refused(format!("'{bad}' cannot join a bridge")));
    }
    if !force && session_on(client, slaves).await? {
        return Err(UgosError::NeedsForce(
            "a port carries the address this session talks through; it moves with the bridge"
                .into(),
        ));
    }
    let moved = client.net_bridge_create(slaves).await?;
    wait(client, false).await?;
    Ok(moved)
}

/// Create a virtual bridge, the kind VMs attach to.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] for no port.
pub async fn create_virtual_bridge(
    client: &UgosClient,
    interfaces: &[String],
) -> Result<Vec<NewAddress>> {
    if interfaces.is_empty() {
        return Err(refused("a virtual bridge needs a port"));
    }
    let moved = client.net_bridge_create_virtual(interfaces).await?;
    for _ in 0..20 {
        tokio::time::sleep(Duration::from_secs(3)).await;
        match client.net_bridge_status_virtual().await {
            Ok(s) if s.status == 2 => {
                let failed = s
                    .interfaces
                    .iter()
                    .filter(|i| i.get("status").and_then(serde_json::Value::as_i64) == Some(3))
                    .count();
                return if failed == 0 {
                    Ok(moved)
                } else {
                    Err(refused(format!("{failed} port(s) failed to join")))
                };
            }
            Ok(s) if s.status == 3 => return Err(refused("UGOS reports that the job failed")),
            _ => {}
        }
    }
    Err(refused("the job did not finish within a minute"))
}

/// Delete a bridge.
///
/// # Errors
///
/// Returns [`UgosError::NeedsForce`] for rules that name it, or this
/// session's address, and [`UgosError::Refused`] when the job fails.
pub async fn delete_bridge(
    client: &UgosClient,
    bridge: &str,
    force: bool,
) -> Result<Vec<NewAddress>> {
    if !force {
        guard_removal(client, bridge).await?;
    }
    let moved = client.net_bridge_delete(bridge).await?;
    wait(client, false).await?;
    Ok(moved)
}
