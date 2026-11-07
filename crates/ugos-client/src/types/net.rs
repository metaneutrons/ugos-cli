//! Types for the network settings of the NAS itself.
//!
//! Not to be confused with `types::kvm::NetworkDetail`, which describes a
//! network a virtual machine attaches to.

use serde::{Deserialize, Serialize};

use super::common::null_vec_as_empty;

/// An IPv4 or IPv6 setup of an interface.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Address {
    /// `static` or `dhcp`.
    #[serde(default)]
    pub protocol: String,
    /// The address.
    #[serde(default)]
    pub ipaddr: String,
    /// Netmask in dotted form (IPv4).
    #[serde(default)]
    pub netmask: String,
    /// Prefix length as UGOS sends it, when it does.
    #[serde(default)]
    pub mask: String,
    /// Gateway.
    #[serde(default)]
    pub gateway: String,
    /// Default route.
    #[serde(default)]
    pub default_route: String,
    /// DNS servers of this interface.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub dns: Vec<String>,
    /// IPv6 addresses with prefix.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub ip6addr: Vec<String>,
    /// Status code, `0` for a configured address.
    #[serde(default)]
    pub status: i64,
    /// `1` when this interface is the default route.
    #[serde(default)]
    pub route: i64,
    /// IPv6 prefix length, when UGOS sends it.
    #[serde(default)]
    pub prefix: i64,
    /// Whatever else UGOS sent. A change writes it back as it was.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// A port that belongs to a bridge or a bond.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Slave {
    /// Interface name, e.g. `eth0`.
    #[serde(default)]
    pub interface: String,
    /// Label the web UI shows, e.g. `LAN1`.
    #[serde(default)]
    pub label: String,
    /// `1` when a cable is connected.
    #[serde(default)]
    pub connection: i64,
    /// Speed in Mbit/s.
    #[serde(default)]
    pub speed: i64,
    /// `full`, `half` or `unknown`.
    #[serde(default)]
    pub duplex: String,
}

/// A network interface.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Iface {
    /// Name, e.g. `bridge0`, `eth0`, `wlan0`.
    #[serde(default)]
    pub interface: String,
    /// Label the web UI shows, e.g. `LAN1` or `VBR-LAN1`.
    #[serde(default)]
    pub label: String,
    /// MAC address.
    #[serde(default)]
    pub mac: String,
    /// MTU.
    #[serde(default)]
    pub mtu: i64,
    /// Whether the MTU was set by hand.
    #[serde(default)]
    pub mtu_manual: i64,
    /// Speed in Mbit/s, `0` when down.
    #[serde(default)]
    pub speed: i64,
    /// `full`, `half` or `unknown`.
    #[serde(default)]
    pub duplex: String,
    /// `1` when a cable is connected.
    #[serde(default)]
    pub connection: i64,
    /// Type code; the web UI names it, the API does not (`2` was a Wi-Fi
    /// adapter and `5` a bridge).
    #[serde(default, rename = "type")]
    pub kind: i64,
    /// Bond mode, `unknown` for none.
    #[serde(default)]
    pub bond_mode: String,
    /// IPv4 setup.
    #[serde(default)]
    pub ipv4: Address,
    /// IPv6 setup.
    #[serde(default)]
    pub ipv6: Address,
    /// Whether 802.1X authentication is on.
    #[serde(default)]
    pub ieee8021x: Ieee8021x,
    /// The ports of a bridge or bond.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub slaves: Vec<Slave>,
    /// Whatever else UGOS sent. A change to a bridge writes the whole item
    /// back, as the web UI does.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// The 802.1X switch of an interface.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ieee8021x {
    /// Whether it is on.
    #[serde(default)]
    pub enable: bool,
}

/// `network/iface/list`.
#[derive(Debug, Deserialize)]
pub(crate) struct IfaceList {
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub ifaces: Vec<Iface>,
}

/// A default gateway.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Gateway {
    /// Address of the gateway.
    #[serde(default)]
    pub ipaddr: String,
    /// Interface it is reached through.
    #[serde(default)]
    pub interface: String,
    /// Label of that interface.
    #[serde(default)]
    pub label: String,
}

