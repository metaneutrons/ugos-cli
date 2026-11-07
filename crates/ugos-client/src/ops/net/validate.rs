//! The input rules of the web UI's network forms, so that a request UGOS
//! would refuse in the browser is refused here, in words.

use std::net::{Ipv4Addr, Ipv6Addr};

use crate::error::{Result, UgosError};

const fn refused(message: String) -> UgosError {
    UgosError::Refused(message)
}

/// A dotted IPv4 address that the UI accepts for a host: four numbers, a
/// first one from 1 to 255, and not `0.0.0.0`.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] with the reason.
pub fn ipv4(text: &str, what: &str) -> Result<Ipv4Addr> {
    let address: Ipv4Addr = text
        .trim()
        .parse()
        .map_err(|_| refused(format!("{what} '{text}' is not an IPv4 address")))?;
    if address.octets()[0] == 0 {
        return Err(refused(format!("{what} '{text}' is not a usable address")));
    }
    Ok(address)
}

/// A netmask: 128.0.0.0 up to 255.255.255.255, ones first, then zeros.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] with the reason.
pub fn netmask(text: &str) -> Result<Ipv4Addr> {
    let mask: Ipv4Addr = text
        .trim()
        .parse()
        .map_err(|_| refused(format!("netmask '{text}' is not an IPv4 address")))?;
    let bits = u32::from(mask);
    if bits == 0 || bits.leading_ones() + bits.trailing_zeros() != 32 {
        return Err(refused(format!("'{text}' is not a valid netmask")));
    }
    Ok(mask)
}

/// A static IPv4 setup: address, mask and gateway that fit together.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] with the reason.
pub fn static_ipv4(address: &str, mask: &str, gateway: Option<&str>) -> Result<()> {
    let ip = ipv4(address, "address")?;
    let _ = netmask(mask)?;
    if u32::from(ip).count_ones() == 0 || ip.is_broadcast() {
        return Err(refused(format!("address '{address}' is not usable")));
    }
    if let Some(gateway) = gateway.filter(|g| !g.is_empty()) {
        let gw = ipv4(gateway, "gateway")?;
        if gw == ip {
            return Err(refused("the gateway must differ from the address".into()));
        }
    }
    Ok(())
}

/// An IPv6 address, without a prefix, that the UI accepts for a host.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] with the reason.
pub fn ipv6(text: &str, what: &str) -> Result<Ipv6Addr> {
    let trimmed = text.trim().trim_end_matches("/64").trim_end_matches("/128");
    let address: Ipv6Addr = trimmed
        .parse()
        .map_err(|_| refused(format!("{what} '{text}' is not an IPv6 address")))?;
    if address.is_unspecified() || address.is_loopback() || address.is_multicast() {
        return Err(refused(format!("{what} '{text}' is not a usable address")));
    }
    Ok(address)
}

/// A prefix length for IPv6.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] when it is not 1 to 128.
pub fn prefix(length: i64) -> Result<i64> {
    if (1..=128).contains(&length) {
        Ok(length)
    } else {
        Err(refused(format!(
            "the prefix length {length} is not 1 to 128"
        )))
    }
}

/// A DNS server: IPv4 or IPv6.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] when it is neither.
pub fn dns_server(text: &str) -> Result<String> {
    let trimmed = text.trim();
    if trimmed.parse::<Ipv4Addr>().is_ok() && ipv4(trimmed, "DNS server").is_ok() {
        return Ok(trimmed.to_owned());
    }
    if trimmed.parse::<Ipv6Addr>().is_ok() {
        return Ok(trimmed.to_owned());
    }
    Err(refused(format!("DNS server '{text}' is not an IP address")))
}

/// An MTU in the range the UI allows for an interface kind.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] when it is out of range.
pub fn mtu(value: i64, max: i64) -> Result<i64> {
    if (1100..=max).contains(&value) {
        Ok(value)
    } else {
        Err(refused(format!("the MTU {value} is not 1100 to {max}")))
    }
}

/// A port from 1 to 65535.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] when it is out of range.
pub fn port(value: i64) -> Result<i64> {
    if (1..=65535).contains(&value) {
        Ok(value)
    } else {
        Err(refused(format!("the port {value} is not 1 to 65535")))
    }
}
