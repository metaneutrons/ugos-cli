//! Rows for the network settings of the NAS.

use serde::Serialize;
use tabled::Tabled;
use ugos_client::types::net::{
    FlowRule, FlowService, GatewayEntry, Gateways, Iface, Ipv6Nominee, NetSettings, Slave,
    WifiNetwork,
};

use super::{FieldRow, field, yes_no};

/// What UGOS's type code of an interface means.
fn kind(code: i64) -> String {
    match code {
        0 => "ethernet".into(),
        1 => "bond".into(),
        2 => "wifi".into(),
        3 => "pppoe".into(),
        4 => "vpn".into(),
        5 => "bridge".into(),
        6 => "virtual bridge".into(),
        11 => "thunderbolt".into(),
        other => format!("type {other}"),
    }
}

fn speed(mbit: i64) -> String {
    match mbit {
        0 => "-".into(),
        m if m >= 1000 => format!("{} Gbit/s", m / 1000),
        m => format!("{m} Mbit/s"),
    }
}

fn address(iface: &Iface) -> String {
    if iface.ipv4.ipaddr.is_empty() {
        iface.ipv4.protocol.clone()
    } else {
        format!("{} ({})", iface.ipv4.ipaddr, iface.ipv4.protocol)
    }
}

/// Table row for an interface.
#[derive(Serialize)]
pub struct IfaceRow {
    pub name: String,
    pub label: String,
    pub kind: String,
    pub link: String,
    pub ipv4: String,
    pub speed: String,
}

table_row! {
  IfaceRow {
    name => "Interface",
    label => "Label",
    kind => "Type",
    link => "Link",
    ipv4 => "IPv4",
    speed => "Speed",
  }
}

impl From<&Iface> for IfaceRow {
    fn from(iface: &Iface) -> Self {
        Self {
            name: iface.interface.clone(),
            label: iface.label.clone(),
            kind: kind(iface.kind),
            link: if iface.connection == 1 { "up" } else { "down" }.into(),
            ipv4: address(iface),
            speed: speed(iface.speed),
        }
    }
}

/// The detail view of an interface.
pub fn iface_rows(iface: &Iface) -> Vec<FieldRow> {
    let mut rows = vec![
        field("Interface", iface.interface.clone()),
        field("Label", iface.label.clone()),
        field("Type", kind(iface.kind)),
        field("MAC", iface.mac.clone()),
        field("Link", if iface.connection == 1 { "up" } else { "down" }),
        field("Speed", speed(iface.speed)),
        field(
            "MTU",
            format!(
                "{}{}",
                iface.mtu,
                if iface.mtu_manual == 1 {
                    " (set by hand)"
                } else {
                    ""
                }
            ),
        ),
        field(
            "IPv4",
            format!("{}, {}", iface.ipv4.protocol, iface.ipv4.ipaddr),
        ),
        field("Netmask", iface.ipv4.netmask.clone()),
        field("Gateway", iface.ipv4.gateway.clone()),
        field("DNS", iface.ipv4.dns.join(", ")),
        field("Default route", yes_no(iface.ipv4.route == 1)),
        field("IPv6", iface.ipv6.protocol.clone()),
        field("IPv6 addresses", iface.ipv6.ip6addr.join(", ")),
        field("802.1X", yes_no(iface.ieee8021x.enable)),
    ];
    if iface.kind == 1 {
        rows.push(field("Bond mode", iface.bond_mode.clone()));
    }
    for slave in &iface.slaves {
        rows.push(field(
            &format!("  port {}", slave.interface),
            slave_text(slave),
        ));
    }
    rows
}

fn slave_text(slave: &Slave) -> String {
    format!(
        "{} ({}, {})",
        slave.label,
        if slave.connection == 1 { "up" } else { "down" },
        speed(slave.speed)
    )
}

/// Table row for a port that could join a bond or bridge.
#[derive(Serialize)]
pub struct PortRow {
    pub name: String,
    pub label: String,
    pub link: String,
    pub speed: String,
}

table_row! {
  PortRow {
    name => "Port",
    label => "Label",
    link => "Link",
    speed => "Speed",
  }
}

impl From<&Slave> for PortRow {
    fn from(slave: &Slave) -> Self {
        Self {
            name: slave.interface.clone(),
            label: slave.label.clone(),
            link: if slave.connection == 1 { "up" } else { "down" }.into(),
            speed: speed(slave.speed),
        }
    }
}

