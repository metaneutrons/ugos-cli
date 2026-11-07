//! Tests of the bond request.

use super::links::{BOND_MODES, BondIpv4, bond_body};

fn ports() -> Vec<String> {
    vec!["eth0".into(), "eth1".into()]
}

fn dhcp() -> BondIpv4 {
    BondIpv4 {
        protocol: "dhcp".into(),
        ..BondIpv4::default()
    }
}

#[test]
fn a_bond_body_is_the_one_the_wizard_builds() {
    let body = bond_body(&ports(), "balance-alb", &dhcp()).unwrap();
    assert_eq!(body["slaves"], serde_json::json!(["eth0", "eth1"]));
    assert_eq!(body["bond_mode"], "balance-alb");
    assert_eq!(body["ipv4"]["protocol"], "dhcp");
    assert_eq!(body["ipv4"]["dns"], serde_json::json!(["", ""]));
    assert_eq!(body["ipv4"]["route"], 0);
    assert!(body.get("ipv6").is_none());
}

#[test]
fn only_the_modes_of_the_ui_are_accepted() {
    for mode in BOND_MODES {
        assert!(bond_body(&ports(), mode, &dhcp()).is_ok(), "{mode}");
    }
    assert!(bond_body(&ports(), "broadcast", &dhcp()).is_err());
}

#[test]
fn a_static_bond_needs_a_valid_setup_with_a_gateway() {
    let mut ipv4 = BondIpv4 {
        protocol: "static".into(),
        address: "192.0.2.10".into(),
        netmask: "255.255.255.0".into(),
        gateway: "192.0.2.1".into(),
        dns: ["192.0.2.1".into(), String::new()],
        route: true,
    };
    let body = bond_body(&ports(), "802.3ad", &ipv4).unwrap();
    assert_eq!(body["ipv4"]["route"], 1);
    ipv4.gateway = String::new();
    assert!(bond_body(&ports(), "802.3ad", &ipv4).is_err());
    ipv4.gateway = "192.0.2.1".into();
    ipv4.dns[1] = "dns.example".into();
    assert!(bond_body(&ports(), "802.3ad", &ipv4).is_err());
    ipv4.protocol = "pppoe".into();
    assert!(bond_body(&ports(), "802.3ad", &ipv4).is_err());
}
