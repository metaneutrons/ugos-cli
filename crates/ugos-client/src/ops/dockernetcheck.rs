//! A check for Docker networks whose subnet may shadow other networks.
//!
//! A bridge network gives the NAS a route to its subnet through the bridge.
//! When that range is also in use elsewhere in the network and the NAS
//! reaches it through its gateway, the NAS sends its replies into the bridge
//! and the devices in that range lose it, while the rest works. Docker picks
//! such a range by itself when a compose project declares no network.
//!
//! UGOS does not expose the labels or the IPAM settings of a network, nor the
//! routing table of the NAS, so the check works from what the API does give:
//! the subnets of the Docker networks, and the address, netmask and gateway of
//! each interface. Two rules, strongest first:
//!
//! 1. [`Rule::Overlap`]: the subnet overlaps the subnet of a NAS interface.
//! 2. [`Rule::SameBlock`]: the subnet lies in the same /16 as an address or a
//!    gateway of the NAS. This is a hint only: the NAS reaches other ranges of
//!    that block through its gateway, which no list of interfaces shows.
//!
//! Only `bridge` networks in private ranges count. `host` and `none` have no
//! subnet, and an `ipvlan` or `macvlan` network sits on a card on purpose.

use serde::Serialize;

use crate::api::net::NetApi;
use crate::client::UgosClient;
use crate::error::Result;
use crate::ops::dockernet;
use crate::types::docker::Network;
use crate::types::net::Iface;

/// Which rule a network broke.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Rule {
    /// Overlaps the subnet of a NAS interface.
    Overlap,
    /// In the same /16 as an address or gateway of the NAS.
    SameBlock,
}

impl Rule {
    /// A few words for a table.
    #[must_use]
    pub const fn summary(self) -> &'static str {
        match self {
            Self::Overlap => "overlaps a subnet of the NAS",
            Self::SameBlock => "same /16 as an address or gateway of the NAS (possible)",
        }
    }
}

/// A network that can shadow others.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    /// Network name.
    pub network: String,
    /// Its IPv4 subnet.
    pub subnet: String,
    /// The rule it broke.
    pub rule: Rule,
    /// Why this is risky.
    pub reason: String,
}

impl Finding {
    /// One line for a warning.
    #[must_use]
    pub fn line(&self) -> String {
        format!(
            "warning: Docker network '{}' ({}) {}. That range may be in use elsewhere in \
             your network. An explicit subnet in the compose file avoids it (networks, \
             ipam, config, subnet).",
            self.network, self.subnet, self.reason
        )
    }
}

/// An IPv4 range, address and prefix length.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Cidr {
    addr: u32,
    prefix: u8,
}

impl Cidr {
    fn parse(text: &str) -> Option<Self> {
        let (addr, prefix) = text.trim().split_once('/')?;
        let prefix: u8 = prefix.parse().ok().filter(|p| *p <= 32)?;
        Some(Self::new(addr.parse().ok()?, prefix))
    }

    fn new(addr: std::net::Ipv4Addr, prefix: u8) -> Self {
        let mut cidr = Self {
            addr: u32::from(addr),
            prefix,
        };
        cidr.addr &= cidr.mask();
        cidr
    }

    fn mask(self) -> u32 {
        u32::MAX
            .checked_shl(32 - u32::from(self.prefix))
            .unwrap_or(0)
    }

    fn overlaps(self, other: Self) -> bool {
        let mask = self.mask().min(other.mask());
        (self.addr & mask) == (other.addr & mask)
    }

    fn contains(self, addr: u32) -> bool {
        addr & self.mask() == self.addr
    }

    /// The /16 around an address.
    fn block16(addr: std::net::Ipv4Addr) -> Self {
        Self::new(addr, 16)
    }
}

impl std::fmt::Display for Cidr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", std::net::Ipv4Addr::from(self.addr), self.prefix)
    }
}

fn private_ranges() -> [Cidr; 3] {
    [
        Cidr::new(std::net::Ipv4Addr::new(10, 0, 0, 0), 8),
        Cidr::new(std::net::Ipv4Addr::new(172, 16, 0, 0), 12),
        Cidr::new(std::net::Ipv4Addr::new(192, 168, 0, 0), 16),
    ]
}

