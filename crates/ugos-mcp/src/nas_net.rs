//! Network settings of the NAS: the logic behind the `ugos_net_*` tools.
//!
//! The same operations as `ugos-cli net`, from `ugos_client::ops::net`. A
//! change that would cut this session off, or take rules with it, is refused
//! unless `force` is given, as on the command line. The client of the tool
//! decides whether a model may call these at all.

use rmcp::schemars;
use serde::Serialize;
use ugos_client::UgosClient;
use ugos_client::api::net::{NetApi, NetFlowApi, NetWifiApi};
use ugos_client::error::UgosError;
use ugos_client::ops::net::flow::{self, NewRule};
use ugos_client::ops::net::general::{
    DnsChange, GeneralChange, ProxyChange, set_gateway_order, set_general, set_proxy,
};
use ugos_client::ops::net::iface::{IfaceChange, Ipv6Mode, Mtu, set_iface};
use ugos_client::ops::net::links::{self, BondIpv4};
use ugos_client::ops::net::wifi;

use crate::manage::failure;

fn json<T: Serialize>(result: Result<T, UgosError>) -> String {
    match result {
        Ok(value) => serde_json::to_string_pretty(&value).unwrap_or_default(),
        Err(e) => failure(&e),
    }
}

fn done(result: Result<String, UgosError>) -> String {
    result.unwrap_or_else(|e| failure(&e))
}

// ── Parameters ──────────────────────────────────────────────────────

