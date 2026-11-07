//! Arguments of the `net` commands: the network settings of the NAS itself.

use clap::{Args, Subcommand};

/// `net` subcommands. `network` is the KVM network of a VM.
#[derive(Debug, Subcommand)]
pub enum NetAction {
    /// List the interfaces.
    List,
    /// Show one interface.
    Show {
        /// Name or label, e.g. `bridge0` or `LAN1`.
        iface: String,
    },
    /// Change the setup of one interface.
    Set(SetArgs),
    /// Show DNS, gateways, IPv6 and the proxy.
    General,
    /// Change DNS or the general switches.
    Dns(DnsArgs),
    /// Default gateways and their order.
    Gateway {
        #[command(subcommand)]
        action: GatewayAction,
    },
    /// The proxy of the NAS.
    Proxy {
        #[command(subcommand)]
        action: ProxyAction,
    },
    /// Bonds (link aggregation).
    Bond {
        #[command(subcommand)]
        action: BondAction,
    },
    /// Bridges.
    Bridge {
        #[command(subcommand)]
        action: BridgeAction,
    },
    /// Wi-Fi.
    Wifi {
        #[command(subcommand)]
        action: WifiCommand,
    },
    /// Traffic control rules.
    Flow {
        #[command(subcommand)]
        action: FlowAction,
    },
}

/// What to set on an interface.
#[derive(Debug, Args)]
// One field per switch of the command line.
#[allow(clippy::struct_excessive_bools)]
pub struct SetArgs {
    /// Name or label of the interface.
    pub iface: String,
    /// Get an IPv4 address from DHCP.
    #[arg(long, conflicts_with_all = ["ip", "netmask", "gateway", "link_local"])]
    pub dhcp: bool,
    /// Use a link-local address (Thunderbolt only).
    #[arg(long, conflicts_with_all = ["ip", "netmask", "gateway"])]
    pub link_local: bool,
    /// Static IPv4 address.
    #[arg(long)]
    pub ip: Option<String>,
    /// Netmask of the static address, e.g. `255.255.255.0`.
    #[arg(long)]
    pub netmask: Option<String>,
    /// Gateway of the static address.
    #[arg(long)]
    pub gateway: Option<String>,
    /// DNS servers of the interface, comma separated; empty to clear.
    #[arg(long)]
    pub dns: Option<String>,
    /// Make the interface the default route.
    #[arg(long, conflicts_with = "no_default_route")]
    pub default_route: bool,
    /// Stop it being the default route.
    #[arg(long)]
    pub no_default_route: bool,
    /// IPv6: `auto`, `off` or `static`.
    #[arg(long, value_enum)]
    pub ipv6: Option<Ipv6Arg>,
    /// Static IPv6 address, without the prefix.
    #[arg(long)]
    pub ipv6_address: Option<String>,
    /// Static IPv6 prefix length.
    #[arg(long)]
    pub ipv6_prefix: Option<i64>,
    /// IPv6 gateway.
    #[arg(long)]
    pub ipv6_gateway: Option<String>,
    /// IPv6 DNS servers, comma separated.
    #[arg(long)]
    pub ipv6_dns: Option<String>,
    /// MTU in bytes, or `auto`.
    #[arg(long)]
    pub mtu: Option<String>,
    /// Bond mode, for a bond: `balance-alb`, `active-backup`, `balance-rr`,
    /// `balance-xor` or `802.3ad`.
    #[arg(long)]
    pub bond_mode: Option<String>,
    /// Go on although this is the interface the CLI talks through. A changed
    /// address cuts the session off.
    #[arg(long)]
    pub force: bool,
}

/// How IPv6 is set up.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum Ipv6Arg {
    /// Get an address automatically.
    Auto,
    /// Switch IPv6 off.
    Off,
    /// Use the address given.
    Static,
}