/// Default gateways, as `network/general/gateway_get` lists them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Gateways {
    /// IPv4 gateways.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub ipv4: Vec<GatewayEntry>,
    /// IPv6 gateways.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub ipv6: Vec<GatewayEntry>,
}

/// One entry of the gateway list.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GatewayEntry {
    /// Address of the gateway.
    #[serde(default)]
    pub gateway: String,
    /// Interface it is reached through.
    #[serde(default)]
    pub interface: String,
    /// Label of that interface.
    #[serde(default)]
    pub label: String,
    /// Order of preference, `0` first.
    #[serde(default)]
    pub priority: i64,
    /// Whatever else UGOS sent. A reorder writes it back.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// General settings: DNS and gateways.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneralNet {
    /// DNS servers.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub dns: Vec<String>,
    /// `0` for the servers above, other codes for DHCP-provided ones.
    #[serde(default)]
    pub dns_mode: i64,
    /// The IPv4 default gateway.
    #[serde(default)]
    pub ipv4_gateway: Gateway,
    /// The IPv6 default gateway.
    #[serde(default)]
    pub ipv6_gateway: Gateway,
    /// Whether several default gateways are allowed.
    #[serde(default)]
    pub multiple_gateways: i64,
    /// Whether IPv4 is preferred.
    #[serde(default)]
    pub prior_ipv4: i64,
    /// Whether the address is checked for a clash on change.
    #[serde(default)]
    pub clash_check: i64,
    /// Whatever else UGOS sent; written back as it was.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// One proxy of the list.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProxyEntry {
    /// Host.
    #[serde(default)]
    pub host: String,
    /// Port.
    #[serde(default)]
    pub port: i64,
    /// `http`, `https`, `ftp`, or `all`.
    #[serde(default)]
    pub protocol: String,
}

/// Proxy credentials. The password travels as UGOS holds it.
#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProxyAuth {
    /// Whether the proxy wants a login.
    #[serde(default)]
    pub enable: i64,
    /// User name.
    #[serde(default)]
    pub username: String,
    /// Password, as read. Never printed by `Debug`.
    #[serde(default)]
    pub password: String,
}

impl std::fmt::Debug for ProxyAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProxyAuth")
            .field("enable", &self.enable)
            .field("username", &self.username)
            .field("password", &"<hidden>")
            .finish()
    }
}

/// The HTTP proxy of the NAS.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Proxy {
    /// Whether it is on.
    #[serde(default)]
    pub enable: i64,
    /// Whether one proxy serves every protocol.
    #[serde(default)]
    pub httpx_proxy: i64,
    /// Whether local addresses use it.
    #[serde(default)]
    pub local_proxy: i64,
    /// Login.
    #[serde(default)]
    pub auth: ProxyAuth,
    /// The proxies.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub list: Vec<ProxyEntry>,
    /// Whatever else UGOS sent; written back as it was.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// `network/general/get`: DNS, gateways and the proxy.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetSettings {
    /// DNS and gateways.
    #[serde(default)]
    pub general: GeneralNet,
    /// The proxy.
    #[serde(default)]
    pub proxy: Proxy,
    /// Whatever else UGOS sent; written back as it was.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// The interface that carries IPv6.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ipv6Nominee {
    /// Whether one is chosen.
    #[serde(default)]
    pub enabled: bool,
    /// Its name.
    #[serde(default)]
    pub interface: String,
}

/// Ports that can still join a bond or a bridge.
#[derive(Debug, Deserialize)]
pub(crate) struct Selectable {
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub ifaces: Vec<Slave>,
}

// ── Traffic control ─────────────────────────────────────────────────