/// Target only, plus a name.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct IfaceForceParam {
    /// Interface name or label, e.g. `bond0`.
    pub name: String,
    /// Go on although rules name it or it carries this session.
    #[serde(default)]
    pub force: bool,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// Change the setup of an interface.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct NetSetParam {
    /// Interface name or label, e.g. `eth0`, `bridge0`, `LAN1`.
    pub name: String,
    /// IPv4 protocol: `dhcp`, `static` or `ipv4ll` (Thunderbolt only).
    #[serde(default)]
    pub ipv4_protocol: Option<String>,
    /// Static IPv4 address.
    #[serde(default)]
    pub ip: Option<String>,
    /// Netmask of the static address, e.g. `255.255.255.0`.
    #[serde(default)]
    pub netmask: Option<String>,
    /// Gateway of the static address.
    #[serde(default)]
    pub gateway: Option<String>,
    /// DNS servers of the interface; an empty list clears them.
    #[serde(default)]
    pub dns: Option<Vec<String>>,
    /// Make the interface the default route.
    #[serde(default)]
    pub default_route: Option<bool>,
    /// IPv6: `auto`, `off` or `static`.
    #[serde(default)]
    pub ipv6: Option<String>,
    /// Static IPv6 address, without the prefix.
    #[serde(default)]
    pub ipv6_address: Option<String>,
    /// Static IPv6 prefix length.
    #[serde(default)]
    pub ipv6_prefix: Option<i64>,
    /// IPv6 gateway.
    #[serde(default)]
    pub ipv6_gateway: Option<String>,
    /// IPv6 DNS servers.
    #[serde(default)]
    pub ipv6_dns: Option<Vec<String>>,
    /// MTU in bytes, or `auto`.
    #[serde(default)]
    pub mtu: Option<String>,
    /// Bond mode, for a bond.
    #[serde(default)]
    pub bond_mode: Option<String>,
    /// Go on although this is the interface this session talks through; a
    /// changed address cuts it off.
    #[serde(default)]
    pub force: bool,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// DNS and the general switches.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct NetDnsParam {
    /// Take the DNS servers from DHCP.
    #[serde(default)]
    pub auto: bool,
    /// One or two DNS servers to use instead.
    #[serde(default)]
    pub servers: Vec<String>,
    /// Allow several default gateways.
    #[serde(default)]
    pub multiple_gateways: Option<bool>,
    /// Prefer IPv4 over IPv6.
    #[serde(default)]
    pub prefer_ipv4: Option<bool>,
    /// Check for an address clash when an address changes.
    #[serde(default)]
    pub clash_check: Option<bool>,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// The order of the default gateways.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GatewayOrderParam {
    /// Interface names or labels, most preferred first.
    pub ifaces: Vec<String>,
    /// Order the IPv6 gateways instead of the IPv4 ones.
    #[serde(default)]
    pub ipv6: bool,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// The proxy of the NAS.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct NetProxyParam {
    /// One proxy for every protocol, `IP:PORT`. Leave out with `off`.
    #[serde(default)]
    pub all: Option<String>,
    /// HTTP proxy, `IP:PORT`.
    #[serde(default)]
    pub http: Option<String>,
    /// HTTPS proxy, `IP:PORT`.
    #[serde(default)]
    pub https: Option<String>,
    /// Use the proxy for local addresses too.
    #[serde(default)]
    pub local: bool,
    /// User name of the proxy.
    #[serde(default)]
    pub user: Option<String>,
    /// Password of the proxy. It passes through the model; prefer `ugos-cli`.
    #[serde(default)]
    pub password: Option<String>,
    /// Switch the proxy off.
    #[serde(default)]
    pub off: bool,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// Create a bond.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct BondCreateParam {
    /// Two or more ports, e.g. `["eth0", "eth1"]`.
    pub ports: Vec<String>,
    /// Mode: `balance-alb` (default), `active-backup`, `balance-rr`,
    /// `balance-xor` or `802.3ad`.
    #[serde(default)]
    pub mode: Option<String>,
    /// Static IPv4 address; DHCP when left out.
    #[serde(default)]
    pub ip: Option<String>,
    /// Netmask of the static address.
    #[serde(default)]
    pub netmask: Option<String>,
    /// Gateway of the static address.
    #[serde(default)]
    pub gateway: Option<String>,
    /// DNS servers, up to two.
    #[serde(default)]
    pub dns: Vec<String>,
    /// Make the bond the default route.
    #[serde(default)]
    pub default_route: bool,
    /// Go on although a port carries this session's address or the ports are
    /// in different networks.
    #[serde(default)]
    pub force: bool,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// Ports for a bridge.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PortsParam {
    /// The ports. Wi-Fi cannot be bridged.
    pub ports: Vec<String>,
    /// Go on although a port carries this session's address.
    #[serde(default)]
    pub force: bool,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// A Wi-Fi interface.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct WifiParam {
    /// The Wi-Fi interface, e.g. `wlan0`.
    pub iface: String,
    /// The network name, where the tool needs one.
    #[serde(default)]
    pub ssid: Option<String>,
    /// Password of the network. It passes through the model; prefer `ugos-cli`.
    #[serde(default)]
    pub password: Option<String>,
    /// Authentication of a hidden network: `wpa`, `wpa2`, `wpa-wpa2`, `wpa3`,
    /// `wpa2-wpa3` or `none`.
    #[serde(default)]
    pub auth: Option<String>,
    /// Switch the radio on (`true`) or off (`false`), for the radio tool.
    #[serde(default)]
    pub on: Option<bool>,
    /// Scan again instead of using the last scan, for the list tool.
    #[serde(default)]
    pub rescan: bool,
    /// Join the network by itself, for the edit tool.
    #[serde(default)]
    pub auto_join: Option<bool>,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// A traffic control rule to add.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct FlowAddParam {
    /// Interface name, or `all`.
    pub interface: String,
    /// A rule for every port.
    #[serde(default)]
    pub all_ports: bool,
    /// Ports: a comma list of up to 15, or a range `a-b`.
    #[serde(default)]
    pub ports: Option<String>,
    /// Built-in services by section id or name, see the services tool.
    #[serde(default)]
    pub services: Vec<String>,
    /// `src` or `dest` port of a custom rule (default `dest`).
    #[serde(default)]
    pub target_port: Option<String>,
    /// `tcp`, `udp` or `all` (default `all`).
    #[serde(default)]
    pub protocol: Option<String>,
    /// Guaranteed rate in KB/s (default 0).
    #[serde(default)]
    pub min_rate: i64,
    /// Limit in KB/s.
    pub max_rate: i64,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// A rule by its position.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct FlowPositionParam {
    /// Position in the list, counting from 1.
    pub position: usize,
    /// Switch the rule on (`true`) or off (`false`), for the enable tool.
    #[serde(default)]
    pub on: Option<bool>,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

// ── Reading ─────────────────────────────────────────────────────────

pub async fn list(client: &UgosClient) -> String {
    json(client.net_ifaces().await)
}

pub async fn show(client: &UgosClient, name: &str) -> String {
    json(client.net_iface(name).await)
}

pub async fn general(client: &UgosClient) -> String {
    let both = async {
        let settings = client.net_settings().await?;
        Ok(serde_json::json!({
            "dns": settings.general.dns, "dns_mode": settings.general.dns_mode,
            "multiple_gateways": settings.general.multiple_gateways,
            "prior_ipv4": settings.general.prior_ipv4,
            "gateways": client.net_gateways().await?,
            "ipv6": client.net_ipv6().await?,
            "proxy": { "enable": settings.proxy.enable, "list": settings.proxy.list },
        }))
    };
    json(both.await)
}

pub async fn candidates(client: &UgosClient, bond: bool) -> String {
    json(if bond {
        client.net_bond_candidates().await
    } else {
        client.net_bridge_candidates().await
    })
}

// ── Interfaces ──────────────────────────────────────────────────────

