//! `net bond`, `net bridge`, `net wifi` and `net flow`.

use std::io::Write;

use anyhow::{Result, bail};
use ugos_client::UgosClient;
use ugos_client::api::net::NetApi;
use ugos_client::ops::net::flow::{self, NewRule};
use ugos_client::ops::net::links::{self, BondIpv4};
use ugos_client::ops::net::wifi;
use ugos_client::types::net::NewAddress;

use super::net::split_list;
use super::share::with_force_hint;
use crate::cli::{BondAction, BridgeAction, FlowAction, OutputFormat, WifiCommand};
use crate::output;
use crate::output::net as rows;

/// Say that an interface job is done, and where the NAS is now.
fn finished(w: &mut impl Write, what: &str, moved: &[NewAddress], fmt: OutputFormat) -> Result<()> {
    let addresses: Vec<&str> = moved
        .iter()
        .map(|a| a.ipaddr.as_str())
        .filter(|a| !a.is_empty())
        .collect();
    let tail = if addresses.is_empty() {
        String::new()
    } else {
        format!(" (addresses now: {})", addresses.join(", "))
    };
    output::print_success(w, &format!("{what}{tail}"), fmt)
}

async fn ports(
    client: &UgosClient,
    bond: bool,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    let free = if bond {
        client.net_bond_candidates().await?
    } else {
        client.net_bridge_candidates().await?
    };
    match fmt {
        OutputFormat::Table => {
            let table: Vec<rows::PortRow> = free.iter().map(Into::into).collect();
            output::print_list(w, &table, fmt)
        }
        OutputFormat::Json => output::print_json(w, &free),
    }
}

/// Run a `net bond` subcommand.
pub async fn bond(
    client: &UgosClient,
    action: &BondAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    match action {
        BondAction::Candidates => ports(client, true, fmt, w).await,
        BondAction::Create {
            ports,
            mode,
            ip,
            netmask,
            gateway,
            dns,
            default_route,
            force,
        } => {
            let mut servers = dns.iter().cloned();
            let ipv4 = BondIpv4 {
                protocol: if ip.is_some() { "static" } else { "dhcp" }.into(),
                address: ip.clone().unwrap_or_default(),
                netmask: netmask.clone().unwrap_or_default(),
                gateway: gateway.clone().unwrap_or_default(),
                dns: [
                    servers.next().unwrap_or_default(),
                    servers.next().unwrap_or_default(),
                ],
                route: *default_route,
            };
            let moved = links::create_bond(client, ports, mode, &ipv4, *force)
                .await
                .map_err(with_force_hint)?;
            finished(w, "Created the bond", &moved, fmt)
        }
        BondAction::Delete { bond, force } => {
            let moved = links::delete_bond(client, bond, *force)
                .await
                .map_err(with_force_hint)?;
            finished(w, &format!("Deleted '{bond}'"), &moved, fmt)
        }
    }
}

/// Run a `net bridge` subcommand.
pub async fn bridge(
    client: &UgosClient,
    action: &BridgeAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    match action {
        BridgeAction::Candidates => ports(client, false, fmt, w).await,
        BridgeAction::Create { ports, force } => {
            let moved = links::create_bridge(client, ports, *force)
                .await
                .map_err(with_force_hint)?;
            finished(w, "Created the bridge", &moved, fmt)
        }
        BridgeAction::CreateVirtual { ports } => {
            let moved = links::create_virtual_bridge(client, ports).await?;
            finished(w, "Created the virtual bridge", &moved, fmt)
        }
        BridgeAction::Delete { bridge, force } => {
            let moved = links::delete_bridge(client, bridge, *force)
                .await
                .map_err(with_force_hint)?;
            finished(w, &format!("Deleted '{bridge}'"), &moved, fmt)
        }
    }
}

/// Run a `net wifi` subcommand.
pub async fn wifi(
    client: &UgosClient,
    action: &WifiCommand,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    match action {
        WifiCommand::Status { iface } => wifi_status(client, iface, fmt, w).await,
        WifiCommand::On { iface } | WifiCommand::Off { iface } => {
            let on = matches!(action, WifiCommand::On { .. });
            client.net_wifi_radio_set(iface, on).await?;
            output::print_success(
                w,
                &format!("Wi-Fi on '{iface}' is {}", if on { "on" } else { "off" }),
                fmt,
            )
        }
        WifiCommand::List { iface, rescan } => wifi_list(client, iface, *rescan, fmt, w).await,
        WifiCommand::Connect {
            iface,
            ssid,
            auth,
            wifi_password_stdin,
        } => {
            let password = wifi_password(client, iface, ssid, *wifi_password_stdin).await?;
            let done =
                wifi::connect(client, iface, ssid, password.as_deref(), auth.as_deref()).await?;
            output::print_success(w, &format!("Connected to '{}'", done.ssid), fmt)
        }
        WifiCommand::Forget { iface, ssid } => {
            wifi::forget(client, iface, ssid).await?;
            output::print_success(w, &format!("Forgot '{ssid}'"), fmt)
        }
        WifiCommand::Edit {
            iface,
            ssid,
            auto_join,
            no_auto_join,
        } => {
            let auto = super::share::flag_pair(*auto_join, *no_auto_join);
            if auto.is_none() {
                bail!("give --auto-join or --no-auto-join");
            }
            wifi::edit(client, iface, ssid, auto, None).await?;
            output::print_success(w, &format!("Updated '{ssid}'"), fmt)
        }
    }
}

