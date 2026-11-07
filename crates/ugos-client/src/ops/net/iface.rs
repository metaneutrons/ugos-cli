//! Changing the settings of one interface.
//!
//! The web UI builds a different body for each kind of interface, and so does
//! this: Ethernet, bond, bridge, Wi-Fi and Thunderbolt. The change is read,
//! applied to a copy of the interface as UGOS reports it, checked against the
//! form rules, sent, and read back.

use std::time::Duration;

use serde_json::{Value, json};

use super::validate;
use crate::api::net::NetApi;
use crate::client::UgosClient;
use crate::error::{Result, UgosError};
use crate::types::net::{Address, Iface, NewAddress};

/// Kinds of interface, as UGOS numbers them.
const ETHERNET: i64 = 0;
const BOND: i64 = 1;
const WIFI: i64 = 2;
const BRIDGE: i64 = 5;
const THUNDERBOLT: i64 = 11;

/// How IPv6 is set up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ipv6Mode {
    /// Obtain an address automatically (`dhcp`).
    Auto,
    /// Switch IPv6 off (`disable`).
    Off,
    /// Use the address given (`static`).
    Static,
}

/// How the MTU is set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mtu {
    /// Let the system choose.
    Auto,
    /// A value in bytes.
    Value(i64),
}

/// What to change about an interface. Whatever is `None` stays as it is.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IfaceChange {
    /// `dhcp`, `static` or, on Thunderbolt, `ipv4ll`.
    pub ipv4_protocol: Option<String>,
    /// Static address.
    pub ipv4_address: Option<String>,
    /// Static netmask.
    pub ipv4_netmask: Option<String>,
    /// Static gateway.
    pub ipv4_gateway: Option<String>,
    /// DNS servers of the interface, `Some(vec![])` to clear them.
    pub ipv4_dns: Option<Vec<String>>,
    /// Make it the default route.
    pub ipv4_route: Option<bool>,
    /// IPv6 setup.
    pub ipv6_mode: Option<Ipv6Mode>,
    /// Static IPv6 address, without the prefix.
    pub ipv6_address: Option<String>,
    /// Static IPv6 prefix length.
    pub ipv6_prefix: Option<i64>,
    /// IPv6 gateway.
    pub ipv6_gateway: Option<String>,
    /// IPv6 DNS servers.
    pub ipv6_dns: Option<Vec<String>>,
    /// Make it the IPv6 default route.
    pub ipv6_route: Option<bool>,
    /// MTU.
    pub mtu: Option<Mtu>,
    /// Bond mode.
    pub bond_mode: Option<String>,
}