/// A rule of the traffic control.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlowRule {
    /// `all` (every port), `sys` (built-in services) or `custom` (ports).
    #[serde(default, rename = "type")]
    pub kind: String,
    /// `1` when the rule applies.
    #[serde(default)]
    pub enable: i64,
    /// Interface name, or `all`.
    #[serde(default)]
    pub interface: String,
    /// Label of the interface, `ALL` for all.
    #[serde(default)]
    pub label: String,
    /// Ports: `all`, a comma list, or a range `a-b`. Not sent for `sys`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ports: Option<String>,
    /// `src` or `dest`. Not sent for `sys`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// `tcp`, `udp` or `all`. Not sent for `sys`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
    /// Guaranteed rate in KB/s.
    #[serde(default)]
    pub min_rate: i64,
    /// Limit in KB/s.
    #[serde(default)]
    pub max_rate: i64,
    /// Names of the built-in services, for `sys`.
    #[serde(
        default,
        rename = "internal_Servers",
        skip_serializing_if = "Option::is_none"
    )]
    pub internal_servers: Option<Vec<String>>,
    /// Ids of the built-in sections, for `sys`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sections: Option<Vec<String>>,
}

/// The rules of one interface.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlowGroup {
    /// Interface name, or `all`.
    #[serde(default)]
    pub interface: String,
    /// Its label.
    #[serde(default)]
    pub label: String,
    /// The rules, in order of priority.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub list: Vec<FlowRule>,
}

/// `network/flow/get`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Flow {
    /// The groups.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub list: Vec<FlowGroup>,
    /// Version counter of the list.
    #[serde(default, skip_serializing)]
    pub ver: i64,
}

/// A built-in service the traffic control knows.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlowService {
    /// Section id.
    #[serde(default)]
    pub section: String,
    /// Name.
    #[serde(default)]
    pub name: String,
    /// Ports as a comma list.
    #[serde(default)]
    pub ports: String,
    /// Protocol.
    #[serde(default)]
    pub protocol: String,
}

// ── Wi-Fi ───────────────────────────────────────────────────────────

/// A wireless network, scanned or saved.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WifiNetwork {
    /// Name.
    #[serde(default)]
    pub ssid: String,
    /// Access point address.
    #[serde(default)]
    pub bssid: String,
    /// Connection status: `0` connected, `1` failed to get an address,
    /// `2` connecting, `3` not found, `5` check the router, `7` wrong password.
    #[serde(default)]
    pub status: i64,
    /// `off` for an open network.
    #[serde(default)]
    pub encryption_key: String,
    /// `1` when the network is known.
    #[serde(default)]
    pub exist: i64,
    /// Authentication string as the scan reports it.
    #[serde(default)]
    pub authentication: String,
    /// Channel.
    #[serde(default)]
    pub channel: i64,
    /// Signal strength.
    #[serde(default)]
    pub signal: i64,
    /// `1` when it joins by itself.
    #[serde(default)]
    pub auto_add: i64,
    /// `1` when it is saved.
    #[serde(default)]
    pub save_net: i64,
}

/// `network/wifi/list`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WifiList {
    /// What a scan found.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub ssid_list: Vec<WifiNetwork>,
    /// The saved networks.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub saved_list: Vec<WifiNetwork>,
    /// The network in use, if any.
    #[serde(default)]
    pub connection: Option<WifiNetwork>,
}

/// A new address that UGOS reports after a change that moved one.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewAddress {
    /// The address. Empty when the interface has none now.
    #[serde(default)]
    pub ipaddr: String,
}

/// The answer to a change of interfaces: the addresses they have now.
#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct AddressResult {
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub result: Vec<NewAddress>,
}

/// What `network/general/set` answers.
#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct GeneralSetReply {
    #[serde(default)]
    pub reboot: bool,
}

/// `network/wifi/connection`: how a connection attempt went.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WifiResult {
    /// `0` connected, `1` failed, `3` not found, `7` wrong password.
    #[serde(default)]
    pub status: i64,
    /// The network.
    #[serde(default)]
    pub ssid: String,
}

/// `network/bridge/status_virtual`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VirtualBridgeStatus {
    /// `0` init, `1` working, `2` done, `3` failed.
    #[serde(default)]
    pub status: i64,
    /// Per interface, with its own `status`.
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    pub interfaces: Vec<serde_json::Value>,
}
