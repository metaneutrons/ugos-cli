use serde_json::{Value, json};

use super::fields::{parse_bool, parse_value, patch, split_setting};
use super::{Config, PortUse, Service, ftp, nfs, port_check, smb, webdav};
use crate::error::UgosError;

fn cfg(value: &Value) -> Config {
    value.as_object().unwrap().clone()
}

fn set(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(a, v)| ((*a).into(), (*v).into()))
        .collect()
}

#[test]
fn booleans_accept_the_usual_spellings() {
    assert!(parse_bool("ON").unwrap());
    assert!(!parse_bool("no").unwrap());
    assert!(parse_bool("maybe").is_err());
}

#[test]
fn a_patch_maps_aliases_to_fields() {
    let got = patch(
        Service::Smb.fields(),
        &set(&[("encryption", "force"), ("workgroup", "HOME")]),
    )
    .unwrap();
    assert_eq!(got["encrypt_mode"], json!(2));
    assert_eq!(got["work_group"], json!("HOME"));
}

#[test]
fn a_patch_refuses_unknown_and_double_settings() {
    assert!(patch(Service::Smb.fields(), &set(&[("nope", "1")])).is_err());
    assert!(
        patch(
            Service::Ftp.fields(),
            &set(&[("port", "21"), ("port", "22")])
        )
        .is_err()
    );
}

#[test]
fn values_are_checked_like_the_web_ui() {
    let ftp = Service::Ftp.fields();
    assert!(patch(ftp, &set(&[("timeout", "0")])).is_err());
    assert!(patch(ftp, &set(&[("timeout", "7200")])).is_ok());
    assert!(patch(ftp, &set(&[("passive-min", "1023")])).is_err());
    assert!(patch(ftp, &set(&[("per-ip", "11")])).is_err());
    let smb = Service::Smb.fields();
    assert!(patch(smb, &set(&[("workgroup", "A,B")])).is_err());
    assert!(patch(smb, &set(&[("workgroup", "0123456789abcdef")])).is_err());
    assert!(patch(smb, &set(&[("wins", "10.0.0.1,fe80::1")])).is_err());
    assert!(patch(smb, &set(&[("wins", "10.0.0.1, 10.0.0.2")])).is_ok());
    assert!(patch(smb, &set(&[("veto", "//")])).is_err());
    assert!(patch(smb, &set(&[("veto", "/a/b/")])).is_ok());
    assert!(patch(smb, &set(&[("veto", "a")])).is_err());
}

#[test]
fn protocols_become_a_sorted_list() {
    let field = Service::Smb
        .fields()
        .iter()
        .find(|f| f.alias == "protocols")
        .unwrap();
    assert_eq!(parse_value(field, "smb2,smb1").unwrap(), json!([1, 2]));
    assert_eq!(parse_value(field, "2").unwrap(), json!([2]));
    assert!(parse_value(field, "smb9").is_err());
}

#[test]
fn a_setting_splits_at_the_first_equals_sign() {
    assert_eq!(
        split_setting("veto=/a=b/").unwrap(),
        ("veto".into(), "/a=b/".into())
    );
    assert!(split_setting("veto").is_err());
}

#[test]
fn reserved_ports_differ_for_ftp() {
    let ftp = PortUse {
        module: "ftp",
        key: "port",
        name: None,
        port: 21,
    };
    assert!(port_check(&ftp, true).is_ok());
    assert!(
        port_check(
            &PortUse {
                module: "rsync",
                ..ftp
            },
            false
        )
        .is_err()
    );
    assert!(port_check(&PortUse { port: 80, ..ftp }, true).is_err());
    assert!(port_check(&PortUse { port: 70000, ..ftp }, true).is_err());
}

fn smb_cfg() -> Config {
    cfg(
        &json!({"status": true, "work_group": "HOME", "smb_protocols": [2],
        "encrypt_mode": 0, "enable_op_locks": true, "enable_follow_symlinks": true}),
    )
}

#[test]
fn smb_needs_force_for_smb1_and_ntlmv1() {
    let current = smb_cfg();
    let mut next = current.clone();
    let _ = next.insert("smb_protocols".into(), json!([1, 2]));
    assert!(matches!(
        smb::finish(&current, &mut next, false, false),
        Err(UgosError::NeedsForce(_))
    ));
    assert!(smb::finish(&current, &mut next, false, true).is_ok());
    let mut next = current.clone();
    let _ = next.insert("enable_ntlmv1".into(), json!(true));
    assert!(matches!(
        smb::finish(&current, &mut next, false, false),
        Err(UgosError::NeedsForce(_))
    ));
}