/// DNS, gateways, IPv6 and the proxy, without the proxy password.
pub fn general_rows(
    settings: &NetSettings,
    gateways: &Gateways,
    ipv6: &Ipv6Nominee,
) -> Vec<FieldRow> {
    let g = &settings.general;
    let p = &settings.proxy;
    let mut rows = vec![
        field(
            "DNS",
            if g.dns_mode == 0 {
                "automatic (DHCP)"
            } else {
                "manual"
            },
        ),
        field("DNS servers", g.dns.join(", ")),
        field("Several default gateways", yes_no(g.multiple_gateways == 1)),
        field("Prefer IPv4", yes_no(g.prior_ipv4 == 1)),
        field("Address clash check", yes_no(g.clash_check == 1)),
        field("IPv4 gateways", gateway_text(&gateways.ipv4)),
        field("IPv6 gateways", gateway_text(&gateways.ipv6)),
        field(
            "IPv6 interface",
            if ipv6.enabled {
                ipv6.interface.clone()
            } else {
                "none".into()
            },
        ),
        field("Proxy", yes_no(p.enable == 1)),
    ];
    for entry in p.list.iter().filter(|e| !e.host.is_empty()) {
        rows.push(field(
            &format!("  {} proxy", entry.protocol),
            format!("{}:{}", entry.host, entry.port),
        ));
    }
    if p.enable == 1 {
        rows.push(field(
            "Proxy login",
            if p.auth.enable == 1 {
                p.auth.username.clone()
            } else {
                "none".into()
            },
        ));
    }
    rows
}

fn gateway_text(entries: &[GatewayEntry]) -> String {
    entries
        .iter()
        .map(|e| format!("{} via {}", e.gateway, e.label))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Table row for a default gateway.
#[derive(Serialize)]
pub struct GatewayRow {
    pub order: String,
    pub gateway: String,
    pub interface: String,
    pub label: String,
}

table_row! {
  GatewayRow {
    order => "Order",
    gateway => "Gateway",
    interface => "Interface",
    label => "Label",
  }
}

impl From<(usize, &GatewayEntry)> for GatewayRow {
    fn from((index, entry): (usize, &GatewayEntry)) -> Self {
        Self {
            order: (index + 1).to_string(),
            gateway: entry.gateway.clone(),
            interface: entry.interface.clone(),
            label: entry.label.clone(),
        }
    }
}

/// Table row for a traffic control rule.
#[derive(Serialize)]
pub struct FlowRow {
    pub position: String,
    pub interface: String,
    pub on: String,
    pub what: String,
    pub rate: String,
}

table_row! {
  FlowRow {
    position => "#",
    interface => "Interface",
    on => "On",
    what => "Applies to",
    rate => "Rate (KB/s)",
  }
}

impl From<(usize, &FlowRule)> for FlowRow {
    fn from((index, rule): (usize, &FlowRule)) -> Self {
        let what = match rule.kind.as_str() {
            "all" => "all ports".to_owned(),
            "sys" => rule.internal_servers.clone().unwrap_or_default().join(", "),
            _ => format!(
                "{} {} {}",
                rule.protocol.clone().unwrap_or_default(),
                rule.target.clone().unwrap_or_default(),
                rule.ports.clone().unwrap_or_default()
            ),
        };
        Self {
            position: (index + 1).to_string(),
            interface: rule.interface.clone(),
            on: yes_no(rule.enable == 1),
            what,
            rate: format!("{}..{}", rule.min_rate, rule.max_rate),
        }
    }
}

/// Table row for a built-in service of the traffic control.
#[derive(Serialize)]
pub struct ServiceRow {
    pub section: String,
    pub name: String,
    pub ports: String,
    pub protocol: String,
}

table_row! {
  ServiceRow {
    section => "Section",
    name => "Name",
    ports => "Ports",
    protocol => "Protocol",
  }
}

impl From<&FlowService> for ServiceRow {
    fn from(service: &FlowService) -> Self {
        Self {
            section: service.section.clone(),
            name: service.name.clone(),
            ports: service.ports.clone(),
            protocol: service.protocol.clone(),
        }
    }
}

/// Table row for a wireless network.
#[derive(Serialize)]
pub struct WifiRow {
    pub ssid: String,
    pub signal: String,
    pub security: String,
    pub saved: String,
}

table_row! {
  WifiRow {
    ssid => "Network",
    signal => "Signal",
    security => "Security",
    saved => "Saved",
  }
}

impl From<&WifiNetwork> for WifiRow {
    fn from(network: &WifiNetwork) -> Self {
        Self {
            ssid: network.ssid.clone(),
            signal: network.signal.to_string(),
            security: if network.encryption_key == "off" {
                "open".into()
            } else {
                network.authentication.clone()
            },
            saved: yes_no(network.exist == 1 || network.save_net == 1),
        }
    }
}
