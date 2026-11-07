//! Tests of the hardware, power, time and terminal operations.

use serde_json::json;

use super::basic::curve;
use super::clock::check_hostname;
use super::power::{PowerChange, Sleep, apply as power_apply, body as power_body, switch};
use super::terminal::{TerminalChange, apply as term_apply};
use super::ups::{Source, UpsSetting, Wait, body as ups_body};
use crate::types::hw::{Power, Terminal};

fn power() -> Power {
    serde_json::from_value(json!({
        "power_boot": false, "wake_on": false, "hard_drive_flag": true,
        "hard_drive_time": 20, "hard_drive_unit": "M", "hard_drive_strategy": 1,
        "usb_hard_drive_flag": true, "usb_hard_drive_time": 20, "usb_hard_drive_unit": "M",
        "usb_hard_drive_strategy": 1, "enable_hard_drive_sleep_log": false,
        "enable_scheduled_power": false, "time_switches": [], "storage_running": true,
        "hard_drive_scope": {"excluded_disk_devs": null, "excluded_pool_names": ["pool2"]},
        "usb_hard_drive_scope": {"excluded_disk_devs": null, "excluded_pool_names": null},
        "hard_drive_target_info": {"pools": []}
    }))
    .unwrap()
}

fn term(change: &TerminalChange) -> crate::error::Result<Terminal> {
    term_apply(&Terminal::default(), change)
}

#[test]
fn a_fan_curve_has_seventeen_points_and_runs_full_from_55_degrees() {
    let pairs = curve(&[30; 17]).unwrap();
    assert_eq!(pairs.len(), 17);
    assert_eq!(pairs[0], [0, 30]);
    assert_eq!(pairs[10], [50, 30]);
    assert_eq!(pairs[11], [55, 100]);
    assert_eq!(pairs[16], [80, 100]);
    assert!(curve(&[30; 16]).is_err());
    assert!(curve(&[19; 17]).is_err());
    assert!(curve(&[101; 17]).is_err());
}

#[test]
fn wake_on_lan_forces_power_on_after_a_loss() {
    let change = PowerChange {
        wake_on: Some(true),
        ..PowerChange::default()
    };
    let next = power_apply(&power(), &change).unwrap();
    assert!(next.wake_on && next.power_boot);
}

#[test]
fn sleep_times_are_the_ones_the_ui_offers() {
    let set = |s| {
        power_apply(
            &power(),
            &PowerChange {
                sleep: Some(s),
                ..PowerChange::default()
            },
        )
    };
    assert_eq!(set(Sleep::Hours(2)).unwrap().hard_drive_unit, "H");
    assert!(!set(Sleep::Never).unwrap().hard_drive_flag);
    assert!(set(Sleep::Minutes(15)).is_err());
    assert!(set(Sleep::Hours(4)).is_err());
    let bad = PowerChange {
        strategy: Some(3),
        ..PowerChange::default()
    };
    assert!(power_apply(&power(), &bad).is_err());
}

#[test]
fn the_power_body_keeps_the_excluded_pools_and_has_the_keys_the_ui_sends() {
    let body = power_body(&power());
    assert_eq!(
        body["hard_drive_scope"]["excluded_pool_names"],
        json!(["pool2"])
    );
    assert_eq!(body["hard_drive_scope"]["excluded_disk_devs"], json!([]));
    assert_eq!(
        body["usb_hard_drive_scope"]["excluded_pool_names"],
        json!([])
    );
    assert_eq!(body["time_switches"], json!([]));
    assert!(body.get("storage_running").is_none());
    assert!(body.get("hard_drive_target_info").is_none());
    assert_eq!(body.as_object().unwrap().len(), 15);
}

#[test]
fn a_schedule_entry_is_checked() {
    let entry = switch(&[5, 1, 1], "07:30", "open").unwrap();
    assert_eq!(entry.frequency, [1, 5]);
    for bad in [
        ("7:30", "open"),
        ("24:00", "open"),
        ("07:60", "close"),
        ("07:30", "toggle"),
    ] {
        assert!(switch(&[1], bad.0, bad.1).is_err(), "{bad:?}");
    }
    assert!(switch(&[], "07:30", "open").is_err());
    assert!(switch(&[8], "07:30", "open").is_err());
}