#[test]
fn smb_refuses_the_combinations_the_web_ui_refuses() {
    for (key, value) in [
        ("enable_smb3_multi_channel", json!(true)),
        ("enable_smb2_leases", json!(true)),
        ("enable_veto_files", json!(true)),
    ] {
        let current = smb_cfg();
        let mut next = current.clone();
        let _ = next.insert("encrypt_mode".into(), json!(2));
        let _ = next.insert("enable_op_locks".into(), json!(key != "enable_smb2_leases"));
        let _ = next.insert(key.into(), value);
        assert!(
            smb::finish(&current, &mut next, false, true).is_err(),
            "{key}"
        );
    }
}

#[test]
fn ftp_starts_secure_and_asks_for_plain() {
    let current = cfg(&json!({"port": 21}));
    let mut next = current.clone();
    let ports = ftp::finish(&current, &mut next, true, false).unwrap();
    assert_eq!(next["enable_ftps"], json!(true));
    assert_eq!(ports[0].port, 21);
    let mut plain = current.clone();
    let _ = plain.insert("enable_ftp".into(), json!(true));
    assert!(matches!(
        ftp::finish(&current, &mut plain, true, false),
        Err(UgosError::NeedsForce(_))
    ));
}

#[test]
fn ftp_swaps_a_reversed_passive_range_and_resets_an_unused_one() {
    let current = cfg(&json!({"port": 21, "pasv_min_port": 0, "pasv_max_port": 0}));
    let mut next = current.clone();
    for (k, v) in [
        ("enable_ftps", json!(true)),
        ("enable_pasv_port", json!(true)),
        ("pasv_min_port", json!(3000)),
        ("pasv_max_port", json!(2000)),
    ] {
        let _ = next.insert(k.into(), v);
    }
    let _ = ftp::finish(&current, &mut next, true, false).unwrap();
    assert_eq!(next["pasv_min_port"], json!(2000));
    assert_eq!(next["pasv_max_port"], json!(3000));
    let mut off = next.clone();
    let _ = off.insert("enable_pasv_port".into(), json!(false));
    let _ = ftp::finish(&current, &mut off, true, false).unwrap();
    assert_eq!(off["pasv_min_port"], json!(0));
}

#[test]
fn nfs_adds_its_constants_and_checks_custom_ports() {
    let current = cfg(&json!({"enableNfsServer": true}));
    let mut next = current.clone();
    let _ = next.insert("customerPort".into(), json!(true));
    let _ = next.insert("statdPort".into(), json!(2050));
    let _ = next.insert("nlockMgrPort".into(), json!(2050));
    assert!(nfs::finish(&current, &mut next, false, false).is_err());
    let _ = next.insert("nlockMgrPort".into(), json!(2051));
    let ports = nfs::finish(&current, &mut next, false, false).unwrap();
    assert_eq!(ports.len(), 2);
    assert_eq!(next["logs"], json!(true));
    assert_eq!(next["enableAdvance"], json!(true));
}

#[test]
fn nfs_needs_force_to_start() {
    let current = cfg(&json!({"enableNfsServer": false}));
    let mut next = current.clone();
    assert!(matches!(
        nfs::finish(&current, &mut next, true, false),
        Err(UgosError::NeedsForce(_))
    ));
}

#[test]
fn webdav_needs_a_protocol_and_distinct_ports() {
    let current = cfg(&json!({}));
    let mut none = current.clone();
    assert!(webdav::finish(&current, &mut none).is_err());
    let mut both = cfg(&json!({"enable_http": true, "enable_https": true}));
    let ports = webdav::finish(&current, &mut both).unwrap();
    assert_eq!(both["http_port"], json!(5005));
    assert_eq!(both["https_port"], json!(5006));
    assert_eq!(ports.len(), 2);
    let mut same = cfg(
        &json!({"enable_http": true, "enable_https": true, "http_port": 7000, "https_port": 7000}),
    );
    assert!(webdav::finish(&current, &mut same).is_err());
}
