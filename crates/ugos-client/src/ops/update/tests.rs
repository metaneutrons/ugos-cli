use serde_json::json;

use super::backup::{RestoreService, check_backup, flatten_services, pick_services};
use super::firmware::{CHUNK, check_image};

fn tree() -> serde_json::Value {
    json!({
        "connect_service": {
            "domain_service": {"service_name": "a.domain"},
            "file_service": [{"service_name": "a.smb"}, {"service_name": "a.ftp"}]
        },
        "system_service": {"update_service": {"service_name": "a.update"}},
        "general_service": {"index_service": {"service_name": ""}}
    })
}

#[test]
fn the_restore_tree_is_flattened_and_empty_names_dropped() {
    let got = flatten_services(&tree());
    assert_eq!(got.len(), 3);
    let files = got
        .iter()
        .find(|s| s.path == "connect_service/file_service")
        .unwrap();
    assert_eq!(files.names, vec!["a.smb", "a.ftp"]);
    assert!(
        got.iter()
            .all(|s| s.path != "general_service/index_service")
    );
}

#[test]
fn services_are_picked_by_path_or_item_or_all() {
    let all = flatten_services(&tree());
    assert_eq!(pick_services(&all, &["all".into()]).unwrap().len(), 4);
    assert_eq!(
        pick_services(&all, &["domain_service".into()]).unwrap(),
        vec!["a.domain"]
    );
    assert_eq!(
        pick_services(&all, &["system_service/update_service".into()]).unwrap(),
        vec!["a.update"]
    );
    assert!(pick_services(&all, &["nope".into()]).is_err());
    assert!(pick_services(&all, &[]).is_err());
    let _: &[RestoreService] = &all;
}

#[test]
fn a_backup_file_is_checked_by_name_and_size() {
    assert!(check_backup("backup_restore.ugb", 1).is_ok());
    assert!(check_backup("config.ugb", 1).is_err());
    assert!(check_backup("backup_restore.zip", 1).is_err());
    assert!(check_backup("backup_restore.ugb", 0).is_err());
    assert!(check_backup("backup_restore.ugb", 100 * 1024 * 1024 + 1).is_err());
}

#[test]
fn a_firmware_image_is_checked_and_cut_in_chunks() {
    assert_eq!(check_image("fw.img", CHUNK as u64 + 1).unwrap(), 2);
    assert_eq!(check_image("fw.img", CHUNK as u64).unwrap(), 1);
    assert!(check_image("fw.bin", 1).is_err());
    assert!(check_image("fw.img", 0).is_err());
}
