//! Tests of the interface change and the bodies it builds.

use serde_json::json;

use super::iface::{IfaceChange, Ipv6Mode, Mtu, apply, body};
use crate::types::net::Iface;

fn iface(kind: i64) -> Iface {
    serde_json::from_value(json!({
        "interface": "eth0", "label": "LAN1", "type": kind, "mtu": 1500, "mtu_manual": 0,
        "bond_mode": "balance-alb",
        "ipv4": {"protocol": "dhcp", "ipaddr": "192.0.2.5", "netmask": "255.255.255.0",
                 "gateway": "192.0.2.1", "dns": ["192.0.2.1"], "route": 1, "status": 0,
                 "future_field": 7},
        "ipv6": {"protocol": "dhcp", "ip6addr": ["2001:db8::5/64"], "dns": null, "route": 0},
        "slaves": [{"interface": "eth1", "label": "LAN2"}],
        "unknown_item_field": "kept"
    }))
    .unwrap()
}

fn static_v4() -> IfaceChange {
    IfaceChange {
        ipv4_protocol: Some("static".into()),
        ipv4_address: Some("192.0.2.10".into()),
        ipv4_netmask: Some("255.255.255.0".into()),
        ipv4_gateway: Some("192.0.2.1".into()),
        ..IfaceChange::default()
    }
}

#[test]
fn a_static_address_is_checked_and_applied() {
    let next = apply(&iface(0), &static_v4()).unwrap();
    assert_eq!(next.ipv4.protocol, "static");
    assert_eq!(next.ipv4.ipaddr, "192.0.2.10");
    let mut bad = static_v4();
    bad.ipv4_netmask = Some("255.0.255.0".into());
    assert!(apply(&iface(0), &bad).is_err());
    let mut same = static_v4();
    same.ipv4_gateway = Some("192.0.2.10".into());
    assert!(apply(&iface(0), &same).is_err());
}

#[test]
fn a_static_address_without_a_gateway_is_refused_except_on_thunderbolt() {
    let mut change = static_v4();
    change.ipv4_gateway = Some(String::new());
    let mut plain = iface(0);
    plain.ipv4.gateway = String::new();
    assert!(apply(&plain, &change).is_err());
    let mut tb = iface(11);
    tb.ipv4.gateway = String::new();
    assert!(apply(&tb, &change).is_ok());
}

#[test]
fn ipv4ll_is_for_thunderbolt_only() {
    let change = IfaceChange {
        ipv4_protocol: Some("ipv4ll".into()),
        ..IfaceChange::default()
    };
    assert!(apply(&iface(0), &change).is_err());
    assert!(apply(&iface(11), &change).is_ok());
}

#[test]
fn ipv6_static_needs_address_prefix_and_gateway() {
    let change = IfaceChange {
        ipv6_mode: Some(Ipv6Mode::Static),
        ipv6_address: Some("2001:db8::9".into()),
        ipv6_prefix: Some(64),
        ipv6_gateway: Some("2001:db8::1".into()),
        ..IfaceChange::default()
    };
    let next = apply(&iface(0), &change).unwrap();
    assert_eq!(next.ipv6.ip6addr, ["2001:db8::9/64"]);
    let mut no_gateway = change.clone();
    no_gateway.ipv6_gateway = Some(String::new());
    assert!(apply(&iface(0), &no_gateway).is_err());
    let mut bad_prefix = change;
    bad_prefix.ipv6_prefix = Some(0);
    assert!(apply(&iface(0), &bad_prefix).is_err());
}

#[test]
fn the_mtu_has_a_range_per_kind_and_none_on_others() {
    let set = |kind, value| {
        apply(
            &iface(kind),
            &IfaceChange {
                mtu: Some(Mtu::Value(value)),
                ..IfaceChange::default()
            },
        )
    };
    assert_eq!(set(0, 9000).unwrap().mtu, 9000);
    assert!(set(0, 9001).is_err());
    assert!(set(11, 62000).is_ok());
    assert!(set(1, 1500).is_err());
    let auto = apply(
        &iface(0),
        &IfaceChange {
            mtu: Some(Mtu::Auto),
            ..IfaceChange::default()
        },
    )
    .unwrap();
    assert_eq!(auto.mtu_manual, 0);
}

#[test]
fn a_bond_mode_belongs_to_a_bond() {
    let change = IfaceChange {
        bond_mode: Some("802.3ad".into()),
        ..IfaceChange::default()
    };
    assert!(apply(&iface(0), &change).is_err());
    assert_eq!(apply(&iface(1), &change).unwrap().bond_mode, "802.3ad");
}

#[test]
fn the_ethernet_body_is_the_one_the_form_builds() {
    let b = body(&iface(0)).unwrap();
    assert_eq!(b["interface"], "eth0");
    assert_eq!(b["mtu"], 0);
    assert_eq!(b["mtu_manual"], 0);
    assert_eq!(b["vlan"], 0);
    assert_eq!(b["status"], 0);
    assert_eq!(b["ipv4"]["protocol"], "dhcp");
    assert_eq!(b["ipv4"]["dns"], json!(["192.0.2.1"]));
    // IPv6 that is not static sends the "/" that the empty form inputs make.
    assert_eq!(b["ipv6"]["ipaddr"], "/");
    assert!(b.get("slaves").is_none());
}

#[test]
fn a_manual_mtu_goes_in_the_body() {
    let mut i = iface(0);
    i.mtu_manual = 1;
    i.mtu = 9000;
    assert_eq!(body(&i).unwrap()["mtu"], 9000);
}

#[test]
fn a_bond_body_keeps_what_ugos_sent_in_ipv4() {
    let b = body(&iface(1)).unwrap();
    assert_eq!(b["bond_mode"], "balance-alb");
    assert_eq!(b["ipv4"]["future_field"], 7);
    assert!(b.get("mtu").is_none());
}

#[test]
fn a_wifi_body_is_small() {
    let b = body(&iface(2)).unwrap();
    let keys: Vec<&str> = b.as_object().unwrap().keys().map(String::as_str).collect();
    assert_eq!(keys.len(), 3, "{keys:?}");
    assert!(b.get("mtu").is_none() && b.get("vlan").is_none());
}

#[test]
fn a_bridge_is_sent_whole() {
    let b = body(&iface(5)).unwrap();
    assert_eq!(b["unknown_item_field"], "kept");
    assert_eq!(b["slaves"][0]["interface"], "eth1");
    assert_eq!(b["ipv6"]["ip6addr"], "2001:db8::5");
    assert_eq!(b["ipv6"]["prefixValue"], "64");
}

#[test]
fn a_thunderbolt_body_has_mtu_and_a_short_ipv6() {
    let mut i = iface(11);
    i.mtu_manual = 1;
    i.mtu = 62000;
    let b = body(&i).unwrap();
    assert_eq!(b["mtu"], 62000);
    assert_eq!(b["ipv6"].as_object().unwrap().len(), 2);
}

#[test]
fn a_kind_without_a_form_is_refused() {
    assert!(body(&iface(4)).is_err());
}

#[test]
fn an_empty_change_is_empty() {
    assert!(IfaceChange::default().is_empty());
    assert!(!static_v4().is_empty());
}
