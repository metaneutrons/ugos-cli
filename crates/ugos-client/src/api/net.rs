//! Network settings of the NAS itself: interfaces, DNS, gateways, proxy.
//!
//! These calls are `network/…` on the core API. The KVM networks are in
//! [`crate::api::kvm`]. See `docs/api-network.md`.

use serde::Deserialize;
use serde_json::{Value, json};

use crate::client::UgosClient;
use crate::error::{Result, UgosError};
use crate::types::common::null_vec_as_empty;
use crate::types::net::{
    AddressResult, Flow, FlowService, Gateways, GeneralSetReply, Iface, IfaceList, Ipv6Nominee,
    NetSettings, NewAddress, Selectable, Slave, VirtualBridgeStatus, WifiList, WifiNetwork,
    WifiResult,
};

/// Network reads.
#[allow(clippy::module_name_repetitions)]
pub trait NetApi {
    /// The interfaces.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on failure.
    fn net_ifaces(&self) -> impl Future<Output = Result<Vec<Iface>>> + Send;

    /// Find an interface by name or label.
    ///
    /// # Errors
    ///
    /// Returns [`UgosError::NotFound`] when none matches.
    fn net_iface(&self, which: &str) -> impl Future<Output = Result<Iface>> + Send;

    /// DNS, gateways and the proxy.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on failure.
    fn net_settings(&self) -> impl Future<Output = Result<NetSettings>> + Send;

    /// The default gateways.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on failure.
    fn net_gateways(&self) -> impl Future<Output = Result<Gateways>> + Send;

    /// The interface that carries IPv6.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on failure.
    fn net_ipv6(&self) -> impl Future<Output = Result<Ipv6Nominee>> + Send;

    /// Ports that could join a bond.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on failure.
    fn net_bond_candidates(&self) -> impl Future<Output = Result<Vec<Slave>>> + Send;

    /// Ports that could join a bridge.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on failure.
    fn net_bridge_candidates(&self) -> impl Future<Output = Result<Vec<Slave>>> + Send;

    /// Send the settings of one interface, in the shape the web UI sends.
    /// Returns the addresses the NAS has afterwards, which differ when the
    /// change moved the one this session talks to.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on failure.
    fn net_iface_set(&self, body: &Value) -> impl Future<Output = Result<Vec<NewAddress>>> + Send;

    /// Send DNS, gateway preference and proxy back. Returns whether UGOS asks
    /// for a reboot.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on failure.
    fn net_settings_set(&self, settings: &NetSettings)
    -> impl Future<Output = Result<bool>> + Send;

    /// Send the order of the default gateways.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on failure.
    fn net_gateways_set(&self, gateways: &Gateways) -> impl Future<Output = Result<()>> + Send;

    /// Whether a firewall rule names this interface.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on failure.
    fn net_has_firewall(&self, iface: &str) -> impl Future<Output = Result<bool>> + Send;

    /// Whether these interfaces sit in one network (`true`), as UGOS answers.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on failure.
    fn net_same_network(&self, ifaces: &[String]) -> impl Future<Output = Result<Value>> + Send;

    /// Create a bond. `body` is `{slaves, bond_mode, ipv4}`.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on failure.
    fn net_bond_create(&self, body: &Value)
    -> impl Future<Output = Result<Vec<NewAddress>>> + Send;

    /// Delete a bond.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on failure.
    fn net_bond_delete(&self, iface: &str) -> impl Future<Output = Result<Vec<NewAddress>>> + Send;

    /// Progress of a bond job: `0` init, `1` working, `2` done, `3` failed.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on failure.
    fn net_bond_status(&self) -> impl Future<Output = Result<i64>> + Send;

    /// Create a bridge from these ports.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on failure.
    fn net_bridge_create(
        &self,
        slaves: &[String],
    ) -> impl Future<Output = Result<Vec<NewAddress>>> + Send;

    /// Create a virtual bridge, which VMs attach to, from these ports.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on failure.
    fn net_bridge_create_virtual(
        &self,
        interfaces: &[String],
    ) -> impl Future<Output = Result<Vec<NewAddress>>> + Send;

    /// Delete a bridge.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on failure.
    fn net_bridge_delete(
        &self,
        iface: &str,
    ) -> impl Future<Output = Result<Vec<NewAddress>>> + Send;

    /// Progress of a bridge job: `0` init, `1` working, `2` done, `3` failed.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on failure.
    fn net_bridge_status(&self) -> impl Future<Output = Result<i64>> + Send;

    /// Progress of a virtual bridge job, with the state of each interface.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on failure.
    fn net_bridge_status_virtual(&self)
    -> impl Future<Output = Result<VirtualBridgeStatus>> + Send;

    /// Whether the radio of a Wi-Fi interface is on.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on failure.
    fn net_wifi_radio(&self, iface: &str) -> impl Future<Output = Result<bool>> + Send;