#[test]
fn a_ups_body_is_masked_by_its_kind() {
    let usb = ups_body(&UpsSetting {
        source: Source::Usb { nut_server: true },
        protect_type: 1,
        wait: Wait::Custom(5, 1),
    });
    assert_eq!(usb["type"], 0);
    assert_eq!(usb["nut_server_enable"], true);
    assert_eq!(usb["standby_time"], 5);
    assert_eq!(usb["snmp_ip"], "");
    let net = ups_body(&UpsSetting {
        source: Source::Network {
            master_ip: "192.0.2.4".into(),
        },
        protect_type: 1,
        wait: Wait::Immediately,
    });
    assert_eq!(net["type"], 2);
    assert_eq!(net["right_now"], true);
    assert_eq!(net["standby_time"], -1);
    assert_eq!(net["ups_master_ip"], "192.0.2.4");
    assert_eq!(net["nut_server_enable"], false);
}

#[test]
fn host_names_follow_the_ui_rules() {
    for good in ["nas1", "picard", "a-b", "NAS-9"] {
        assert!(check_hostname(good).is_ok(), "{good}");
    }
    for bad in [
        "",
        "1nas",
        "-nas",
        "nas-",
        "nas_1",
        "waytoolonganame123",
        "nä",
    ] {
        assert!(check_hostname(bad).is_err(), "{bad}");
    }
}

#[test]
fn terminal_ports_and_levels_are_checked() {
    let port = |p| TerminalChange {
        ssh_port: Some(p),
        ..TerminalChange::default()
    };
    assert_eq!(term(&port(2222)).unwrap().ssh.port, 2222);
    for bad in [0, 70000, 80, 443, 23] {
        assert!(term(&port(bad)).is_err(), "{bad}");
    }
    assert!(
        term(&TerminalChange {
            telnet_port: Some(22),
            ..TerminalChange::default()
        })
        .is_err()
    );
    assert!(
        term(&TerminalChange {
            ssh_encryption: Some(2),
            ..TerminalChange::default()
        })
        .is_err()
    );
    assert!(
        term(&TerminalChange {
            ssh_duration: Some(8),
            ..TerminalChange::default()
        })
        .is_err()
    );
    let lan = term(&TerminalChange {
        lan_only: Some(true),
        ..TerminalChange::default()
    })
    .unwrap();
    assert_eq!(
        (
            lan.ssh.permit_lan_access_only,
            lan.telnet.permit_lan_access_only
        ),
        (1, 1)
    );
}

#[test]
fn two_running_services_may_not_share_a_port() {
    let mut both = Terminal::default();
    both.ssh.enable = 1;
    both.ssh.port = 2222;
    both.telnet.enable = 1;
    both.telnet.port = 2222;
    let change = TerminalChange {
        lan_only: Some(true),
        ..TerminalChange::default()
    };
    assert!(term_apply(&both, &change).is_err());
}

#[test]
fn short_forms_are_read() {
    use super::parse::{days, numbers, sleep, unix_utc, wait};
    assert_eq!(sleep("20m").unwrap(), Sleep::Minutes(20));
    assert_eq!(sleep("2h").unwrap(), Sleep::Hours(2));
    assert_eq!(sleep("never").unwrap(), Sleep::Never);
    assert!(sleep("20").is_err() && sleep("m").is_err() && sleep("2d").is_err());
    assert_eq!(wait("now").unwrap(), Wait::Immediately);
    assert_eq!(wait("battery-low").unwrap(), Wait::BatteryLow);
    assert_eq!(wait("5m").unwrap(), Wait::Custom(5, 1));
    assert_eq!(wait("2h").unwrap(), Wait::Custom(2, 2));
    assert!(wait("soon").is_err());
    assert_eq!(days("weekdays").unwrap(), [1, 2, 3, 4, 5]);
    assert_eq!(days("weekend").unwrap(), [6, 7]);
    assert_eq!(days("1, 3").unwrap(), [1, 3]);
    assert!(days("mon").is_err());
    assert_eq!(numbers("1,2, 3").unwrap(), [1, 2, 3]);
    assert!(numbers("1,x").is_err());
    assert_eq!(unix_utc("1970-01-01 00:00:00").unwrap(), 0);
    assert_eq!(unix_utc("2027-01-31 00:00:00").unwrap(), 1_801_353_600);
    assert_eq!(unix_utc("2000-03-01 12:30:15").unwrap(), 951_913_815);
    for bad in [
        "2027-13-01 00:00:00",
        "2027-01-31",
        "2027-01-31 24:00:00",
        "x y",
    ] {
        assert!(unix_utc(bad).is_err(), "{bad}");
    }
}