/// DNS and the general switches.
#[derive(Debug, Args)]
// One field per switch of the command line.
#[allow(clippy::struct_excessive_bools)]
pub struct DnsArgs {
    /// Take the DNS servers from DHCP.
    #[arg(long, conflicts_with = "server")]
    pub auto: bool,
    /// A DNS server; give it twice for a second one.
    #[arg(long, num_args = 1, action = clap::ArgAction::Append)]
    pub server: Vec<String>,
    /// Allow several default gateways.
    #[arg(long, conflicts_with = "single_gateway")]
    pub multiple_gateways: bool,
    /// Allow one default gateway only.
    #[arg(long)]
    pub single_gateway: bool,
    /// Prefer IPv4 over IPv6.
    #[arg(long, conflicts_with = "prefer_ipv6")]
    pub prefer_ipv4: bool,
    /// Prefer IPv6 over IPv4.
    #[arg(long)]
    pub prefer_ipv6: bool,
    /// Check for an address clash when an address changes.
    #[arg(long, conflicts_with = "no_clash_check")]
    pub clash_check: bool,
    /// Do not check for an address clash.
    #[arg(long)]
    pub no_clash_check: bool,
}

/// Gateway subcommands.
#[derive(Debug, Subcommand)]
pub enum GatewayAction {
    /// List the default gateways in order of preference.
    List,
    /// Put these interfaces first, in this order.
    Order {
        /// Interface names or labels, most preferred first.
        #[arg(required = true)]
        ifaces: Vec<String>,
        /// Order the IPv6 gateways.
        #[arg(long)]
        ipv6: bool,
    },
}

/// Proxy subcommands.
#[derive(Debug, Subcommand)]
pub enum ProxyAction {
    /// Show the proxy.
    Show,
    /// Switch the proxy off.
    Off,
    /// Set one proxy for every protocol, or one for HTTP and one for HTTPS.
    /// The proxy password is read from stdin with `--password-stdin`.
    Set {
        /// Proxy for every protocol, `IP:PORT`.
        #[arg(long, conflicts_with_all = ["http", "https"])]
        all: Option<String>,
        /// HTTP proxy, `IP:PORT`.
        #[arg(long)]
        http: Option<String>,
        /// HTTPS proxy, `IP:PORT`.
        #[arg(long)]
        https: Option<String>,
        /// Use the proxy for local addresses too.
        #[arg(long)]
        local: bool,
        /// User name of the proxy.
        #[arg(long, requires = "proxy_password_stdin")]
        user: Option<String>,
        /// Read the proxy password from stdin.
        #[arg(long)]
        proxy_password_stdin: bool,
    },
}

/// Bond subcommands.
#[derive(Debug, Subcommand)]
pub enum BondAction {
    /// List the ports that could join a bond.
    Candidates,
    /// Create a bond from two or more ports.
    Create {
        /// The ports, e.g. `eth0 eth1`.
        #[arg(required = true, num_args = 2..)]
        ports: Vec<String>,
        /// Mode: `balance-alb` (default), `active-backup`, `balance-rr`,
        /// `balance-xor` or `802.3ad`.
        #[arg(long, default_value = "balance-alb")]
        mode: String,
        /// Static IPv4 address; DHCP when left out.
        #[arg(long, requires_all = ["netmask", "gateway"])]
        ip: Option<String>,
        /// Netmask of the static address.
        #[arg(long)]
        netmask: Option<String>,
        /// Gateway of the static address.
        #[arg(long)]
        gateway: Option<String>,
        /// DNS server, up to two.
        #[arg(long, num_args = 1, action = clap::ArgAction::Append)]
        dns: Vec<String>,
        /// Make the bond the default route.
        #[arg(long)]
        default_route: bool,
        /// Go on although a port carries the address the CLI talks through.
        #[arg(long)]
        force: bool,
    },
    /// Delete a bond.
    Delete {
        /// The bond, e.g. `bond0`.
        bond: String,
        /// Go on although rules name it or it carries this session.
        #[arg(long)]
        force: bool,
    },
}

