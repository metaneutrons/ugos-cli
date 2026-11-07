//! Tests of the network input rules.

use super::validate::*;

#[test]
fn hosts_are_dotted_ipv4_with_a_first_octet() {
    assert!(ipv4("192.168.1.10", "address").is_ok());
    assert!(ipv4("0.0.0.0", "address").is_err());
    assert!(ipv4("10.0.0", "address").is_err());
    assert!(ipv4("a.b.c.d", "address").is_err());
    assert!(ipv4("0.1.2.3", "address").is_err());
}

#[test]
fn a_netmask_is_ones_then_zeros() {
    for good in [
        "255.255.255.0",
        "255.255.240.0",
        "128.0.0.0",
        "255.255.255.255",
    ] {
        assert!(netmask(good).is_ok(), "{good}");
    }
    for bad in [
        "0.0.0.0",
        "255.0.255.0",
        "255.255.255.1",
        "192.168.1.1",
        "1.2.3",
    ] {
        assert!(netmask(bad).is_err(), "{bad}");
    }
}

#[test]
fn a_static_setup_needs_a_gateway_that_differs() {
    assert!(static_ipv4("192.168.1.10", "255.255.255.0", Some("192.168.1.1")).is_ok());
    assert!(static_ipv4("192.168.1.10", "255.255.255.0", None).is_ok());
    assert!(static_ipv4("192.168.1.10", "255.255.255.0", Some("192.168.1.10")).is_err());
    assert!(static_ipv4("192.168.1.10", "255.0.255.0", None).is_err());
    assert!(static_ipv4("0.0.0.0", "255.255.255.0", None).is_err());
}

#[test]
fn ipv6_refuses_the_addresses_the_ui_refuses() {
    assert!(ipv6("2001:db8::5", "address").is_ok());
    assert!(ipv6("2001:db8::5/64", "address").is_ok());
    for bad in ["::", "::1", "ff02::1", "not-an-address", "192.168.1.1"] {
        assert!(ipv6(bad, "address").is_err(), "{bad}");
    }
    assert!(prefix(64).is_ok());
    assert!(prefix(0).is_err());
    assert!(prefix(129).is_err());
}

#[test]
fn a_dns_server_is_an_ip_address_of_either_family() {
    assert_eq!(dns_server(" 192.168.1.1 ").unwrap(), "192.168.1.1");
    assert_eq!(
        dns_server("2001:4860:4860::8888").unwrap(),
        "2001:4860:4860::8888"
    );
    assert!(dns_server("dns.example").is_err());
    assert!(dns_server("0.0.0.0").is_err());
}

#[test]
fn mtu_and_port_ranges() {
    assert!(mtu(1500, 9000).is_ok());
    assert!(mtu(1000, 9000).is_err());
    assert!(mtu(9001, 9000).is_err());
    assert!(mtu(62000, 62000).is_ok());
    assert!(port(1).is_ok() && port(65535).is_ok());
    assert!(port(0).is_err() && port(65536).is_err());
}
