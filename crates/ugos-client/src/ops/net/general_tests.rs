//! Tests of the general settings, gateway and proxy changes.

use super::general::{DnsChange, GeneralChange, ProxyChange, apply, apply_proxy};
use crate::types::net::NetSettings;

fn settings() -> NetSettings {
    serde_json::from_value(serde_json::json!({
        "general": {"clash_check": 1, "dns": ["192.0.2.1"], "dns_mode": 0,
                    "ipv4_gateway": {"interface": "bridge0", "ipaddr": "192.0.2.1"},
                    "multiple_gateways": 0, "prior_ipv4": 1, "extra_flag": 3},
        "proxy": {"enable": 0, "httpx_proxy": 0, "local_proxy": 0,
                  "auth": {"enable": 0, "username": "", "password": ""}, "list": []},
        "version": 9
    }))
    .unwrap()
}

#[test]
fn manual_dns_takes_one_or_two_valid_servers() {
    let change = |servers: &[&str]| GeneralChange {
        dns: Some(DnsChange::Servers(
            servers.iter().map(|s| (*s).to_owned()).collect(),
        )),
        ..GeneralChange::default()
    };
    let next = apply(
        &settings(),
        &change(&["192.0.2.53", "2001:4860:4860::8888"]),
    )
    .unwrap();
    assert_eq!(next.general.dns_mode, 1);
    assert_eq!(next.general.dns, ["192.0.2.53", "2001:4860:4860::8888"]);
    assert!(apply(&settings(), &change(&[])).is_err());
    assert!(
        apply(
            &settings(),
            &change(&["192.0.2.1", "192.0.2.2", "192.0.2.3"])
        )
        .is_err()
    );
    assert!(apply(&settings(), &change(&["dns.example"])).is_err());
}

#[test]
fn automatic_dns_keeps_the_list_and_sets_the_mode() {
    let mut manual = settings();
    manual.general.dns_mode = 1;
    let change = GeneralChange {
        dns: Some(DnsChange::Auto),
        ..GeneralChange::default()
    };
    assert_eq!(apply(&manual, &change).unwrap().general.dns_mode, 0);
}

#[test]
fn switches_change_alone_and_the_rest_is_written_back() {
    let change = GeneralChange {
        multiple_gateways: Some(true),
        prior_ipv4: Some(false),
        ..GeneralChange::default()
    };
    let next = apply(&settings(), &change).unwrap();
    assert_eq!(next.general.multiple_gateways, 1);
    assert_eq!(next.general.prior_ipv4, 0);
    assert_eq!(next.general.clash_check, 1);
    // What UGOS sent and the client does not model goes back as it was.
    let body = serde_json::to_value(&next).unwrap();
    assert_eq!(body["general"]["extra_flag"], 3);
    assert_eq!(body["version"], 9);
    assert!(GeneralChange::default().is_empty());
}

#[test]
fn a_proxy_needs_ip_hosts_and_ports() {
    let all = |host: &str, port: i64| ProxyChange::All {
        host: host.into(),
        port,
        local: false,
        auth: None,
    };
    let next = apply_proxy(&settings(), &all("192.0.2.9", 3128)).unwrap();
    assert_eq!(next.proxy.enable, 1);
    assert_eq!(next.proxy.httpx_proxy, 0);
    assert_eq!(next.proxy.list[0].protocol, "all");
    assert!(apply_proxy(&settings(), &all("proxy.example", 3128)).is_err());
    assert!(apply_proxy(&settings(), &all("192.0.2.9", 70000)).is_err());
    assert_eq!(
        apply_proxy(&settings(), &ProxyChange::Off)
            .unwrap()
            .proxy
            .enable,
        0
    );
}

#[test]
fn a_split_proxy_needs_one_side_and_a_login_needs_both_parts() {
    let split = |http: Option<(&str, i64)>, auth: Option<(&str, &str)>| ProxyChange::Split {
        http: http.map(|(h, p)| (h.to_owned(), p)),
        https: None,
        local: true,
        auth: auth.map(|(u, p)| (u.to_owned(), p.to_owned())),
    };
    let next = apply_proxy(
        &settings(),
        &split(Some(("192.0.2.9", 8080)), Some(("u", "p"))),
    )
    .unwrap();
    assert_eq!(next.proxy.httpx_proxy, 1);
    assert_eq!(next.proxy.local_proxy, 1);
    assert_eq!(next.proxy.list.len(), 1);
    assert_eq!(next.proxy.auth.enable, 1);
    assert!(apply_proxy(&settings(), &split(None, None)).is_err());
    assert!(
        apply_proxy(
            &settings(),
            &split(Some(("192.0.2.9", 8080)), Some(("u", "")))
        )
        .is_err()
    );
}