fn ipv6_mode(text: &str) -> Result<Ipv6Mode, UgosError> {
    match text {
        "auto" => Ok(Ipv6Mode::Auto),
        "off" => Ok(Ipv6Mode::Off),
        "static" => Ok(Ipv6Mode::Static),
        other => Err(UgosError::Refused(format!(
            "ipv6 is auto, off or static, not '{other}'"
        ))),
    }
}

fn mtu_of(text: &str) -> Result<Mtu, UgosError> {
    if text.eq_ignore_ascii_case("auto") {
        return Ok(Mtu::Auto);
    }
    text.parse()
        .map(Mtu::Value)
        .map_err(|_| UgosError::Refused(format!("the MTU '{text}' is a number or 'auto'")))
}

fn iface_change(p: &NetSetParam) -> Result<IfaceChange, UgosError> {
    let protocol = p.ipv4_protocol.clone().or_else(|| {
        (p.ip.is_some() || p.netmask.is_some() || p.gateway.is_some()).then(|| "static".to_owned())
    });
    Ok(IfaceChange {
        ipv4_protocol: protocol,
        ipv4_address: p.ip.clone(),
        ipv4_netmask: p.netmask.clone(),
        ipv4_gateway: p.gateway.clone(),
        ipv4_dns: p.dns.clone(),
        ipv4_route: p.default_route,
        ipv6_mode: p.ipv6.as_deref().map(ipv6_mode).transpose()?,
        ipv6_address: p.ipv6_address.clone(),
        ipv6_prefix: p.ipv6_prefix,
        ipv6_gateway: p.ipv6_gateway.clone(),
        ipv6_dns: p.ipv6_dns.clone(),
        ipv6_route: None,
        mtu: p.mtu.as_deref().map(mtu_of).transpose()?,
        bond_mode: p.bond_mode.clone(),
    })
}

pub async fn set(client: &UgosClient, p: &NetSetParam) -> String {
    let run = async {
        let change = iface_change(p)?;
        let outcome = set_iface(client, &p.name, &change, p.force).await?;
        Ok(serde_json::json!({ "iface": outcome.iface, "new_addresses": outcome.new_addresses }))
    };
    json(run.await)
}

pub async fn dns(client: &UgosClient, p: &NetDnsParam) -> String {
    let dns = if p.auto {
        Some(DnsChange::Auto)
    } else if p.servers.is_empty() {
        None
    } else {
        Some(DnsChange::Servers(p.servers.clone()))
    };
    let change = GeneralChange {
        dns,
        multiple_gateways: p.multiple_gateways,
        prior_ipv4: p.prefer_ipv4,
        clash_check: p.clash_check,
    };
    json(
        set_general(client, &change)
            .await
            .map(|(now, reboot)| serde_json::json!({ "reboot": reboot, "dns": now.general.dns, "dns_mode": now.general.dns_mode })),
    )
}

pub async fn gateway_order(client: &UgosClient, p: &GatewayOrderParam) -> String {
    json(set_gateway_order(client, p.ipv6, &p.ifaces).await)
}

fn host_port(text: &str) -> Result<(String, i64), UgosError> {
    let (host, port) = text
        .rsplit_once(':')
        .ok_or_else(|| UgosError::Refused(format!("'{text}' is not IP:PORT")))?;
    let port = port
        .parse()
        .map_err(|_| UgosError::Refused(format!("'{port}' is not a port")))?;
    Ok((host.to_owned(), port))
}

fn proxy_change(p: &NetProxyParam) -> Result<ProxyChange, UgosError> {
    if p.off {
        return Ok(ProxyChange::Off);
    }
    let auth = p.user.clone().zip(p.password.clone());
    match (&p.all, &p.http, &p.https) {
        (Some(all), None, None) => {
            let (host, port) = host_port(all)?;
            Ok(ProxyChange::All {
                host,
                port,
                local: p.local,
                auth,
            })
        }
        (None, http, https) if http.is_some() || https.is_some() => Ok(ProxyChange::Split {
            http: http.as_deref().map(host_port).transpose()?,
            https: https.as_deref().map(host_port).transpose()?,
            local: p.local,
            auth,
        }),
        _ => Err(UgosError::Refused(
            "give all, or http and/or https, or off".into(),
        )),
    }
}

pub async fn proxy(client: &UgosClient, p: &NetProxyParam) -> String {
    let run = async {
        let (now, reboot) = set_proxy(client, &proxy_change(p)?).await?;
        Ok(
            serde_json::json!({ "reboot": reboot, "enable": now.proxy.enable, "list": now.proxy.list }),
        )
    };
    json(run.await)
}

// ── Bonds and bridges ───────────────────────────────────────────────