    /// Turn the radio of a Wi-Fi interface on or off.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on failure.
    fn net_wifi_radio_set(&self, iface: &str, on: bool) -> impl Future<Output = Result<()>> + Send;

    /// Scanned and saved Wi-Fi networks. `rescan` asks for a fresh scan.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on failure.
    fn net_wifi_list(
        &self,
        iface: &str,
        rescan: bool,
    ) -> impl Future<Output = Result<WifiList>> + Send;

    /// The network a Wi-Fi interface is on, if any.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on failure.
    fn net_wifi_status(
        &self,
        iface: &str,
    ) -> impl Future<Output = Result<Option<WifiNetwork>>> + Send;

    /// Connect to a Wi-Fi network. `body` is built by the caller as the web
    /// UI builds it; the password in it is sent as given.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on failure.
    fn net_wifi_connect(&self, body: &Value) -> impl Future<Output = Result<WifiResult>> + Send;

    /// Switch to a saved network, change one, or forget one: the web UI's
    /// `switching`, `conn_editor` and `delete` calls.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on failure.
    fn net_wifi_post(
        &self,
        action: WifiAction,
        body: &Value,
    ) -> impl Future<Output = Result<()>> + Send;

    /// The traffic control rules.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on failure.
    fn net_flow(&self) -> impl Future<Output = Result<Flow>> + Send;

    /// Replace the traffic control rules with these.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on failure.
    fn net_flow_set(&self, flow: &Flow) -> impl Future<Output = Result<()>> + Send;

    /// The built-in services the traffic control knows, with the sections
    /// already chosen marked.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on failure.
    fn net_flow_services(
        &self,
        sections: &[String],
    ) -> impl Future<Output = Result<Vec<FlowService>>> + Send;
}

/// The Wi-Fi calls that take a body and answer nothing of interest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WifiAction {
    /// Switch to a saved network.
    Switch,
    /// Change a saved network.
    Edit,
    /// Forget a saved network.
    Forget,
}

impl WifiAction {
    const fn path(self) -> &'static str {
        match self {
            Self::Switch => "network/wifi/switching",
            Self::Edit => "network/wifi/conn_editor",
            Self::Forget => "network/wifi/delete",
        }
    }
}

#[derive(Deserialize)]
struct Bare {
    #[serde(default)]
    result: i64,
}

#[derive(Deserialize)]
struct Flag {
    #[serde(default)]
    result: Value,
}

#[derive(Deserialize)]
struct RadioStatus {
    #[serde(default)]
    status: Value,
}

#[derive(Deserialize)]
struct WifiConnection {
    #[serde(default)]
    connection: Option<WifiNetwork>,
}

#[derive(Deserialize)]
struct ServiceList {
    #[serde(default, deserialize_with = "null_vec_as_empty")]
    list: Vec<FlowService>,
}

/// Pick an interface by name or label.
///
/// # Errors
///
/// Returns [`UgosError::NotFound`] when none matches.
pub fn pick_iface(ifaces: &[Iface], which: &str) -> Result<Iface> {
    ifaces
        .iter()
        .find(|iface| iface.interface == which || iface.label.eq_ignore_ascii_case(which))
        .cloned()
        .ok_or_else(|| UgosError::NotFound {
            kind: "interface",
            name: which.to_owned(),
        })
}

impl NetApi for UgosClient {
    async fn net_ifaces(&self) -> Result<Vec<Iface>> {
        let reply: IfaceList = self.get("network/iface/list").await?;
        Ok(reply.ifaces)
    }

    async fn net_iface(&self, which: &str) -> Result<Iface> {
        pick_iface(&self.net_ifaces().await?, which)
    }

    async fn net_settings(&self) -> Result<NetSettings> {
        self.get("network/general/get").await
    }

    async fn net_gateways(&self) -> Result<Gateways> {
        self.get("network/general/gateway_get").await
    }

    async fn net_ipv6(&self) -> Result<Ipv6Nominee> {
        self.get("network/ipv6/nominate").await
    }

    async fn net_bond_candidates(&self) -> Result<Vec<Slave>> {
        let reply: Selectable = self.get("network/bond/selectable").await?;
        Ok(reply.ifaces)
    }

    async fn net_bridge_candidates(&self) -> Result<Vec<Slave>> {
        let reply: Selectable = self.get("network/bridge/selectable").await?;
        Ok(reply.ifaces)
    }

    async fn net_iface_set(&self, body: &Value) -> Result<Vec<NewAddress>> {
        let reply: AddressResult = self.post("network/iface/set", body).await?;
        Ok(reply.result)
    }

    async fn net_settings_set(&self, settings: &NetSettings) -> Result<bool> {
        let reply: GeneralSetReply = self.post("network/general/set", settings).await?;
        Ok(reply.reboot)
    }