/// What the NAS has on an interface.
struct Own {
    label: String,
    subnet: Option<Cidr>,
    address: Option<std::net::Ipv4Addr>,
    gateway: Option<std::net::Ipv4Addr>,
}

fn prefix_of(netmask: &str) -> Option<u8> {
    let mask = u32::from(netmask.trim().parse::<std::net::Ipv4Addr>().ok()?);
    // A netmask is ones then zeros.
    (mask.leading_ones() + mask.trailing_zeros() == 32)
        .then(|| u8::try_from(mask.leading_ones()).ok())?
}

fn own_of(iface: &Iface) -> Own {
    let v4 = &iface.ipv4;
    let address = v4.ipaddr.trim().parse::<std::net::Ipv4Addr>().ok();
    let subnet = address
        .zip(prefix_of(&v4.netmask))
        .map(|(a, p)| Cidr::new(a, p));
    let label = if iface.label.is_empty() {
        iface.interface.clone()
    } else {
        format!("{} ({})", iface.interface, iface.label)
    };
    Own {
        label,
        subnet,
        address,
        gateway: v4.gateway.trim().parse().ok(),
    }
}

/// The first rule the subnet breaks, with the reason.
type Verdict = (Rule, String);

fn overlap(subnet: Cidr, own: &[Own]) -> Option<Verdict> {
    let (o, s) = own
        .iter()
        .find_map(|o| o.subnet.filter(|s| s.overlaps(subnet)).map(|s| (o, s)))?;
    Some((
        Rule::Overlap,
        format!(
            "overlaps {} of the NAS ({s}). The NAS then sends traffic for that range into the \
             Docker bridge instead of the network",
            o.label
        ),
    ))
}

fn same_block(subnet: Cidr, own: &[Own]) -> Option<Verdict> {
    let near = own.iter().find_map(|o| {
        [(o.address, "the address"), (o.gateway, "the gateway")]
            .into_iter()
            .find_map(|(a, what)| {
                let a = a?;
                Cidr::block16(a)
                    .overlaps(subnet)
                    .then(|| format!("{what} {a} of {}", o.label))
            })
    })?;
    Some((
        Rule::SameBlock,
        format!(
            "lies in the same /16 as {near}. It may shadow a range in that block that the NAS \
             reaches through its gateway: the NAS would then answer into the Docker bridge \
             (a possibility, not a certainty)"
        ),
    ))
}

fn judge(net: &Network, own: &[Own]) -> Option<Finding> {
    if net.driver != "bridge" {
        return None;
    }
    let subnet = Cidr::parse(&net.ipv4_subnet)?;
    if !private_ranges().iter().any(|r| r.contains(subnet.addr)) {
        return None;
    }
    let (rule, reason) = overlap(subnet, own).or_else(|| same_block(subnet, own))?;
    Some(Finding {
        network: net.network_name.clone(),
        subnet: net.ipv4_subnet.clone(),
        rule,
        reason,
    })
}

/// The networks that can shadow others, given the NAS's own interfaces.
#[must_use]
pub fn check(networks: &[Network], ifaces: &[Iface]) -> Vec<Finding> {
    let own: Vec<Own> = ifaces.iter().map(own_of).collect();
    networks.iter().filter_map(|n| judge(n, &own)).collect()
}

/// Narrow findings to the networks a compose project created. Compose names
/// them `<project>_<network>`.
#[must_use]
pub fn of_project(findings: Vec<Finding>, project: &str) -> Vec<Finding> {
    let prefix = format!("{}_", project.to_lowercase());
    findings
        .into_iter()
        .filter(|f| f.network.to_lowercase().starts_with(&prefix))
        .collect()
}

/// Read the networks and interfaces and check them.
///
/// # Errors
///
/// Returns the appropriate [`crate::error::UgosError`] when either read fails.
pub async fn run(client: &UgosClient) -> Result<Vec<Finding>> {
    let networks = dockernet::list(client).await?;
    let ifaces = client.net_ifaces().await?;
    Ok(check(&networks, &ifaces))
}

