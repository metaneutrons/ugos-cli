//! `net`: the network settings of the NAS itself.
//!
//! The checks and the read-back live in `ugos_client::ops::net`. This file
//! maps the command line onto them and prints.

use std::io::Write;

use anyhow::{Result, anyhow, bail};
use ugos_client::UgosClient;
use ugos_client::api::net::NetApi;
use ugos_client::ops::net::general::{
    DnsChange, GeneralChange, ProxyChange, set_gateway_order, set_general, set_proxy,
};
use ugos_client::ops::net::iface::{IfaceChange, Ipv6Mode, Mtu, set_iface};

use super::net_links;
use super::share::{flag_pair, with_force_hint};
use crate::cli::{DnsArgs, GatewayAction, Ipv6Arg, NetAction, OutputFormat, ProxyAction, SetArgs};
use crate::output;
use crate::output::net as rows;

/// Run a `net` subcommand.
pub async fn net(
    client: &UgosClient,
    action: &NetAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    match action {
        NetAction::List => list(client, fmt, w).await,
        NetAction::Show { iface } => show(client, iface, fmt, w).await,
        NetAction::Set(args) => set(client, args, fmt, w).await,
        NetAction::General => general(client, fmt, w).await,
        NetAction::Dns(args) => dns(client, args, fmt, w).await,
        NetAction::Gateway { action } => gateway(client, action, fmt, w).await,
        NetAction::Proxy { action } => proxy(client, action, fmt, w).await,
        NetAction::Bond { action } => net_links::bond(client, action, fmt, w).await,
        NetAction::Bridge { action } => net_links::bridge(client, action, fmt, w).await,
        NetAction::Wifi { action } => net_links::wifi(client, action, fmt, w).await,
        NetAction::Flow { action } => net_links::flow(client, action, fmt, w).await,
    }
}

async fn list(client: &UgosClient, fmt: OutputFormat, w: &mut impl Write) -> Result<()> {
    let ifaces = client.net_ifaces().await?;
    match fmt {
        OutputFormat::Table => {
            let table: Vec<rows::IfaceRow> = ifaces.iter().map(Into::into).collect();
            output::print_list(w, &table, fmt)
        }
        OutputFormat::Json => output::print_json(w, &ifaces),
    }
}

async fn show(
    client: &UgosClient,
    which: &str,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    let iface = client.net_iface(which).await?;
    match fmt {
        OutputFormat::Table => output::print_list(w, &rows::iface_rows(&iface), fmt),
        OutputFormat::Json => output::print_json(w, &iface),
    }
}

/// Split a comma separated list. An empty text is an empty list.
pub(super) fn split_list(text: &str) -> Vec<String> {
    text.split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_owned)
        .collect()
}

fn parse_mtu(text: &str) -> Result<Mtu> {
    if text.eq_ignore_ascii_case("auto") {
        return Ok(Mtu::Auto);
    }
    text.parse()
        .map(Mtu::Value)
        .map_err(|_| anyhow!("the MTU '{text}' is a number or 'auto'"))
}

fn ipv4_protocol(args: &SetArgs) -> Option<String> {
    if args.dhcp {
        Some("dhcp".into())
    } else if args.link_local {
        Some("ipv4ll".into())
    } else if args.ip.is_some() || args.netmask.is_some() || args.gateway.is_some() {
        Some("static".into())
    } else {
        None
    }
}

/// What a `net set` command line asks for.
fn change_from(args: &SetArgs) -> Result<IfaceChange> {
    Ok(IfaceChange {
        ipv4_protocol: ipv4_protocol(args),
        ipv4_address: args.ip.clone(),
        ipv4_netmask: args.netmask.clone(),
        ipv4_gateway: args.gateway.clone(),
        ipv4_dns: args.dns.as_deref().map(split_list),
        ipv4_route: flag_pair(args.default_route, args.no_default_route),
        ipv6_mode: args.ipv6.map(|mode| match mode {
            Ipv6Arg::Auto => Ipv6Mode::Auto,
            Ipv6Arg::Off => Ipv6Mode::Off,
            Ipv6Arg::Static => Ipv6Mode::Static,
        }),
        ipv6_address: args.ipv6_address.clone(),
        ipv6_prefix: args.ipv6_prefix,
        ipv6_gateway: args.ipv6_gateway.clone(),
        ipv6_dns: args.ipv6_dns.as_deref().map(split_list),
        ipv6_route: None,
        mtu: args.mtu.as_deref().map(parse_mtu).transpose()?,
        bond_mode: args.bond_mode.clone(),
    })
}

async fn set(
    client: &UgosClient,
    args: &SetArgs,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    let change = change_from(args)?;
    if change.is_empty() {
        bail!("nothing to change: see `net set --help`");
    }
    let outcome = set_iface(client, &args.iface, &change, args.force)
        .await
        .map_err(with_force_hint)?;
    match (fmt, &outcome.iface) {
        (OutputFormat::Json, iface) => output::print_json(
            w,
            &serde_json::json!({
                "iface": iface, "new_addresses": outcome.new_addresses,
            }),
        ),
        (OutputFormat::Table, Some(iface)) => {
            writeln!(w, "Updated '{}'", iface.interface)?;
            output::print_list(w, &rows::iface_rows(iface), fmt)
        }
        (OutputFormat::Table, None) => {
            let moved: Vec<&str> = outcome
                .new_addresses
                .iter()
                .map(|a| a.ipaddr.as_str())
                .filter(|a| !a.is_empty())
                .collect();
            writeln!(
                w,
                "Sent. The NAS could not be read again; its address may have moved to: {}",
                moved.join(", ")
            )?;
            Ok(())
        }
    }
}