    async fn net_gateways_set(&self, gateways: &Gateways) -> Result<()> {
        let _: Value = self.post("network/general/gateway_set", gateways).await?;
        Ok(())
    }

    async fn net_has_firewall(&self, iface: &str) -> Result<bool> {
        let reply: Flag = self
            .get_with_params("network/iface/has_firewall", &[("interface", iface)])
            .await?;
        Ok(truthy(&reply.result))
    }

    async fn net_same_network(&self, ifaces: &[String]) -> Result<Value> {
        let params: Vec<(&str, &str)> = ifaces.iter().map(|i| ("ifaces", i.as_str())).collect();
        let reply: Flag = self
            .get_with_params("network/iface/compare_network", &params)
            .await?;
        Ok(reply.result)
    }

    async fn net_bond_create(&self, body: &Value) -> Result<Vec<NewAddress>> {
        let reply: AddressResult = self.post("network/bond/create", body).await?;
        Ok(reply.result)
    }

    async fn net_bond_delete(&self, iface: &str) -> Result<Vec<NewAddress>> {
        let reply: AddressResult = self
            .post("network/bond/delete", &json!({ "interface": iface }))
            .await?;
        Ok(reply.result)
    }

    async fn net_bond_status(&self) -> Result<i64> {
        let reply: Bare = self.get("network/bond/status").await?;
        Ok(reply.result)
    }

    async fn net_bridge_create(&self, slaves: &[String]) -> Result<Vec<NewAddress>> {
        let body =
            json!({ "slaves": slaves, "ipv4": { "protocol": "dhcp", "default_gateway": "0" } });
        let reply: AddressResult = self.post("network/bridge/create", &body).await?;
        Ok(reply.result)
    }

    async fn net_bridge_create_virtual(&self, interfaces: &[String]) -> Result<Vec<NewAddress>> {
        let reply: AddressResult = self
            .post(
                "network/bridge/create_virtual",
                &json!({ "interfaces": interfaces }),
            )
            .await?;
        Ok(reply.result)
    }

    async fn net_bridge_delete(&self, iface: &str) -> Result<Vec<NewAddress>> {
        let reply: AddressResult = self
            .post("network/bridge/delete", &json!({ "interface": iface }))
            .await?;
        Ok(reply.result)
    }

    async fn net_bridge_status(&self) -> Result<i64> {
        let reply: Bare = self.get("network/bridge/status").await?;
        Ok(reply.result)
    }

    async fn net_bridge_status_virtual(&self) -> Result<VirtualBridgeStatus> {
        self.get("network/bridge/status_virtual").await
    }

    async fn net_wifi_radio(&self, iface: &str) -> Result<bool> {
        let reply: RadioStatus = self
            .get_with_params("network/wifi/service_status", &[("interface", iface)])
            .await?;
        Ok(truthy(&reply.status))
    }

    async fn net_wifi_radio_set(&self, iface: &str, on: bool) -> Result<()> {
        let path = if on {
            "network/wifi/service_start"
        } else {
            "network/wifi/service_stop"
        };
        let _: Value = self.get_with_params(path, &[("interface", iface)]).await?;
        Ok(())
    }

    async fn net_wifi_list(&self, iface: &str, rescan: bool) -> Result<WifiList> {
        let cache = if rescan { "0" } else { "1" };
        self.get_with_params(
            "network/wifi/list",
            &[("interface", iface), ("from_cache", cache)],
        )
        .await
    }

    async fn net_wifi_status(&self, iface: &str) -> Result<Option<WifiNetwork>> {
        let reply: WifiConnection = self
            .get_with_params("network/wifi/status", &[("interface", iface)])
            .await?;
        Ok(reply.connection)
    }

    async fn net_wifi_connect(&self, body: &Value) -> Result<WifiResult> {
        self.post("network/wifi/connection", body).await
    }

    async fn net_wifi_post(&self, action: WifiAction, body: &Value) -> Result<()> {
        let _: Value = self.post(action.path(), body).await?;
        Ok(())
    }

    async fn net_flow(&self) -> Result<Flow> {
        self.get("network/flow/get").await
    }

    async fn net_flow_set(&self, flow: &Flow) -> Result<()> {
        let _: Value = self.post("network/flow/set", flow).await?;
        Ok(())
    }

    async fn net_flow_services(&self, sections: &[String]) -> Result<Vec<FlowService>> {
        let reply: ServiceList = self
            .post("network/flow/application", &json!({ "section": sections }))
            .await?;
        Ok(reply.list)
    }
}

/// UGOS answers booleans as `true`, `1`, or a non-empty string.
fn truthy(value: &Value) -> bool {
    match value {
        Value::Bool(flag) => *flag,
        Value::Number(number) => number.as_i64().is_some_and(|n| n != 0),
        Value::String(text) => !text.is_empty() && text != "0",
        _ => false,
    }
}
