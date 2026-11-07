//! Tests of the traffic control rules.

use super::flow::{NewRule, build, check_ports, flatten, group};
use crate::types::net::{Flow, FlowService};

fn custom() -> NewRule {
    NewRule {
        interface: "eth0".into(),
        kind: "custom".into(),
        ports: "80,443".into(),
        target: "dest".into(),
        protocol: "tcp".into(),
        min_rate: 100,
        max_rate: 5000,
        sections: Vec::new(),
    }
}

fn services() -> Vec<FlowService> {
    vec![
        FlowService {
            section: "com.ugreen.pro.ugos.web".into(),
            name: "UGOS web".into(),
            ports: "9443".into(),
            protocol: "tcp".into(),
        },
        FlowService {
            section: "com.example.media".into(),
            name: "Media".into(),
            ports: "8096".into(),
            protocol: "tcp".into(),
        },
    ]
}

#[test]
fn ports_are_a_short_list_or_a_range() {
    for good in [
        "80",
        "80,443",
        "1-65535",
        "1,2,3,4,5,6,7,8,9,10,11,12,13,14,15",
    ] {
        assert!(check_ports(good).is_ok(), "{good}");
    }
    for bad in [
        "",
        "0",
        "65536",
        "80,",
        "a",
        "443-80",
        "80-80",
        "1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16",
    ] {
        assert!(check_ports(bad).is_err(), "{bad:?}");
    }
}

#[test]
fn a_custom_rule_carries_ports_target_and_protocol() {
    let rule = build(&custom(), "LAN1", &[]).unwrap();
    assert_eq!((rule.kind.as_str(), rule.enable), ("custom", 1));
    assert_eq!(rule.ports.as_deref(), Some("80,443"));
    assert_eq!(rule.target.as_deref(), Some("dest"));
    assert!(rule.internal_servers.is_none());
    let mut bad = custom();
    bad.protocol = "icmp".into();
    assert!(build(&bad, "LAN1", &[]).is_err());
    bad = custom();
    bad.target = "both".into();
    assert!(build(&bad, "LAN1", &[]).is_err());
}

#[test]
fn an_all_ports_rule_sends_all_and_src() {
    let mut rule = custom();
    rule.kind = "all".into();
    let built = build(&rule, "ALL", &[]).unwrap();
    assert_eq!(built.ports.as_deref(), Some("all"));
    assert_eq!(built.target.as_deref(), Some("src"));
    assert!(built.protocol.is_none());
}

#[test]
fn a_service_rule_names_services_and_sections_and_no_ports() {
    let mut rule = custom();
    rule.kind = "sys".into();
    rule.sections = vec!["com.example.media".into()];
    let built = build(&rule, "LAN1", &services()).unwrap();
    assert_eq!(
        built.internal_servers.as_deref(),
        Some(&["Media".to_owned()][..])
    );
    assert_eq!(
        built.sections.as_deref(),
        Some(&["com.example.media".to_owned()][..])
    );
    assert!(built.ports.is_none() && built.target.is_none() && built.protocol.is_none());
    let json = serde_json::to_value(&built).unwrap();
    assert!(json.get("ports").is_none());
    assert_eq!(json["internal_Servers"][0], "Media");
    rule.sections = vec!["nope".into()];
    assert!(build(&rule, "LAN1", &services()).is_err());
}

#[test]
fn rates_have_rules() {
    let mut rule = custom();
    rule.min_rate = 6000;
    assert!(build(&rule, "LAN1", &[]).is_err());
    rule = custom();
    rule.max_rate = 1_000_000;
    assert!(build(&rule, "LAN1", &[]).is_err());
    rule = custom();
    rule.kind = "sys".into();
    rule.sections = vec!["com.ugreen.pro.ugos.web".into()];
    rule.min_rate = 100;
    assert!(build(&rule, "LAN1", &services()).is_err());
    rule.min_rate = 1024;
    rule.max_rate = 2048;
    assert!(build(&rule, "LAN1", &services()).is_ok());
}

#[test]
fn rules_are_grouped_by_interface_in_the_order_first_seen() {
    let a = build(&custom(), "LAN1", &[]).unwrap();
    let mut other = custom();
    other.interface = "eth1".into();
    let b = build(&other, "LAN2", &[]).unwrap();
    let flow = group(vec![a.clone(), b, a]);
    assert_eq!(flow.list.len(), 2);
    assert_eq!(flow.list[0].interface, "eth0");
    assert_eq!(flow.list[0].label, "LAN1");
    assert_eq!(flow.list[0].list.len(), 2);
    assert_eq!(flatten(&flow).len(), 3);
    assert_eq!(group(Vec::new()), Flow::default());
}

#[test]
fn a_rule_takes_its_interface_from_the_group_and_loses_what_is_not_its_kind() {
    let flow: Flow = serde_json::from_value(serde_json::json!({
        "list": [{"interface": "all", "label": "ALL", "list": [
            {"type": "custom", "enable": 1, "ports": "65001", "target": "dest", "protocol": "tcp",
             "min_rate": 0, "max_rate": 999_999, "internal_Servers": [], "sections": []},
            {"type": "sys", "enable": 1, "min_rate": 1024, "max_rate": 2048,
             "internal_Servers": ["SMB"], "sections": ["com.ugreen.pro.file.samba"], "ports": "445"}
        ]}], "ver": 0
    }))
    .unwrap();
    let rules = flatten(&flow);
    assert_eq!(rules[0].interface, "all");
    assert_eq!(rules[0].label, "ALL");
    assert!(rules[0].internal_servers.is_none() && rules[0].sections.is_none());
    assert_eq!(rules[1].interface, "all");
    assert!(rules[1].ports.is_none());
    assert_eq!(
        rules[1].sections.as_deref(),
        Some(&["com.ugreen.pro.file.samba".to_owned()][..])
    );
    // Written back, it is the same single group.
    let again = group(rules);
    assert_eq!(again.list.len(), 1);
    assert_eq!(again.list[0].interface, "all");
}