/// A password is asked for only when the network is not saved and not open.
async fn wifi_password(
    client: &UgosClient,
    iface: &str,
    ssid: &str,
    from_stdin: bool,
) -> Result<Option<String>> {
    let list = client.net_wifi_list(iface, false).await?;
    let needs = !list.saved_list.iter().any(|n| n.ssid == ssid)
        && list
            .ssid_list
            .iter()
            .find(|n| n.ssid == ssid)
            .is_none_or(|n| n.encryption_key != "off");
    if !needs && !from_stdin {
        return Ok(None);
    }
    super::secret::read(
        "Wi-Fi password",
        "UGOS_WIFI_PASSWORD",
        "--wifi-password-stdin",
        from_stdin,
    )
    .map(Some)
}

async fn wifi_status(
    client: &UgosClient,
    iface: &str,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    let radio = client.net_wifi_radio(iface).await?;
    let now = client.net_wifi_status(iface).await?;
    match fmt {
        OutputFormat::Json => {
            output::print_json(w, &serde_json::json!({ "radio": radio, "connection": now }))
        }
        OutputFormat::Table => {
            writeln!(w, "Radio: {}", if radio { "on" } else { "off" })?;
            match now {
                Some(n) if n.ssid.is_empty() => writeln!(w, "Not connected")?,
                Some(n) if n.status == 0 => writeln!(w, "Connected to '{}'", n.ssid)?,
                Some(n) => writeln!(w, "Network '{}': {}", n.ssid, wifi::failure_text(n.status))?,
                None => writeln!(w, "Not connected")?,
            }
            Ok(())
        }
    }
}

async fn wifi_list(
    client: &UgosClient,
    iface: &str,
    rescan: bool,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    let list = client.net_wifi_list(iface, rescan).await?;
    match fmt {
        OutputFormat::Json => output::print_json(w, &list),
        OutputFormat::Table => {
            let table: Vec<rows::WifiRow> = list
                .ssid_list
                .iter()
                .chain(
                    list.saved_list
                        .iter()
                        .filter(|s| !list.ssid_list.iter().any(|n| n.ssid == s.ssid)),
                )
                .map(Into::into)
                .collect();
            output::print_list(w, &table, fmt)
        }
    }
}

/// Run a `net flow` subcommand.
pub async fn flow(
    client: &UgosClient,
    action: &FlowAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    let current = match action {
        FlowAction::List => client.net_flow().await?,
        FlowAction::Services => return flow_services(client, fmt, w).await,
        FlowAction::Add {
            interface,
            all_ports,
            ports,
            service,
            target,
            protocol,
            min_rate,
            max_rate,
        } => {
            let kind = if *all_ports {
                "all"
            } else if !service.is_empty() {
                "sys"
            } else if ports.is_some() {
                "custom"
            } else {
                bail!("give --all-ports, --ports or --service")
            };
            let rule = NewRule {
                interface: interface.clone(),
                kind: kind.into(),
                ports: ports.clone().unwrap_or_default(),
                target: target.clone(),
                protocol: protocol.clone(),
                min_rate: *min_rate,
                max_rate: *max_rate,
                sections: service.iter().flat_map(|s| split_list(s)).collect(),
            };
            flow::add_rule(client, &rule).await?
        }
        FlowAction::Rm { position } => flow::remove_rule(client, *position).await?,
        FlowAction::Enable { position } => flow::enable_rule(client, *position, true).await?,
        FlowAction::Disable { position } => flow::enable_rule(client, *position, false).await?,
    };
    match fmt {
        OutputFormat::Table => {
            let table: Vec<rows::FlowRow> = flow::flatten(&current)
                .iter()
                .enumerate()
                .map(Into::into)
                .collect();
            output::print_list(w, &table, fmt)
        }
        OutputFormat::Json => output::print_json(w, &current),
    }
}

async fn flow_services(client: &UgosClient, fmt: OutputFormat, w: &mut impl Write) -> Result<()> {
    let services = client.net_flow_services(&[]).await?;
    match fmt {
        OutputFormat::Table => {
            let table: Vec<rows::ServiceRow> = services.iter().map(Into::into).collect();
            output::print_list(w, &table, fmt)
        }
        OutputFormat::Json => output::print_json(w, &services),
    }
}
