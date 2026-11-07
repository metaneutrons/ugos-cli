//! DNS, the preference among gateways, and the proxy.
//!
//! UGOS wants the whole settings object back on a change, so each of these
//! reads it, edits a copy, sends it, and reads it again.

use super::validate;
use crate::api::net::NetApi;
use crate::client::UgosClient;
use crate::error::{Result, UgosError};
use crate::types::net::{Gateways, NetSettings, ProxyAuth, ProxyEntry};

/// DNS servers: automatic, or these one or two.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DnsChange {
    /// Take the servers that DHCP hands out.
    Auto,
    /// Use these servers, one or two.
    Servers(Vec<String>),
}

/// What to change in the general settings. `None` leaves a setting alone.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GeneralChange {
    /// DNS.
    pub dns: Option<DnsChange>,
    /// Allow several default gateways.
    pub multiple_gateways: Option<bool>,
    /// Prefer IPv4 over IPv6.
    pub prior_ipv4: Option<bool>,
    /// Check for an address clash on change.
    pub clash_check: Option<bool>,
}

impl GeneralChange {
    /// Whether nothing is to change.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// Apply a change to a copy of the settings.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] for DNS servers the web UI would refuse.
pub fn apply(settings: &NetSettings, change: &GeneralChange) -> Result<NetSettings> {
    let mut next = settings.clone();
    match &change.dns {
        Some(DnsChange::Auto) => next.general.dns_mode = 0,
        Some(DnsChange::Servers(servers)) => {
            if servers.is_empty() || servers.len() > 2 {
                return Err(UgosError::Refused("give one or two DNS servers".into()));
            }
            next.general.dns = servers
                .iter()
                .map(|server| validate::dns_server(server))
                .collect::<Result<_>>()?;
            next.general.dns_mode = 1;
        }
        None => {}
    }
    if let Some(flag) = change.multiple_gateways {
        next.general.multiple_gateways = i64::from(flag);
    }
    if let Some(flag) = change.prior_ipv4 {
        next.general.prior_ipv4 = i64::from(flag);
    }
    if let Some(flag) = change.clash_check {
        next.general.clash_check = i64::from(flag);
    }
    Ok(next)
}

/// Send the settings and read them back. Returns them with whether UGOS asks
/// for a reboot.
async fn send(client: &UgosClient, wanted: &NetSettings) -> Result<(NetSettings, bool)> {
    let reboot = client.net_settings_set(wanted).await?;
    let now = client.net_settings().await?;
    Ok((now, reboot))
}

/// Change DNS or the general switches, and check that UGOS kept them.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] when there is nothing to change, for input
/// the web UI would refuse, or for a change UGOS did not keep.
pub async fn set_general(
    client: &UgosClient,
    change: &GeneralChange,
) -> Result<(NetSettings, bool)> {
    if change.is_empty() {
        return Err(UgosError::Refused("nothing to change".into()));
    }
    let before = client.net_settings().await?;
    let wanted = apply(&before, change)?;
    let (now, reboot) = send(client, &wanted).await?;
    let g = (&now.general, &wanted.general);
    let kept = g.0.dns_mode == g.1.dns_mode
        && (g.1.dns_mode == 0 || g.0.dns == g.1.dns)
        && g.0.multiple_gateways == g.1.multiple_gateways
        && g.0.prior_ipv4 == g.1.prior_ipv4
        && g.0.clash_check == g.1.clash_check;
    if kept {
        Ok((now, reboot))
    } else {
        Err(UgosError::Refused(
            "UGOS accepted the request but the settings did not change".into(),
        ))
    }
}

/// Put the default gateways in this order of preference, by interface name or
/// label. Gateways not named follow in their old order.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] for an interface with no gateway, or an
/// order UGOS did not keep.
pub async fn set_gateway_order(
    client: &UgosClient,
    ipv6: bool,
    order: &[String],
) -> Result<Gateways> {
    let mut gateways = client.net_gateways().await?;
    let list = if ipv6 {
        &mut gateways.ipv6
    } else {
        &mut gateways.ipv4
    };
    let mut sorted = Vec::with_capacity(list.len());
    for name in order {
        let at = list
            .iter()
            .position(|e| e.interface == *name || e.label.eq_ignore_ascii_case(name))
            .ok_or_else(|| UgosError::Refused(format!("'{name}' has no gateway to order")))?;
        sorted.push(list.remove(at));
    }
    sorted.append(list);
    for (index, entry) in sorted.iter_mut().enumerate() {
        entry.priority = i64::try_from(index).unwrap_or(0);
    }
    let wanted: Vec<String> = sorted.iter().map(|e| e.interface.clone()).collect();
    *list = sorted;
    client.net_gateways_set(&gateways).await?;
    let now = client.net_gateways().await?;
    let seen: Vec<String> = if ipv6 { &now.ipv6 } else { &now.ipv4 }
        .iter()
        .map(|e| e.interface.clone())
        .collect();
    if seen == wanted {
        Ok(now)
    } else {
        Err(UgosError::Refused(
            "UGOS accepted the request but the order did not change".into(),
        ))
    }
}

/// The proxy of the NAS. The web UI hides this part of the page in the
/// build it was read from, so it follows the form that is still in the code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProxyChange {
    /// Switch the proxy off.
    Off,
    /// One proxy for every protocol.
    All {
        /// Host, an IP address.
        host: String,
        /// Port.
        port: i64,
        /// Whether local addresses use it too.
        local: bool,
        /// Login, as `(user, password)`.
        auth: Option<(String, String)>,
    },
    /// A proxy each for HTTP and HTTPS, at least one of them.
    Split {
        /// HTTP proxy as `(host, port)`.
        http: Option<(String, i64)>,
        /// HTTPS proxy as `(host, port)`.
        https: Option<(String, i64)>,
        /// Whether local addresses use it too.
        local: bool,
        /// Login, as `(user, password)`.
        auth: Option<(String, String)>,
    },
}

