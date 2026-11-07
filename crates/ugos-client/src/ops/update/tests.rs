use serde_json::json;

use super::backup::{RestoreService, check_backup_file, flatten_services, pick_services};
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
    let dir = std::env::temp_dir().join(format!("ugos-update-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let good = dir.join("backup_restore.ugb");
    std::fs::write(&good, b"x").unwrap();
    assert_eq!(check_backup_file(&good).unwrap(), 1);
    let wrong_name = dir.join("config.ugb");
    std::fs::write(&wrong_name, b"x").unwrap();
    assert!(check_backup_file(&wrong_name).is_err());
    let empty = dir.join("backup_restore2.ugb");
    std::fs::write(&empty, b"").unwrap();
    assert!(check_backup_file(&empty).is_err());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_firmware_image_is_checked_and_cut_in_chunks() {
    let dir = std::env::temp_dir().join(format!("ugos-image-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let image = dir.join("fw.img");
    std::fs::write(&image, vec![0u8; CHUNK + 1]).unwrap();
    assert_eq!(check_image(&image).unwrap(), (CHUNK as u64 + 1, 2));
    let not_img = dir.join("fw.bin");
    std::fs::write(&not_img, b"x").unwrap();
    assert!(check_image(&not_img).is_err());
    std::fs::remove_dir_all(&dir).unwrap();
}