impl IfaceChange {
    /// Whether nothing is to change.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// What a change did.
#[derive(Debug, Clone)]
pub struct IfaceOutcome {
    /// The interface as it reads afterwards, `None` when the NAS could not be
    /// reached again, as happens when its address moved.
    pub iface: Option<Iface>,
    /// The addresses UGOS reports after the change.
    pub new_addresses: Vec<NewAddress>,
}

fn route_flag(flag: bool) -> i64 {
    i64::from(flag)
}

/// Apply a change to a copy of the interface.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] for input the web UI would refuse.
pub fn apply(iface: &Iface, change: &IfaceChange) -> Result<Iface> {
    let mut next = iface.clone();
    apply_ipv4(&mut next, change)?;
    apply_ipv6(&mut next, change)?;
    apply_mtu(&mut next, change)?;
    if let Some(mode) = &change.bond_mode {
        if next.kind != BOND {
            return Err(UgosError::Refused("only a bond has a mode".into()));
        }
        next.bond_mode.clone_from(mode);
    }
    Ok(next)
}

fn dns_list(servers: &[String]) -> Result<Vec<String>> {
    servers.iter().map(|s| validate::dns_server(s)).collect()
}

fn apply_ipv4(next: &mut Iface, change: &IfaceChange) -> Result<()> {
    let v4 = &mut next.ipv4;
    if let Some(protocol) = &change.ipv4_protocol {
        let ok = match protocol.as_str() {
            "dhcp" | "static" => true,
            "ipv4ll" => next.kind == THUNDERBOLT,
            _ => false,
        };
        if !ok {
            return Err(UgosError::Refused(format!(
                "the IPv4 protocol '{protocol}' is not dhcp or static (ipv4ll on Thunderbolt)"
            )));
        }
        v4.protocol.clone_from(protocol);
    }
    if let Some(address) = &change.ipv4_address {
        v4.ipaddr.clone_from(address);
    }
    if let Some(mask) = &change.ipv4_netmask {
        v4.netmask.clone_from(mask);
    }
    if let Some(gateway) = &change.ipv4_gateway {
        v4.gateway.clone_from(gateway);
    }
    if let Some(dns) = &change.ipv4_dns {
        v4.dns = dns_list(dns)?;
    }
    if let Some(route) = change.ipv4_route {
        v4.route = route_flag(route);
    }
    if v4.protocol == "static" {
        let needs_gateway = next.kind != THUNDERBOLT;
        let gateway = (!v4.gateway.is_empty()).then_some(v4.gateway.as_str());
        if needs_gateway && gateway.is_none() {
            return Err(UgosError::Refused(
                "a static address needs a gateway".into(),
            ));
        }
        validate::static_ipv4(&v4.ipaddr, &v4.netmask, gateway)?;
    }
    Ok(())
}

fn apply_ipv6(next: &mut Iface, change: &IfaceChange) -> Result<()> {
    let v6 = &mut next.ipv6;
    if let Some(mode) = change.ipv6_mode {
        v6.protocol = match mode {
            Ipv6Mode::Auto => "dhcp",
            Ipv6Mode::Off => "disable",
            Ipv6Mode::Static => "static",
        }
        .into();
    }
    if let Some(dns) = &change.ipv6_dns {
        v6.dns = dns_list(dns)?;
    }
    if let Some(route) = change.ipv6_route {
        v6.route = route_flag(route);
    }
    if v6.protocol != "static" {
        return Ok(());
    }
    let address = change
        .ipv6_address
        .clone()
        .or_else(|| {
            v6.ip6addr
                .first()
                .map(|a| a.split('/').next().unwrap_or_default().to_owned())
        })
        .unwrap_or_default();
    let _ = validate::ipv6(&address, "address")?;
    let prefix =
        validate::prefix(
            change
                .ipv6_prefix
                .unwrap_or(if v6.prefix > 0 { v6.prefix } else { 64 }),
        )?;
    let gateway = change
        .ipv6_gateway
        .clone()
        .unwrap_or_else(|| v6.gateway.clone());
    let _ = validate::ipv6(&gateway, "gateway")?;
    v6.ip6addr = vec![format!("{address}/{prefix}")];
    v6.prefix = prefix;
    v6.gateway = gateway;
    Ok(())
}

fn apply_mtu(next: &mut Iface, change: &IfaceChange) -> Result<()> {
    let Some(mtu) = change.mtu else {
        return Ok(());
    };
    let max = match next.kind {
        ETHERNET => 9000,
        THUNDERBOLT => 62000,
        _ => {
            return Err(UgosError::Refused(
                "this kind of interface has no MTU setting".into(),
            ));
        }
    };
    match mtu {
        Mtu::Auto => next.mtu_manual = 0,
        Mtu::Value(value) => {
            next.mtu = validate::mtu(value, max)?;
            next.mtu_manual = 1;
        }
    }
    Ok(())
}

fn ipv4_minimal(v4: &Address) -> Value {
    json!({
        "protocol": v4.protocol, "ipaddr": v4.ipaddr, "netmask": v4.netmask,
        "gateway": v4.gateway, "dns": v4.dns, "route": v4.route,
    })
}

fn ipv6_address_text(v6: &Address) -> String {
    if v6.protocol == "static" {
        v6.ip6addr.first().cloned().unwrap_or_else(|| "/".into())
    } else {
        // The web UI builds "address/prefix" from two empty inputs.
        "/".into()
    }
}

fn ipv6_minimal(v6: &Address) -> Value {
    json!({
        "protocol": v6.protocol, "ipaddr": ipv6_address_text(v6),
        "gateway": v6.gateway, "dns": v6.dns, "route": v6.route,
    })
}

/// Spread of the whole object as UGOS sent it, with the overrides on top.
fn spread(address: &Address, overrides: &Value) -> Value {
    let mut base = serde_json::to_value(address).unwrap_or_else(|_| json!({}));
    if let (Some(base), Some(over)) = (base.as_object_mut(), overrides.as_object()) {
        for (key, value) in over {
            let _ = base.insert(key.clone(), value.clone());
        }
    }
    base
}

/// The body of `network/iface/set` for this interface, as the web UI's form
/// for its kind builds it.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] for a kind of interface with no form.
pub fn body(iface: &Iface) -> Result<Value> {
    let name = &iface.interface;
    match iface.kind {
        ETHERNET => Ok(json!({
            "interface": name,
            "mtu": if iface.mtu_manual == 1 { iface.mtu } else { 0 },
            "mtu_manual": iface.mtu_manual,
            "vlan": 0, "status": 0,
            "ipv4": ipv4_minimal(&iface.ipv4), "ipv6": ipv6_minimal(&iface.ipv6),
        })),
        BOND => Ok(json!({
            "interface": name, "bond_mode": iface.bond_mode,
            "ipv4": spread(&iface.ipv4, &ipv4_minimal(&iface.ipv4)),
            "ipv6": ipv6_minimal(&iface.ipv6),
        })),
        WIFI => Ok(json!({
            "interface": name,
            "ipv4": ipv4_minimal(&iface.ipv4), "ipv6": ipv6_minimal(&iface.ipv6),
        })),
        BRIDGE => bridge_body(iface),
        THUNDERBOLT => Ok(json!({
            "interface": name, "status": 0, "mtu_manual": iface.mtu_manual,
            "mtu": iface.mtu,
            "ipv4": spread(&iface.ipv4, &json!({
                "protocol": iface.ipv4.protocol, "ipaddr": iface.ipv4.ipaddr,
                "netmask": iface.ipv4.netmask,
            })),
            "ipv6": { "protocol": iface.ipv6.protocol, "ipaddr": ipv6_address_text(&iface.ipv6) },
        })),
        other => Err(UgosError::Refused(format!(
            "the web UI has no form for an interface of type {other}"
        ))),
    }
}

/// A bridge is sent whole, as the item UGOS listed, with the two address
/// objects replaced.
fn bridge_body(iface: &Iface) -> Result<Value> {
    let mut whole = serde_json::to_value(iface)?;
    let Some(map) = whole.as_object_mut() else {
        return Err(UgosError::Refused("cannot build the bridge body".into()));
    };
    let v6 = &iface.ipv6;
    let first = v6.ip6addr.first().cloned().unwrap_or_default();
    let (address, prefix) = first.split_once('/').unwrap_or(("", ""));
    let _ = map.insert(
        "ipv4".into(),
        spread(
            &iface.ipv4,
            &json!({ "dns": iface.ipv4.dns, "route": iface.ipv4.route }),
        ),
    );
    let _ = map.insert(
        "ipv6".into(),
        spread(
            v6,
            &json!({
                "ip6addr": address, "prefixValue": prefix, "dns": v6.dns,
                "ipaddr": ipv6_address_text(v6), "route": v6.route,
            }),
        ),
    );
    Ok(whole)
}

/// Whether an interface carries an address this session talks to.
async fn carries_session(client: &UgosClient, iface: &Iface) -> bool {
    let session = client.session_addresses().await;
    let own = std::iter::once(iface.ipv4.ipaddr.as_str()).chain(
        iface
            .ipv6
            .ip6addr
            .iter()
            .map(|a| a.split('/').next().unwrap_or_default()),
    );
    own.filter_map(|text| text.parse::<std::net::IpAddr>().ok())
        .any(|ip| session.contains(&ip))
}

/// Change an interface and read it back.
///
/// # Errors
///
/// Returns [`UgosError::NeedsForce`] when the interface is the one this
/// session is connected through and `force` is not set, and
/// [`UgosError::Refused`] for input the web UI would refuse or a change that
/// UGOS did not apply.
pub async fn set_iface(
    client: &UgosClient,
    which: &str,
    change: &IfaceChange,
    force: bool,
) -> Result<IfaceOutcome> {
    if change.is_empty() {
        return Err(UgosError::Refused("nothing to change".into()));
    }
    let before = client.net_iface(which).await?;
    let next = apply(&before, change)?;
    let request = body(&next)?;
    if !force && carries_session(client, &before).await {
        return Err(UgosError::NeedsForce(format!(
            "'{}' is the interface this session talks through; a changed address cuts it off",
            before.interface
        )));
    }
    let new_addresses = client.net_iface_set(&request).await?;
    let iface = read_back(client, &before.interface, &next).await?;
    Ok(IfaceOutcome {
        iface,
        new_addresses,
    })
}

/// Read the interface again until it shows the new setup. `None` when the NAS
/// cannot be reached, which is what a moved address looks like.
async fn read_back(client: &UgosClient, name: &str, wanted: &Iface) -> Result<Option<Iface>> {
    for round in 0..8 {
        tokio::time::sleep(Duration::from_millis(if round == 0 { 1000 } else { 1500 })).await;
        let Ok(now) = client.net_iface(name).await else {
            return Ok(None);
        };
        if matches(&now, wanted) {
            return Ok(Some(now));
        }
    }
    Err(UgosError::Refused(format!(
        "UGOS accepted the request but '{name}' does not show the new setup"
    )))
}

fn matches(now: &Iface, wanted: &Iface) -> bool {
    let same_v4 = now.ipv4.protocol == wanted.ipv4.protocol
        && (wanted.ipv4.protocol != "static" || now.ipv4.ipaddr == wanted.ipv4.ipaddr);
    let v6_ok = now.ipv6.protocol == wanted.ipv6.protocol
        || (wanted.ipv6.protocol == "disable" && now.ipv6.ip6addr.is_empty());
    let mtu_ok = wanted.mtu_manual == 0 || now.mtu == wanted.mtu;
    same_v4 && v6_ok && mtu_ok
}