/// Bridge subcommands.
#[derive(Debug, Subcommand)]
pub enum BridgeAction {
    /// List the ports that could join a bridge.
    Candidates,
    /// Create a bridge from two or more ports.
    Create {
        /// The ports, e.g. `eth0 eth1`. Wi-Fi cannot be bridged.
        #[arg(required = true, num_args = 2..)]
        ports: Vec<String>,
        /// Go on although a port carries the address the CLI talks through.
        #[arg(long)]
        force: bool,
    },
    /// Create a virtual bridge, which VMs attach to.
    CreateVirtual {
        /// The ports.
        #[arg(required = true)]
        ports: Vec<String>,
    },
    /// Delete a bridge.
    Delete {
        /// The bridge, e.g. `bridge0`.
        bridge: String,
        /// Go on although rules name it or it carries this session.
        #[arg(long)]
        force: bool,
    },
}

/// Wi-Fi subcommands.
#[derive(Debug, Subcommand)]
pub enum WifiCommand {
    /// Show whether the radio is on, and the network in use.
    Status {
        /// The Wi-Fi interface, e.g. `wlan0`.
        iface: String,
    },
    /// Switch the radio on.
    On {
        /// The Wi-Fi interface.
        iface: String,
    },
    /// Switch the radio off.
    Off {
        /// The Wi-Fi interface.
        iface: String,
    },
    /// List the networks in range and the saved ones.
    List {
        /// The Wi-Fi interface.
        iface: String,
        /// Scan again instead of using the last scan.
        #[arg(long)]
        rescan: bool,
    },
    /// Connect to a network. A password is read from stdin with
    /// `--wifi-password-stdin`, or from `UGOS_WIFI_PASSWORD`.
    Connect {
        /// The Wi-Fi interface.
        iface: String,
        /// The network name.
        ssid: String,
        /// Authentication of a hidden network: `wpa`, `wpa2`, `wpa-wpa2`,
        /// `wpa3`, `wpa2-wpa3` or `none`.
        #[arg(long)]
        auth: Option<String>,
        /// Read the Wi-Fi password from stdin.
        #[arg(long)]
        wifi_password_stdin: bool,
    },
    /// Forget a saved network.
    Forget {
        /// The Wi-Fi interface.
        iface: String,
        /// The network name.
        ssid: String,
    },
    /// Change a saved network.
    Edit {
        /// The Wi-Fi interface.
        iface: String,
        /// The network name.
        ssid: String,
        /// Join this network by itself.
        #[arg(long, conflicts_with = "no_auto_join")]
        auto_join: bool,
        /// Do not join it by itself.
        #[arg(long)]
        no_auto_join: bool,
    },
}

/// Traffic control subcommands.
#[derive(Debug, Subcommand)]
pub enum FlowAction {
    /// List the rules in order of priority, counting from 1.
    List,
    /// List the built-in services a rule can name.
    Services,
    /// Add a rule at the end.
    Add {
        /// Interface name, or `all`.
        interface: String,
        /// Rule for every port.
        #[arg(long, conflicts_with_all = ["ports", "service"])]
        all_ports: bool,
        /// Ports: a comma list of up to 15, or a range `a-b`.
        #[arg(long)]
        ports: Option<String>,
        /// Built-in service by section id or name; repeat for several.
        #[arg(long, num_args = 1, action = clap::ArgAction::Append)]
        service: Vec<String>,
        /// `src` or `dest` port of a custom rule.
        #[arg(long, default_value = "dest")]
        target: String,
        /// `tcp`, `udp` or `all` for a custom rule.
        #[arg(long, default_value = "all")]
        protocol: String,
        /// Guaranteed rate in KB/s.
        #[arg(long, default_value_t = 0)]
        min_rate: i64,
        /// Limit in KB/s.
        #[arg(long)]
        max_rate: i64,
    },
    /// Remove the rule at this position.
    Rm {
        /// Position, counting from 1.
        position: usize,
    },
    /// Switch the rule at this position on.
    Enable {
        /// Position, counting from 1.
        position: usize,
    },
    /// Switch the rule at this position off.
    Disable {
        /// Position, counting from 1.
        position: usize,
    },
}
