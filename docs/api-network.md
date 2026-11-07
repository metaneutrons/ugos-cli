# Network settings of the NAS

Interfaces, DNS, gateways, bonds, bridges, Wi-Fi and traffic control, from the
control panel (`com.ugreen.ctlmgr`, section Network). Not to be confused with
the KVM networks of `ugos-cli network`, which VMs attach to.

Reconstructed from the app's bundle, including its lazily loaded modals, and
read against a live NAS on UGOS 1.20 on 2026-11-07. What was and was not
tried live is under [Verification](#verification).

All paths are `v1/network/…`, plain transport. Bodies are JSON.

## Endpoints

| Method | Path | Command |
|--------|------|---------|
| GET | `iface/list?interface=` | `net list`, `net show` |
| POST | `iface/set` | `net set` |
| GET | `general/get` | `net general` |
| POST | `general/set` | `net dns`, `net proxy` |
| GET | `general/gateway_get` | `net gateway list` |
| POST | `general/gateway_set` | `net gateway order` |
| GET | `ipv6/nominate` | `net general` |
| GET | `bond/selectable`, `bridge/selectable` | `net bond candidates`, `net bridge candidates` |
| POST | `bond/create`, `bond/delete` | `net bond create`, `delete` |
| GET | `bond/status` | polled by `bond create` and `delete` |
| POST | `bridge/create`, `bridge/create_virtual`, `bridge/delete` | `net bridge …` |
| GET | `bridge/status`, `bridge/status_virtual` | polled by `net bridge …` |
| GET | `iface/has_firewall?interface=` | checked before deleting a bond or bridge |
| GET | `iface/compare_network?ifaces=a&ifaces=b` | checked before creating a bond |
| GET | `wifi/service_status`, `service_start`, `service_stop` (`?interface=`) | `net wifi status`, `on`, `off` |
| GET | `wifi/list?interface=&from_cache=0\|1` | `net wifi list` |
| GET | `wifi/status?interface=` | `net wifi status`, polled by `connect` |
| POST | `wifi/connection`, `wifi/switching`, `wifi/conn_editor`, `wifi/delete` | `net wifi connect`, `edit`, `forget` |
| GET | `flow/get` | `net flow list` |
| POST | `flow/set` | `net flow add`, `rm`, `enable`, `disable` |
| POST | `flow/application` | `net flow services` |
| | `iface/all`, `wifi/connect_net`, `auth/status`, `auth/upload`, `auth/test`, `auth/delete` | not wired: the web UI has wrappers and no caller |

`iface/list` takes `interface=all` (every interface), `valid_eth` (Ethernet
candidates), `valid_physics` (physical ports, bridge members included), `valid`
(interfaces for traffic rules) and `fw` (for firewall rules).

## Interfaces

`iface/list` answers `{ifaces: […]}`. An interface has `interface`, `label`,
`type`, `connection` (`1` cable connected), `mac`, `mtu`, `mtu_manual`, `speed`
(Mbit/s), `duplex`, `bond_mode`, `ipv4` and `ipv6` objects (`protocol`,
`ipaddr`, `netmask`, `gateway`, `dns`, `route`; IPv6 has `ip6addr` as
`addr/prefix` strings), `ieee8021x`, and `slaves` for a bond or bridge.

Types: Ethernet `0`, bond `1`, Wi-Fi `2`, PPPoE `3`, VPN `4`, bridge `5`,
virtual bridge `6`, Thunderbolt `11`. IPv4 protocols are `dhcp` and `static`,
and `ipv4ll` (link-local) on Thunderbolt. IPv6 protocols the form offers are
`dhcp`, `static` and `disable`.

### `iface/set`

The web UI builds a different body for each type, and `ugos-cli` mirrors it.
All carry `interface`.

| Type | Body |
|------|------|
| Ethernet | `mtu` (0 when not manual), `mtu_manual`, `vlan: 0`, `status: 0`, `ipv4 {protocol, ipaddr, netmask, gateway, dns[], route}`, `ipv6 {protocol, ipaddr: "addr/prefix", gateway, dns[], route}` |
| Bond | `bond_mode`, `ipv4` (the object as listed, spread, with the fields above on top), `ipv6` as above |
| Bridge | the whole listed item, with `ipv4` and `ipv6` replaced (`ipv6` adds `ip6addr`, `prefixValue`) |
| Wi-Fi | `ipv4` and `ipv6` as for Ethernet, no MTU |
| Thunderbolt | `status: 0`, `mtu_manual`, `mtu`, `ipv4` (spread, `protocol` `ipv4ll` or `static`, `ipaddr`, `netmask`), `ipv6 {protocol, ipaddr}` |

For an IPv6 that is not static the form sends `ipaddr: "/"`, which comes from
two empty inputs, and so does `ugos-cli`. The answer is `{result: [{ipaddr}]}`,
the addresses the NAS has afterwards. When the changed interface carried the
address a client is connected to, the NAS is reachable at a new one, and the
web UI redirects there.

Form rules, checked before sending: a static IPv4 needs address, netmask and
(except on Thunderbolt) a gateway; the netmask is ones then zeros; address and
gateway differ; DNS servers are IP addresses. A static IPv6 needs address,
gateway and a prefix of 1 to 128. MTU is 1100 to 9000 on Ethernet and 1100 to
62000 on Thunderbolt. `ugos-cli` refuses a change to the interface it is
connected through unless `--force` is given.

## DNS, gateways and proxy

`general/get` answers `{general, proxy}`. `general` has `dns` (two
servers), `dns_mode` (`0` automatic, `1` manual), `ipv4_gateway`,
`ipv6_gateway`, `multiple_gateways`, `prior_ipv4`, `clash_check`. `proxy` has
`enable`, `httpx_proxy`, `local_proxy`, `list` (`{protocol, host, port}`) and
`auth` (`{enable, username, password}`, the password in the clear).

`general/set` takes **the whole object back**, edited. It answers `{reboot}`;
when true UGOS asks for a restart, which the web UI offers and does not do.
The proxy section is hidden in the web UI build that was read, and the form
that is still in its code builds `{enable: 1, local_proxy, httpx_proxy, auth,
list: [{http}, {https}]}`. `httpx_proxy: 0` means one proxy for every protocol
(`protocol: "all"`), `1` separate ones for HTTP and HTTPS.

`gateway_get` answers `{ipv4: […], ipv6: […]}`; an entry has `gateway`,
`interface`, `label`, `priority`. `gateway_set` takes both arrays with `priority`
rewritten to the index. The first entry is the default gateway.

## Bonds and bridges

- `bond/selectable` and `bridge/selectable` answer `{ifaces: […]}`, the ports
  that could join. A bond needs two, a bridge needs two, a virtual bridge one.
- `bond/create`: `{slaves: [names], bond_mode, ipv4: {protocol, ipaddr, netmask,
  gateway, dns: [primary, alternate], route}}`. Modes: `balance-alb` (default),
  `active-backup`, `balance-rr`, `balance-xor`, `802.3ad`.
- `bridge/create`: `{slaves: [names], ipv4: {protocol: "dhcp", default_gateway:
  "0"}}`. `bridge/create_virtual`: `{interfaces: [names]}`.
- `bond/delete`, `bridge/delete`: `{interface}`.

They answer `{result: [{ipaddr}]}` like `iface/set`. The work is done in the
background: `bond/status` and `bridge/status` answer `{result}` (`0` init, `1`
working, `2` done, `3` failed), and `status_virtual` answers `{status,
interfaces: [{status}]}`. The web UI polls every 3 s, up to 20 times. A
firewall or traffic rule that names a bond or bridge goes with it
(`iface/has_firewall`, `flow/get`), and `compare_network` answers whether the
ports are in one network.

## Wi-Fi

`wifi/list` answers `ssid_list` (scan), `saved_list` and `connection`. A
network has `ssid`, `bssid`, `status`, `encryption_key` (`off` for an open
one), `exist`, `authentication`, `channel`, `signal`, `auto_add`, `save_net`.
Status: `0` connected, `1` no address, `2` connecting, `3` not found, `5` check
the router, `7` wrong password.

Connecting (`wifi/connection`) takes, for a secured network from the scan,
`{interface, ssid, bssid, channel, signal, authentication, password (8 or
more), auto_add: 1, save_net: 1, status: 0}`; for an open one the scan item
with `interface`, `auto_add`, `save_net`; for one added by hand `{interface,
ssid, authentication, encryption_key: "off"|"on", password, auto_add, save_net,
status, bssid: "", channel: 0, signal: 0}` with an authentication of
`[WPA-PSK-TKIP]`, `[WPA2-PSK+PSK-SHA256-TKIP+CCMP]`, the two together,
`[WPA2-SAE-CCMP]` (WPA3), WPA2 and WPA3 together, or `NONE`. Switching to a
saved network (`switching`) and forgetting one (`delete`) take `{interface,
ssid, bssid}`; editing one (`conn_editor`) adds `authentication`, `save_net`,
`auto_add`.

## Traffic control

`flow/get` answers `{list: [{interface, label, list: [rule]}], ver}`: groups by
interface, in priority order. **`interface` and `label` are on the group, not
on the rule.** `flow/set` takes `{list: […]}`, always the whole list; rules
carry `type`, `enable`, `interface`, `label`, `min_rate`, `max_rate` (KB/s) and,
by type: `all` has `ports: "all"`, `target: "src"`; `custom` has `ports`,
`target` (`src`/`dest`), `protocol` (`tcp`/`udp`/`all`); `sys` has
`internal_Servers` (names) and `sections` (ids) and no ports. UGOS reads back
empty `internal_Servers` and `sections` for a port rule, and `ugos-cli` drops
them before it sends the list again. A group that loses its last rule leaves
the request. Ports are a comma list of up to 15 or a range `a-b`. Rates are
digits, at most six, minimum not above maximum, and at least 1024 for the web
interface and SSH sections. `flow/application` takes `{section: [ids]}` and
answers `{list: [{section, name, ports, protocol}]}`.

## Verification

Read against a live NAS: interfaces, general settings, gateways, IPv6, bond and
bridge candidates and status, traffic rules, the built-in services.

Written against a live NAS, each undone or a no-op: `general/set` with the
settings unchanged; a traffic rule added, switched off and on, and removed
(which found that `interface` lives on the group); `iface/set` for the unused
Wi-Fi interface with DHCP as it already was.

**Not tried**, because a mistake can cut the NAS off or the NAS has no hardware
for it: `iface/set` for Ethernet, bond, bridge and Thunderbolt, `gateway_set`,
the proxy, bond and bridge create and delete, and everything about Wi-Fi (the
test NAS answers `9999` to the radio status). Their bodies follow the web UI's
code and are covered by unit tests only.

## Not known

- Whether the server accepts a smaller `iface/set` body for a bridge than the
  whole item.
- What `compare_network` answers other than `true` and `false`.
- The 802.1X endpoints, which the web UI wraps and never calls.
- What `valid_eth`, `valid_physics` and `valid` leave out.