pub async fn bond_create(client: &UgosClient, p: &BondCreateParam) -> String {
    let mut servers = p.dns.iter().cloned();
    let ipv4 = BondIpv4 {
        protocol: if p.ip.is_some() { "static" } else { "dhcp" }.into(),
        address: p.ip.clone().unwrap_or_default(),
        netmask: p.netmask.clone().unwrap_or_default(),
        gateway: p.gateway.clone().unwrap_or_default(),
        dns: [
            servers.next().unwrap_or_default(),
            servers.next().unwrap_or_default(),
        ],
        route: p.default_route,
    };
    let mode = p.mode.as_deref().unwrap_or("balance-alb");
    json(links::create_bond(client, &p.ports, mode, &ipv4, p.force).await)
}

pub async fn bond_delete(client: &UgosClient, p: &IfaceForceParam) -> String {
    json(links::delete_bond(client, &p.name, p.force).await)
}

pub async fn bridge_create(client: &UgosClient, p: &PortsParam) -> String {
    json(links::create_bridge(client, &p.ports, p.force).await)
}

pub async fn bridge_create_virtual(client: &UgosClient, p: &PortsParam) -> String {
    json(links::create_virtual_bridge(client, &p.ports).await)
}

pub async fn bridge_delete(client: &UgosClient, p: &IfaceForceParam) -> String {
    json(links::delete_bridge(client, &p.name, p.force).await)
}

// ── Wi-Fi ───────────────────────────────────────────────────────────

fn ssid_of(p: &WifiParam) -> Result<&str, UgosError> {
    p.ssid
        .as_deref()
        .ok_or_else(|| UgosError::Refused("give the network name (ssid)".into()))
}

pub async fn wifi_status(client: &UgosClient, p: &WifiParam) -> String {
    let both = async {
        Ok(serde_json::json!({
            "radio": client.net_wifi_radio(&p.iface).await?,
            "connection": client.net_wifi_status(&p.iface).await?,
        }))
    };
    json(both.await)
}

pub async fn wifi_radio(client: &UgosClient, p: &WifiParam) -> String {
    let Some(on) = p.on else {
        return "error: give on as true or false".into();
    };
    done(
        client
            .net_wifi_radio_set(&p.iface, on)
            .await
            .map(|()| format!("Wi-Fi on {} is {}", p.iface, if on { "on" } else { "off" })),
    )
}

pub async fn wifi_list(client: &UgosClient, p: &WifiParam) -> String {
    json(client.net_wifi_list(&p.iface, p.rescan).await)
}

pub async fn wifi_connect(client: &UgosClient, p: &WifiParam) -> String {
    let run = async {
        let ssid = ssid_of(p)?;
        wifi::connect(
            client,
            &p.iface,
            ssid,
            p.password.as_deref(),
            p.auth.as_deref(),
        )
        .await
    };
    json(run.await)
}

pub async fn wifi_forget(client: &UgosClient, p: &WifiParam) -> String {
    let run = async {
        let ssid = ssid_of(p)?;
        wifi::forget(client, &p.iface, ssid)
            .await
            .map(|()| format!("Forgot {ssid}"))
    };
    done(run.await)
}

pub async fn wifi_edit(client: &UgosClient, p: &WifiParam) -> String {
    let run = async {
        let ssid = ssid_of(p)?;
        wifi::edit(client, &p.iface, ssid, p.auto_join, None)
            .await
            .map(|()| format!("Updated {ssid}"))
    };
    done(run.await)
}

// ── Traffic control ─────────────────────────────────────────────────

pub async fn flow_list(client: &UgosClient) -> String {
    json(client.net_flow().await)
}

pub async fn flow_services(client: &UgosClient) -> String {
    json(client.net_flow_services(&[]).await)
}

pub async fn flow_add(client: &UgosClient, p: &FlowAddParam) -> String {
    let kind = if p.all_ports {
        "all"
    } else if !p.services.is_empty() {
        "sys"
    } else if p.ports.is_some() {
        "custom"
    } else {
        return "error: give all_ports, ports or services".into();
    };
    let rule = NewRule {
        interface: p.interface.clone(),
        kind: kind.into(),
        ports: p.ports.clone().unwrap_or_default(),
        target: p.target_port.clone().unwrap_or_else(|| "dest".into()),
        protocol: p.protocol.clone().unwrap_or_else(|| "all".into()),
        min_rate: p.min_rate,
        max_rate: p.max_rate,
        sections: p.services.clone(),
    };
    json(flow::add_rule(client, &rule).await)
}

pub async fn flow_remove(client: &UgosClient, p: &FlowPositionParam) -> String {
    json(flow::remove_rule(client, p.position).await)
}

pub async fn flow_enable(client: &UgosClient, p: &FlowPositionParam) -> String {
    let Some(on) = p.on else {
        return "error: give on as true or false".into();
    };
    json(flow::enable_rule(client, p.position, on).await)
}