fn entry(protocol: &str, hosted: &(String, i64)) -> Result<ProxyEntry> {
    let _ = validate::ipv4(&hosted.0, "proxy host")?;
    Ok(ProxyEntry {
        host: hosted.0.clone(),
        port: validate::port(hosted.1)?,
        protocol: protocol.into(),
    })
}

fn auth_of(login: Option<&(String, String)>) -> Result<ProxyAuth> {
    match login {
        None => Ok(ProxyAuth::default()),
        Some((user, password)) if !user.is_empty() && !password.is_empty() => Ok(ProxyAuth {
            enable: 1,
            username: user.clone(),
            password: password.clone(),
        }),
        Some(_) => Err(UgosError::Refused(
            "a proxy login needs a user name and a password".into(),
        )),
    }
}

/// Apply a proxy change to a copy of the settings.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] for input the web UI would refuse.
pub fn apply_proxy(settings: &NetSettings, change: &ProxyChange) -> Result<NetSettings> {
    let mut next = settings.clone();
    let proxy = &mut next.proxy;
    match change {
        ProxyChange::Off => proxy.enable = 0,
        ProxyChange::All {
            host,
            port,
            local,
            auth,
        } => {
            proxy.enable = 1;
            proxy.httpx_proxy = 0;
            proxy.local_proxy = i64::from(*local);
            proxy.list = vec![entry("all", &(host.clone(), *port))?];
            proxy.auth = auth_of(auth.as_ref())?;
        }
        ProxyChange::Split {
            http,
            https,
            local,
            auth,
        } => {
            if http.is_none() && https.is_none() {
                return Err(UgosError::Refused("give an HTTP or an HTTPS proxy".into()));
            }
            proxy.enable = 1;
            proxy.httpx_proxy = 1;
            proxy.local_proxy = i64::from(*local);
            proxy.list = [("http", http), ("https", https)]
                .iter()
                .filter_map(|(name, hosted)| hosted.as_ref().map(|h| entry(name, h)))
                .collect::<Result<_>>()?;
            proxy.auth = auth_of(auth.as_ref())?;
        }
    }
    Ok(next)
}

/// Change the proxy and check that UGOS kept it.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] for input the web UI would refuse, or a
/// change UGOS did not keep.
pub async fn set_proxy(client: &UgosClient, change: &ProxyChange) -> Result<(NetSettings, bool)> {
    let before = client.net_settings().await?;
    let wanted = apply_proxy(&before, change)?;
    let (now, reboot) = send(client, &wanted).await?;
    if now.proxy.enable == wanted.proxy.enable && now.proxy.list.len() == wanted.proxy.list.len() {
        Ok((now, reboot))
    } else {
        Err(UgosError::Refused(
            "UGOS accepted the request but the proxy did not change".into(),
        ))
    }
}