async fn general(client: &UgosClient, fmt: OutputFormat, w: &mut impl Write) -> Result<()> {
    let settings = client.net_settings().await?;
    let gateways = client.net_gateways().await?;
    let ipv6 = client.net_ipv6().await?;
    match fmt {
        OutputFormat::Table => {
            output::print_list(w, &rows::general_rows(&settings, &gateways, &ipv6), fmt)
        }
        OutputFormat::Json => output::print_json(
            w,
            &serde_json::json!({
                "dns": settings.general.dns, "dns_mode": settings.general.dns_mode,
                "gateways": gateways, "ipv6": ipv6,
                "proxy": { "enable": settings.proxy.enable, "list": settings.proxy.list },
            }),
        ),
    }
}

fn dns_change(args: &DnsArgs) -> GeneralChange {
    let dns = if args.auto {
        Some(DnsChange::Auto)
    } else if args.server.is_empty() {
        None
    } else {
        Some(DnsChange::Servers(args.server.clone()))
    };
    GeneralChange {
        dns,
        multiple_gateways: flag_pair(args.multiple_gateways, args.single_gateway),
        prior_ipv4: flag_pair(args.prefer_ipv4, args.prefer_ipv6),
        clash_check: flag_pair(args.clash_check, args.no_clash_check),
    }
}

async fn dns(
    client: &UgosClient,
    args: &DnsArgs,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    let (now, reboot) = set_general(client, &dns_change(args)).await?;
    match fmt {
        OutputFormat::Table => {
            writeln!(w, "Updated the general network settings")?;
            if reboot {
                writeln!(w, "UGOS asks for a reboot to apply them.")?;
            }
            output::print_list(
                w,
                &rows::general_rows(
                    &now,
                    &client.net_gateways().await?,
                    &client.net_ipv6().await?,
                ),
                fmt,
            )
        }
        OutputFormat::Json => output::print_json(
            w,
            &serde_json::json!({ "reboot": reboot, "dns": now.general.dns, "dns_mode": now.general.dns_mode }),
        ),
    }
}

async fn gateway(
    client: &UgosClient,
    action: &GatewayAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    let gateways = match action {
        GatewayAction::List => client.net_gateways().await?,
        GatewayAction::Order { ifaces, ipv6 } => set_gateway_order(client, *ipv6, ifaces).await?,
    };
    match fmt {
        OutputFormat::Table => {
            let table: Vec<rows::GatewayRow> =
                gateways.ipv4.iter().enumerate().map(Into::into).collect();
            output::print_list(w, &table, fmt)?;
            let v6: Vec<rows::GatewayRow> =
                gateways.ipv6.iter().enumerate().map(Into::into).collect();
            if !v6.is_empty() {
                output::print_list(w, &v6, fmt)?;
            }
            Ok(())
        }
        OutputFormat::Json => output::print_json(w, &gateways),
    }
}

/// `IP:PORT` as a pair.
fn host_port(text: &str) -> Result<(String, i64)> {
    let (host, port) = text
        .rsplit_once(':')
        .ok_or_else(|| anyhow!("'{text}' is not IP:PORT"))?;
    let port = port
        .parse()
        .map_err(|_| anyhow!("'{port}' is not a port"))?;
    Ok((host.to_owned(), port))
}

async fn proxy(
    client: &UgosClient,
    action: &ProxyAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    let change = match action {
        ProxyAction::Show => {
            let settings = client.net_settings().await?;
            return match fmt {
                OutputFormat::Table => output::print_list(
                    w,
                    &rows::general_rows(
                        &settings,
                        &client.net_gateways().await?,
                        &client.net_ipv6().await?,
                    ),
                    fmt,
                ),
                OutputFormat::Json => output::print_json(
                    w,
                    &serde_json::json!({ "enable": settings.proxy.enable, "list": settings.proxy.list }),
                ),
            };
        }
        ProxyAction::Off => ProxyChange::Off,
        ProxyAction::Set {
            all,
            http,
            https,
            local,
            user,
            proxy_password_stdin,
        } => {
            let auth = match user {
                Some(user) => Some((
                    user.clone(),
                    super::secret::read(
                        "proxy password",
                        "UGOS_PROXY_PASSWORD",
                        "--proxy-password-stdin",
                        *proxy_password_stdin,
                    )?,
                )),
                None => None,
            };
            match (all, http, https) {
                (Some(all), None, None) => ProxyChange::All {
                    host: host_port(all)?.0,
                    port: host_port(all)?.1,
                    local: *local,
                    auth,
                },
                (None, h, s) if h.is_some() || s.is_some() => ProxyChange::Split {
                    http: h.as_deref().map(host_port).transpose()?,
                    https: s.as_deref().map(host_port).transpose()?,
                    local: *local,
                    auth,
                },
                _ => bail!("give --all, or --http and/or --https"),
            }
        }
    };
    let (now, reboot) = set_proxy(client, &change).await?;
    output::print_success(
        w,
        &format!(
            "Proxy {}{}",
            if now.proxy.enable == 1 { "set" } else { "off" },
            if reboot {
                "; UGOS asks for a reboot"
            } else {
                ""
            }
        ),
        fmt,
    )
}