/// The warnings for the networks of a project, or none when the check cannot
/// run. A failed check must not fail the command it follows.
pub async fn warnings_for_project(client: &UgosClient, project: &str) -> Vec<String> {
    run(client).await.map_or_else(
        |_| Vec::new(),
        |found| {
            of_project(found, project)
                .iter()
                .map(Finding::line)
                .collect()
        },
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn iface(addr: &str, mask: &str, gw: &str) -> Iface {
        serde_json::from_value(serde_json::json!({
            "interface": "bridge0", "label": "LAN1",
            "ipv4": {"ipaddr": addr, "netmask": mask, "gateway": gw}
        }))
        .unwrap()
    }

    fn net(name: &str, driver: &str, subnet: &str) -> Network {
        serde_json::from_value(serde_json::json!({
            "networkId": name, "networkName": name, "driver": driver, "ipv4Subnet": subnet
        }))
        .unwrap()
    }

    fn rule_for(nas: &Iface, n: &Network) -> Option<Rule> {
        check(std::slice::from_ref(n), std::slice::from_ref(nas))
            .first()
            .map(|f| f.rule)
    }

    #[test]
    fn rules_by_case() {
        let lan = iface("192.168.2.5", "255.255.240.0", "192.168.1.1");
        let ten = iface("10.1.2.5", "255.255.255.0", "10.1.2.1");
        let cases = [
            // The incident: a range behind the gateway, not attached to the NAS.
            (&lan, "bridge", "192.168.16.0/20", Some(Rule::SameBlock)),
            (&lan, "bridge", "10.77.5.0/24", None),
            (&lan, "bridge", "172.17.0.0/16", None),
            (&lan, "bridge", "192.168.0.0/24", Some(Rule::Overlap)),
            (&lan, "bridge", "192.168.0.0/16", Some(Rule::Overlap)),
            (&lan, "bridge", "10.0.0.0/8", None),
            // Same /16 as the gateway, not as the address.
            (
                &iface("10.9.9.5", "255.255.255.0", "192.168.1.1"),
                "bridge",
                "192.168.200.0/24",
                Some(Rule::SameBlock),
            ),
            (&ten, "bridge", "10.1.200.0/24", Some(Rule::SameBlock)),
            (&ten, "bridge", "10.2.0.0/24", None),
            // Elsewhere in the same private range, but not in the /16.
            (&ten, "bridge", "192.168.48.0/20", None),
            (&lan, "ipvlan", "192.168.0.0/20", None),
            (&lan, "host", "", None),
            (&lan, "bridge", "--", None),
            (&lan, "bridge", "198.51.100.0/24", None),
        ];
        for (nas, driver, subnet, want) in cases {
            let n = net("n", driver, subnet);
            assert_eq!(rule_for(nas, &n), want, "{driver} {subnet}");
        }
    }

    #[test]
    fn the_incident_names_network_subnet_and_reason() {
        let lan = iface("192.168.2.5", "255.255.240.0", "192.168.1.1");
        let n = net("proj_default", "bridge", "192.168.16.0/20");
        let found = check(&[n], &[lan]);
        let line = found[0].line();
        assert!(line.contains("proj_default") && line.contains("192.168.16.0/20"));
        assert!(line.contains("may shadow") && line.contains("192.168.2.5"));
    }

    #[test]
    fn an_interface_without_ipv4_is_ignored() {
        let down = iface("", "", "");
        let n = net("n", "bridge", "10.77.5.0/24");
        assert_eq!(check(&[n], &[down]).len(), 0);
    }

    #[test]
    fn a_netmask_must_be_contiguous() {
        assert_eq!(prefix_of("255.255.240.0"), Some(20));
        assert_eq!(prefix_of("255.0.255.0"), None);
        assert_eq!(prefix_of(""), None);
    }

    #[test]
    fn a_project_keeps_only_its_own_networks() {
        let lan = iface("192.168.2.5", "255.255.240.0", "192.168.1.1");
        let all = check(
            &[
                net("Shop_default", "bridge", "192.168.16.0/20"),
                net("other_default", "bridge", "192.168.32.0/20"),
            ],
            &[lan],
        );
        assert_eq!(all.len(), 2);
        let mine = of_project(all, "shop");
        assert_eq!(mine.len(), 1);
        assert_eq!(mine[0].network, "Shop_default");
    }
}
