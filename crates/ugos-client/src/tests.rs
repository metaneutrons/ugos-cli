//! Unit tests for the UGOS client library.
#![allow(clippy::unwrap_used, clippy::panic)]

use crate::client::UgosClient;
use crate::error::UgosError;
use crate::types::common::{ApiResponse, ResultWrapper};
use crate::types::docker::{
    Container, ContainerDetail, ContainerPage, DockerImage, DockerOverview, Mirror,
};
use crate::types::kvm::{
    HostInfo, ImageInfo, NetworkDetail, NetworkSummary, Snapshot, StorageInfo, UsbDevice, VmDetail,
    VmSummary, VncLink,
};

// ── JSON fixture deserialization ────────────────────────────────────

#[test]
fn deserialize_vm_summary() {
    let json = r#"{
        "virName": "797fb54f-1234-5678-9abc-def012345678",
        "virID": 4,
        "virDisplayName": "CachyOS",
        "storageName": "Volume 1",
        "systemType": "linux",
        "systemVersion": "",
        "guestCpuPercent": 2,
        "guestTotalMemory": 25165824,
        "guestUsedMemory": 18958724,
        "hostCpuPercent": 2,
        "hostUsedMemory": 19497880,
        "hostTotalMemory": 65581740,
        "upload": 0,
        "download": 590,
        "status": "running",
        "processStatus": "createSuccess",
        "progress": 0,
        "createTime": 1775925041
    }"#;
    let vm: VmSummary = serde_json::from_str(json).unwrap();
    assert_eq!(vm.vir_display_name, "CachyOS");
    assert_eq!(vm.status, "running");
    assert_eq!(vm.guest_cpu_percent, 2);
    assert_eq!(vm.guest_total_memory, 25_165_824);
    assert_eq!(vm.vir_id, 4);
}

#[test]
fn deserialize_vm_detail() {
    let json = r#"{
        "virtualMachineName": "797fb54f-1234",
        "virtualMachineDisplayName": "CachyOS",
        "systemType": "linux",
        "systemVersion": "",
        "core": {"value": 12},
        "memory": {"value": 25165824},
        "images": [{"path": "/volume1/iso/CachyOS.iso", "dev": "hda", "order": 2}],
        "dists": [{"bus": "virtio", "size": 1048576000, "dev": "vda", "path": "/vol/disk.qcow2", "order": 1}],
        "networks": [{"name": "vnet-bridge0", "macAddress": "52:54:00:d7:38:b5", "type": "virtio"}],
        "device": {"usbController": 2, "usbDevices": [], "graphicsCard": "virtio", "bootType": "uefi"},
        "otherConfig": {"autoMaticStartUp": true, "keyboardLanguage": "de", "shareDirectory": []},
        "storageName": "volume1",
        "ovaPath": ""
    }"#;
    let d: VmDetail = serde_json::from_str(json).unwrap();
    assert_eq!(d.virtual_machine_display_name, "CachyOS");
    assert_eq!(d.core.value, 12);
    assert_eq!(d.memory.value, 25_165_824);
    assert_eq!(d.device.boot_type, "uefi");
    assert!(d.other_config.auto_matic_start_up);
    assert_eq!(d.networks.len(), 1);
    assert_eq!(d.dists.len(), 1);
    assert_eq!(d.images.len(), 1);
}

#[test]
fn deserialize_network_summary() {
    let json = r#"{
        "networkName": "vnet-bridge0",
        "networkUUID": "8171cf35-abcd",
        "networkLabel": "VBR-LAN1",
        "networkValid": true,
        "networkType": "bridge",
        "networkMode": "bridge",
        "interfaceName": "bridge0",
        "virtualDisplayNames": ["CachyOS", "Worf"],
        "createTime": 0,
        "systemNetwork": true
    }"#;
    let n: NetworkSummary = serde_json::from_str(json).unwrap();
    assert_eq!(n.network_name, "vnet-bridge0");
    assert_eq!(n.virtual_display_names, vec!["CachyOS", "Worf"]);
    assert!(n.system_network);
}

#[test]
fn deserialize_network_detail() {
    let json = r#"{
        "networkUUID": "abc",
        "networkName": "vnet-bridge0",
        "networkType": "bridge",
        "networkMode": "bridge",
        "mappingNetwork": "bridge0",
        "enableIpv4": false,
        "allocateIpv4": false,
        "ipv4Subnet": "",
        "ipv4Gateway": "",
        "ipv4DHCPStartIp": "",
        "ipv4DHCPEndIp": "",
        "enableIpv6": false,
        "ipv6Subnet": "",
        "ipv6Gateway": "",
        "ipv6DHCPStartIp": "",
        "ipv6DHCPEndIp": "",
        "allocateIpv6": false
    }"#;
    let d: NetworkDetail = serde_json::from_str(json).unwrap();
    assert_eq!(d.network_name, "vnet-bridge0");
    assert_eq!(d.mapping_network, "bridge0");
    assert!(!d.enable_ipv4);
}

#[test]
fn deserialize_storage_info() {
    let json = r#"{
        "name": "volume1",
        "label": "Volume 1",
        "health": 0,
        "status": 0,
        "totalCapacity": 23955125567488,
        "availableCapacity": 21626153811968,
        "uuid": "Csuzof-yZcf",
        "path": "/volume1",
        "filesystem": "btrfs",
        "describe": ""
    }"#;
    let s: StorageInfo = serde_json::from_str(json).unwrap();
    assert_eq!(s.name, "volume1");
    assert_eq!(s.filesystem, "btrfs");
    assert_eq!(s.total_capacity, 23_955_125_567_488);
}

#[test]
fn deserialize_image_info() {
    let json = r#"{
        "id": 1,
        "fileName": "EndavourOS.iso",
        "imageName": "EndavourOS",
        "fileSize": 3498065920,
        "progress": 0,
        "state": "completed",
        "imageType": "iso",
        "path": "/volume1/@appstore/com.ugreen.kvm/iso/EndavourOS.iso",
        "virtualDiskSize": 0
    }"#;
    let i: ImageInfo = serde_json::from_str(json).unwrap();
    assert_eq!(i.image_name, "EndavourOS");
    assert_eq!(i.file_size, 3_498_065_920);
    assert_eq!(i.state, "completed");
}

#[test]
fn deserialize_snapshot() {
    // Shape captured from a live NAS: createTime is a string, and there is no
    // isCurrent field.
    let json = r#"{
        "createTime": "2026-08-18 08:40:50",
        "description": "before the update",
        "displayName": "2026-08-18 08:40:50",
        "id": 1,
        "name": "d8becfe2-9044-41d0-bc63-32bc65aa59d7_1787035250",
        "screenshot": "",
        "virName": "d8becfe2-9044-41d0-bc63-32bc65aa59d7"
    }"#;
    let s: Snapshot = serde_json::from_str(json).unwrap();
    assert_eq!(s.display_name, "2026-08-18 08:40:50");
    assert_eq!(s.create_time, "2026-08-18 08:40:50");
    assert_eq!(s.description, "before the update");
    assert_eq!(s.id, 1);
    assert_eq!(s.vir_name, "d8becfe2-9044-41d0-bc63-32bc65aa59d7");
}

#[test]
fn deserialize_snapshot_tolerates_missing_fields() {
    let s: Snapshot = serde_json::from_str(r#"{"name": "snap-1234"}"#).unwrap();
    assert_eq!(s.name, "snap-1234");
    assert_eq!(s.create_time, "");
}

#[test]
fn deserialize_usb_device() {
    let json = r#"{
        "vendorID": "0x8087",
        "vendorName": "Intel Corp.",
        "productID": "0x0033",
        "productName": "AX211 Bluetooth",
        "busID": 3,
        "deviceID": 2,
        "usedBy": ""
    }"#;
    let u: UsbDevice = serde_json::from_str(json).unwrap();
    assert_eq!(u.vendor_name, "Intel Corp.");
    assert_eq!(u.product_name, "AX211 Bluetooth");
    assert_eq!(u.used_by, "");
}

#[test]
fn deserialize_host_info() {
    let json = r#"{"cores": 12, "memory": 67155701760}"#;
    let h: HostInfo = serde_json::from_str(json).unwrap();
    assert_eq!(h.cores, 12);
    assert_eq!(h.memory, 67_155_701_760);
}

#[test]
fn deserialize_docker_overview() {
    let json = r#"{
        "containerCount": 3,
        "runContainerCount": 1,
        "imageCount": 5,
        "memoryUsed": 22000000000,
        "totalMemory": 67000000000,
        "containerMemory": 500000000,
        "cpuUsed": 5,
        "containerCpuUsed": 2,
        "status": true
    }"#;
    let o: DockerOverview = serde_json::from_str(json).unwrap();
    assert_eq!(o.container_count, 3);
    assert!(o.status);
}

#[test]
fn deserialize_container() {
    let json = r#"{
        "name": "hello-world-1",
        "imageId": "sha256:e2ac70e7319a",
        "containerId": "a4325c2117997f37",
        "imageName": "hello-world",
        "version": "latest",
        "projectName": "",
        "createAt": 1776033624,
        "status": "exited",
        "application": ""
    }"#;
    let c: Container = serde_json::from_str(json).unwrap();
    assert_eq!(c.name, "hello-world-1");
    assert_eq!(c.status, "exited");
    assert_eq!(c.image_name, "hello-world");
}

#[test]
fn deserialize_container_detail() {
    let json = r#"{
        "imageName": "hello-world",
        "imageId": "sha256:e2ac70e7319a",
        "imageVersion": "latest",
        "tag": "hello-world:latest",
        "containerName": "hello-world-1",
        "cpuLimit": 0,
        "memLimit": 0,
        "noRestrictions": true,
        "networkMode": "bridge",
        "hardwareAcceleration": false,
        "privilegedMode": false,
        "abnormalReset": false,
        "runContainer": false,
        "portMapping": [],
        "volumes": null,
        "environmentVariables": [{"variable": "PATH", "price": "/usr/bin"}],
        "containerRunCommand": ["/hello"],
        "permAndFunc": ["CAP_CHOWN"],
        "projectName": ""
    }"#;
    let d: ContainerDetail = serde_json::from_str(json).unwrap();
    assert_eq!(d.container_name, "hello-world-1");
    assert!(d.no_restrictions);
    assert_eq!(d.environment_variables.len(), 1);
    assert_eq!(d.environment_variables[0].variable, "PATH");
    assert_eq!(d.environment_variables[0].price, "/usr/bin");
    assert!(d.volumes.is_none());
}

#[test]
fn deserialize_container_detail_null_lists() {
    let json = r#"{
        "containerName": "one-shot",
        "gpuIds": null,
        "subnetSettings": null,
        "portMapping": null,
        "environmentVariables": null,
        "containerRunCommand": null
    }"#;
    let detail: ContainerDetail = serde_json::from_str(json).unwrap();
    assert_eq!(detail.gpu_ids, Vec::<String>::new());
    assert!(detail.subnet_settings.is_empty());
    assert!(detail.port_mapping.is_empty());
    assert!(detail.environment_variables.is_empty());
    assert_eq!(detail.container_run_command, Vec::<String>::new());
}

#[test]
fn deserialize_container_page_null_result() {
    let json = r#"{"originalTotal": 0, "result": null, "total": 0}"#;
    let p: ContainerPage = serde_json::from_str(json).unwrap();
    assert!(p.result.is_none());
    assert_eq!(p.total, 0);
}

#[test]
fn deserialize_docker_image() {
    let json = r#"{
        "imageId": "sha256:e2ac70e7319a",
        "imageRef": "hello-world:latest",
        "imageName": "hello-world",
        "imageSize": 10072,
        "imageVersion": "latest",
        "status": 1,
        "create": 1774301639
    }"#;
    let i: DockerImage = serde_json::from_str(json).unwrap();
    assert_eq!(i.image_name, "hello-world");
    assert_eq!(i.image_size, 10072);
    assert_eq!(i.status, 1);
}

#[test]
fn deserialize_mirror() {
    let json = r#"{
        "id": 0,
        "alias": "DockerHub",
        "address": "https://hub.docker.com/",
        "isDockerhub": true,
        "status": true
    }"#;
    let m: Mirror = serde_json::from_str(json).unwrap();
    assert_eq!(m.alias, "DockerHub");
    assert!(m.status);
    assert!(m.is_dockerhub);
}

#[test]
fn deserialize_vnc_link() {
    let json = r#"{"link": "https://vnc.example.com/abc", "type": 0, "password": "secret", "apiKey": "key123"}"#;
    let v: VncLink = serde_json::from_str(json).unwrap();
    assert_eq!(v.link, "https://vnc.example.com/abc");
    assert_eq!(v.link_type, 0);
}

// ── ApiResponse error code mapping ──────────────────────────────────

#[test]
fn api_response_success() {
    let resp = ApiResponse {
        code: 200,
        msg: "success".into(),
        data: "hello",
    };
    assert_eq!(resp.into_result().unwrap(), "hello");
}

#[test]
fn api_response_auth_failed() {
    let resp = ApiResponse {
        code: 1003,
        msg: "bad password".into(),
        data: (),
    };
    assert!(matches!(resp.into_result(), Err(UgosError::AuthFailed)));
}

#[test]
fn api_response_parameter_error() {
    let resp = ApiResponse {
        code: 1005,
        msg: "bad param".into(),
        data: (),
    };
    assert!(matches!(
        resp.into_result(),
        Err(UgosError::ParameterError(_))
    ));
}

#[test]
fn api_response_login_expired() {
    let resp = ApiResponse {
        code: 1024,
        msg: "expired".into(),
        data: (),
    };
    assert!(matches!(resp.into_result(), Err(UgosError::LoginExpired)));
}

#[test]
fn api_response_operation_failed_3004() {
    let resp = ApiResponse {
        code: 3004,
        msg: "vm op failed".into(),
        data: (),
    };
    assert!(matches!(
        resp.into_result(),
        Err(UgosError::OperationFailed(_))
    ));
}

#[test]
fn api_response_docker_2031() {
    let resp = ApiResponse {
        code: 2031,
        msg: "create failed".into(),
        data: (),
    };
    assert!(matches!(
        resp.into_result(),
        Err(UgosError::OperationFailed(_))
    ));
}

#[test]
fn api_response_docker_2052() {
    let resp = ApiResponse {
        code: 2052,
        msg: "special chars".into(),
        data: (),
    };
    assert!(matches!(
        resp.into_result(),
        Err(UgosError::ParameterError(_))
    ));
}

#[test]
fn api_response_docker_2063() {
    let resp = ApiResponse {
        code: 2063,
        msg: "downloading".into(),
        data: (),
    };
    assert!(matches!(
        resp.into_result(),
        Err(UgosError::OperationFailed(_))
    ));
}

#[test]
fn api_response_app_not_found() {
    let resp = ApiResponse {
        code: 9404,
        msg: "not installed".into(),
        data: (),
    };
    assert!(matches!(resp.into_result(), Err(UgosError::AppNotFound(_))));
}

#[test]
fn api_response_unknown_code() {
    let resp = ApiResponse {
        code: 9999,
        msg: "unknown".into(),
        data: (),
    };
    match resp.into_result() {
        Err(UgosError::Api { code, msg }) => {
            assert_eq!(code, 9999);
            assert_eq!(msg, "unknown");
        }
        other => panic!("expected Api error, got {other:?}"),
    }
}

// ── ResultWrapper ───────────────────────────────────────────────────

#[test]
fn result_wrapper_vec() {
    let json = r#"{"result": [1, 2, 3]}"#;
    let w: ResultWrapper<Vec<i32>> = serde_json::from_str(json).unwrap();
    assert_eq!(w.result, vec![1, 2, 3]);
}

#[test]
fn result_wrapper_string() {
    let json = r#"{"result": "successful"}"#;
    let w: ResultWrapper<String> = serde_json::from_str(json).unwrap();
    assert_eq!(w.result, "successful");
}

// ── Token URL appending ─────────────────────────────────────────────

#[test]
fn append_token_percent_encodes_the_token() {
    // An `&` would split the query, a `#` would cut off everything after it,
    // a `+` would arrive as a space.
    let url = UgosClient::append_token("https://nas/ugreen/v1/kvm/vm/list", "a&b#c+d/e");
    assert!(
        url.ends_with("?token=a%26b%23c%2Bd%2Fe"),
        "token not encoded: {url}"
    );
}

#[test]
fn append_token_leaves_ordinary_tokens_readable() {
    let url = UgosClient::append_token("https://nas/x", "ABC123");
    assert!(url.ends_with("?token=ABC123"), "{url}");
}

#[test]
fn append_token_no_query() {
    let url = "https://nas:9443/ugreen/v1/kvm/manager/ShowLocalVirtualList";
    let result = UgosClient::append_token(url, "ABC123");
    assert_eq!(
        result,
        "https://nas:9443/ugreen/v1/kvm/manager/ShowLocalVirtualList?token=ABC123"
    );
}

#[test]
fn append_token_existing_query() {
    let url = "https://nas:9443/ugreen/v1/kvm/manager/PowerOn?name=abc";
    let result = UgosClient::append_token(url, "ABC123");
    assert_eq!(
        result,
        "https://nas:9443/ugreen/v1/kvm/manager/PowerOn?name=abc&token=ABC123"
    );
}

// ── Full API envelope deserialization ───────────────────────────────

#[test]
fn full_envelope_vm_list() {
    let json = r#"{
        "code": 200,
        "msg": "success",
        "data": {
            "result": [{
                "virName": "abc-123",
                "virID": 1,
                "virDisplayName": "TestVM",
                "storageName": "volume1",
                "systemType": "linux",
                "systemVersion": "",
                "guestCpuPercent": 0,
                "guestTotalMemory": 0,
                "guestUsedMemory": 0,
                "hostCpuPercent": 0,
                "hostUsedMemory": 0,
                "hostTotalMemory": 0,
                "upload": 0,
                "download": 0,
                "status": "shutoff",
                "processStatus": "createSuccess",
                "progress": 0,
                "createTime": 0
            }]
        },
        "time": 0.001
    }"#;
    let resp: ApiResponse<ResultWrapper<Vec<VmSummary>>> = serde_json::from_str(json).unwrap();
    let vms = resp.into_result().unwrap().result;
    assert_eq!(vms.len(), 1);
    assert_eq!(vms[0].vir_display_name, "TestVM");
}

#[test]
fn full_envelope_error() {
    let json = r#"{
        "code": 1024,
        "msg": "Login has expired, please login again!",
        "debug": "Err - code: 1024",
        "data": {},
        "time": 0.0006
    }"#;
    let resp: ApiResponse<serde_json::Value> = serde_json::from_str(json).unwrap();
    assert!(matches!(resp.into_result(), Err(UgosError::LoginExpired)));
}

#[test]
fn a_running_download_reports_its_progress_as_a_fraction() {
    // A running task answered `"plan": 5.24`, which made `download list` fail with
    // "invalid type: floating point `5.24`, expected i64".
    let running = serde_json::json!({
        "id": 1, "task_id": "t1", "download_file_name": "a.iso",
        "total_size": 6_000_000_000_i64, "downloaded_size": 314_400_000_i64,
        "download_speed": 48_234_496_i64, "plan": 5.24, "remaining_time": 118
    });
    let task: crate::types::download::DownloadTask = serde_json::from_value(running).unwrap();
    assert!((task.plan - 5.24).abs() < f64::EPSILON);
    assert_eq!(task.download_speed, 48_234_496);

    // Finished tasks and older answers carry a whole number, or none.
    let done: crate::types::download::DownloadTask =
        serde_json::from_value(serde_json::json!({ "plan": 100 })).unwrap();
    assert!((done.plan - 100.0).abs() < f64::EPSILON);
    let bare: crate::types::download::DownloadTask =
        serde_json::from_value(serde_json::json!({})).unwrap();
    assert!(bare.plan.abs() < f64::EPSILON);
}

// ── Shared folders and SMB ──────────────────────────────────────────

mod shares {
    use crate::api::shares::{acl_body, pick_share, update_body};
    use crate::error::UgosError;
    use crate::types::common::ResultWrapper;
    use crate::types::shares::{
        Access, AclChange, CreatedShare, NewShare, PathAcl, PrincipalKind, Share, ShareInfo,
        SmbAdvanced, SmbConfig,
    };

    /// `filemgr/getShareList`, trimmed to two entries.
    const SHARE_LIST: &str = r#"{"result": [
        {"atime": 1794014795, "ctime": 1766182462, "device": "", "ext": "",
         "file_type": 1, "filesystem": "btrfs", "icon": 0, "id": 7, "index": 0,
         "is_migrated": false, "is_mount": true, "mtime": 1792357980,
         "name": "media", "owner": "root", "owner_type": "user",
         "path": "/volume1/media", "permission_mask": 7, "size": 0,
         "tag_ids": null, "time_backup": false, "type": 1, "uid": 0},
        {"ctime": 1793105644, "filesystem": "btrfs", "id": 8, "mtime": 1793106057,
         "name": "7", "owner": "root", "path": "/volume1/7", "time_backup": true}
    ]}"#;

    fn shares() -> Vec<Share> {
        let list: ResultWrapper<Vec<Share>> = serde_json::from_str(SHARE_LIST).unwrap();
        list.result
    }

    #[test]
    fn a_share_list_entry_keeps_what_a_share_needs() {
        let all = shares();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].id, 7);
        assert_eq!(all[0].name, "media");
        assert_eq!(all[0].path, "/volume1/media");
        assert_eq!(all[0].filesystem, "btrfs");
        assert!(!all[0].time_backup);
        assert!(all[1].time_backup);
    }

    #[test]
    fn a_name_wins_over_an_id_and_an_id_is_the_fallback() {
        let all = shares();
        // "7" is the name of share 8 and the id of share 7: the name wins.
        assert_eq!(pick_share(&all, "7").unwrap().id, 8);
        assert_eq!(pick_share(&all, "media").unwrap().id, 7);
        // No share is called "8", so it is read as an id.
        assert_eq!(pick_share(&all, "8").unwrap().name, "7");
    }

    #[test]
    fn an_unknown_share_is_not_found() {
        let err = pick_share(&shares(), "nope").unwrap_err();
        assert!(matches!(
            err,
            UgosError::NotFound { kind: "share", ref name } if name == "nope"
        ));
        assert!(pick_share(&shares(), "99").is_err());
    }

    /// `filemgr/getShareFolderInfo`.
    const SHARE_INFO: &str = r#"{"data_check": false, "encrypt_status": 0, "id": 42,
        "name": "scratch", "network_places_hide": true, "no_permission_hide": true,
        "owner": "root", "owner_name": "", "owner_type": "user",
        "path": "/volume1/scratch", "quota": 5, "uid": 0, "unit": 1,
        "volume": "/volume1"}"#;

    #[test]
    fn share_settings_deserialize() {
        let info: ShareInfo = serde_json::from_str(SHARE_INFO).unwrap();
        assert_eq!(info.id, 42);
        assert_eq!(info.volume, "/volume1");
        assert_eq!((info.quota, info.unit), (5, 1));
        assert!(info.network_places_hide);
        assert!(info.no_permission_hide);
        assert!(!info.data_check);
    }

    #[test]
    fn the_update_body_nests_the_settings_and_lifts_one_flag() {
        let info: ShareInfo = serde_json::from_str(SHARE_INFO).unwrap();
        let body = update_body(&info);
        assert_eq!(body["id"], 42);
        let nested = &body["set_share_folder_info"];
        assert_eq!(nested["name"], "scratch");
        assert_eq!(nested["volume"], "/volume1");
        assert_eq!(nested["quota"], 5);
        assert_eq!(nested["unit"], 1);
        assert_eq!(nested["network_places_hide"], true);
        assert_eq!(nested["data_check"], false);
        // `no_permission_hide` travels beside the settings, not inside them.
        assert!(nested.get("no_permission_hide").is_none());
        assert_eq!(body["no_permission_hide"], true);
        assert_eq!(body["permissions"], serde_json::json!([]));
        // Nothing sits at the top level that UGOS would silently ignore.
        assert!(body.get("quota").is_none());
    }

    #[test]
    fn a_new_share_defaults_to_what_the_web_ui_offers() {
        let body = serde_json::to_value(NewShare::new("/volume1", "scratch")).unwrap();
        assert_eq!(body["volume"], "/volume1");
        assert_eq!(body["name"], "scratch");
        assert_eq!(body["quota"], 0);
        assert_eq!(body["unit"], 1);
        assert_eq!(body["data_check"], false);
        assert_eq!(body["network_places_hide"], false);
        assert_eq!(body["no_permission_hide"], true);
        assert_eq!(body["enable_recycle"], true);
        assert_eq!(body["allow_only_admin_recycle"], true);
        assert_eq!(body["permissions"], serde_json::json!([]));
        // Encryption is never asked for.
        assert_eq!(body["open_encrypt"], false);
        assert_eq!(body["encrypt_session_id"], "");
    }

    #[test]
    fn the_created_share_names_its_id_and_path() {
        let created: CreatedShare = serde_json::from_str(
            r#"{"id": 42, "name": "scratch", "path": "/volume1/scratch", "owner": "root"}"#,
        )
        .unwrap();
        assert_eq!(created.id, 42);
        assert_eq!(created.path, "/volume1/scratch");
    }

    /// `v2/filemgr/pathAcl`, as a share with one admin group answers.
    const PATH_ACL: &str = r#"{"acls": [{"full_name": "admin", "gid": 10, "name": "admin",
        "parent_permissions": -1, "permission_mask": 7, "select_permissions": 7,
        "users": [{"full_name": "fabian", "is_admin": true, "parent_permissions": -1,
                   "permission_mask": 7, "select_permissions": -1, "uid": 1000,
                   "username": "fabian"}]}],
        "no_permission_hide": true, "total_users": 1}"#;

    #[test]
    fn an_acl_lists_groups_with_their_members() {
        let acl: PathAcl = serde_json::from_str(PATH_ACL).unwrap();
        assert_eq!(acl.total_users, 1);
        assert!(acl.no_permission_hide);
        let group = &acl.acls[0];
        assert_eq!(group.name, "admin");
        assert_eq!(group.select_permissions, 7);
        let user = &group.users[0];
        assert_eq!(user.username, "fabian");
        assert!(user.is_admin);
        // Nothing set for the user, so the group's access applies.
        assert_eq!(user.select_permissions, -1);
        assert_eq!(user.permission_mask, 7);
    }

    #[test]
    fn masks_read_the_way_the_web_ui_reads_them() {
        assert_eq!(Access::from_mask(0), Access::None);
        assert_eq!(Access::from_mask(4), Access::Read);
        assert_eq!(Access::from_mask(5), Access::Read);
        assert_eq!(Access::from_mask(6), Access::ReadWrite);
        assert_eq!(Access::from_mask(7), Access::ReadWrite);
        assert_eq!(Access::from_mask(-1), Access::Inherit);
        assert_eq!(Access::from_mask(3), Access::Other(3));
    }

    #[test]
    fn settings_write_the_masks_the_web_ui_writes() {
        assert_eq!(Access::None.mask(), 0);
        assert_eq!(Access::Read.mask(), 4);
        assert_eq!(Access::ReadWrite.mask(), 7);
        assert_eq!(Access::Inherit.mask(), -1);
        assert_eq!(Access::Other(3).mask(), 3);
    }

    #[test]
    fn access_words_round_trip_and_reject_nonsense() {
        for access in [
            Access::None,
            Access::Read,
            Access::ReadWrite,
            Access::Inherit,
        ] {
            assert_eq!(Access::parse(&access.label()), Ok(access));
        }
        assert_eq!(Access::parse(" RW "), Ok(Access::ReadWrite));
        assert_eq!(Access::parse("write"), Ok(Access::ReadWrite));
        assert_eq!(Access::parse("maybe"), Err("maybe".to_owned()));
        // A mask this client does not know cannot be asked for by word.
        assert!(Access::parse("mask 3").is_err());
    }

    #[test]
    fn the_acl_body_carries_only_the_changes() {
        let changes = [
            AclChange {
                kind: PrincipalKind::User,
                name: "fabian".into(),
                permission_mask: Access::Read.mask(),
                is_admin: true,
            },
            AclChange {
                kind: PrincipalKind::Group,
                name: "users".into(),
                permission_mask: Access::Inherit.mask(),
                is_admin: false,
            },
        ];
        let body = acl_body("/volume1/scratch", &changes);
        assert_eq!(body["path"], "/volume1/scratch");
        let list = body["permissions"].as_array().unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0]["type"], "user");
        assert_eq!(list[0]["name"], "fabian");
        assert_eq!(list[0]["permission_mask"], 4);
        assert_eq!(list[0]["is_admin"], true);
        assert_eq!(list[1]["type"], "group");
        assert_eq!(list[1]["permission_mask"], -1);
    }

    #[test]
    fn the_smb_config_keeps_what_is_worth_showing() {
        let json = r#"{"status": true, "nas_name": "nas", "work_group": "HOME",
            "wins_server": "", "is_joined_domain": false,
            "hide_share_dir_for_no_permission": true, "smb_protocols": [2],
            "min_protocol": 0, "max_protocol": 0, "encrypt_mode": 0,
            "server_sign_mode": 1, "enable_ntlmv1": false,
            "enable_op_locks": true, "veto_files": ""}"#;
        let config: SmbConfig = serde_json::from_str(json).unwrap();
        assert!(config.status);
        assert_eq!(config.work_group, "HOME");
        assert_eq!(config.smb_protocols, vec![2]);
        assert_eq!(config.server_sign_mode, 1);
        assert!(!config.enable_ntlmv1);
        assert!(config.hide_share_dir_for_no_permission);
    }

    #[test]
    fn the_advanced_config_maps_its_camel_case_names() {
        let json = r#"{"enableBonjourService": false, "enableTimeMachine": true,
            "enableUPnPService": true, "enableWsdd2Service": true,
            "sharedFileInfos": [{"id": 4, "modify_time": 1793483009, "name": "docker",
                                  "path": "/volume1/docker", "time_backup": false, "type": 1}]}"#;
        let advanced: SmbAdvanced = serde_json::from_str(json).unwrap();
        assert!(!advanced.bonjour);
        assert!(advanced.wsdd2);
        assert!(advanced.upnp);
        assert!(advanced.time_machine);
        assert_eq!(advanced.shares.len(), 1);
        assert_eq!(advanced.shares[0].name, "docker");
    }
}

// ── User accounts ───────────────────────────────────────────────────

mod users {
    use crate::api::users::{create_body, edit_body, pick_user};
    use crate::error::UgosError;
    use crate::types::syslog::{GroupRef, User};
    use crate::types::users::{DISABLED, DeleteCheck, EXPIRES_NEVER, NewUser, ROLE_USER, UserEdit};

    /// `user/list`, trimmed to what matters and with made-up people.
    const USER_LIST: &str = r#"{"list": [
        {"account_type": 0, "deny_change_pwd": false, "description": "",
         "dir_quota": 0, "email": "a****a@example.org",
         "email_encrypted": "AAAA", "enable_home_dir": true,
         "expire_date": "", "expire_time": -1,
         "groups": [{"gid": 10, "name": "admin"}, {"gid": 100, "name": "users"}],
         "is_expired": false, "role": "admin", "uid": 1000, "unit": 0,
         "username": "anna", "otp_info": {"secret": ""}},
        {"description": "guest", "email": "", "expire_time": 1, "groups": null,
         "role": "users", "uid": 1001, "username": "1000"}
    ]}"#;

    fn users() -> Vec<User> {
        #[derive(serde::Deserialize)]
        struct List {
            list: Vec<User>,
        }
        serde_json::from_str::<List>(USER_LIST).unwrap().list
    }

    #[test]
    fn a_listed_account_keeps_what_an_edit_needs() {
        let all = users();
        let anna = &all[0];
        assert_eq!(anna.role, "admin");
        assert_eq!(anna.uid, 1000);
        assert_eq!(anna.groups.len(), 2);
        assert_eq!(anna.groups[1].name, "users");
        assert_eq!(anna.expire_time, EXPIRES_NEVER);
        assert!(anna.enable_home_dir);
        assert_eq!(anna.email_encrypted, "AAAA");
    }

    #[test]
    fn missing_fields_and_null_groups_do_not_break_the_list() {
        let guest = &users()[1];
        assert_eq!(guest.expire_time, DISABLED);
        assert_eq!(guest.groups, Vec::<GroupRef>::new());
        assert_eq!(guest.dir_quota, 0);
    }

    #[test]
    fn the_obscured_email_does_not_go_back_out_in_json() {
        let json = serde_json::to_value(&users()[0]).unwrap();
        assert!(json.get("email_encrypted").is_none());
        assert_eq!(json["email"], "a****a@example.org");
    }

    #[test]
    fn a_name_wins_over_a_uid_and_a_uid_still_finds() {
        let all = users();
        assert_eq!(pick_user(&all, "1000").unwrap().uid, 1001);
        assert_eq!(pick_user(&all, "anna").unwrap().uid, 1000);
        assert_eq!(pick_user(&all, "1001").unwrap().username, "1000");
        assert!(matches!(
            pick_user(&all, "nobody"),
            Err(UgosError::NotFound { kind: "user", .. })
        ));
    }

    #[test]
    fn the_create_body_carries_the_sealed_password_only() {
        let mut new = NewUser::new("ben", "secret-in-the-clear");
        new.groups = vec!["family".into()];
        new.email = "ben@example.org".into();
        let body = create_body(&new, "SEALED");
        assert_eq!(body["password"], "SEALED");
        assert!(!body.to_string().contains("secret-in-the-clear"));
        assert_eq!(body["groups"], serde_json::json!([{"name": "family"}]));
        assert_eq!(body["role"], ROLE_USER);
        assert_eq!(body["enable_home_dir"], true);
        assert_eq!(body["email"], "ben@example.org");
    }

    fn edit() -> UserEdit {
        UserEdit {
            uid: 1001,
            username: "ben".into(),
            email: "ben@example.org".into(),
            description: "Ben".into(),
            role: ROLE_USER.into(),
            groups: vec!["family".into()],
            enable_home_dir: true,
            deny_change_pwd: false,
            dir_quota: 0,
            unit: 1,
            password: Some("secret-in-the-clear".into()),
            expire_time: EXPIRES_NEVER,
            expire_date: String::new(),
        }
    }

    #[test]
    fn the_edit_body_always_names_the_email_and_the_uid() {
        // Leaving the email out made UGOS blank it.
        let body = edit_body(&edit(), None);
        assert_eq!(body["email"], "ben@example.org");
        assert_eq!(body["uid"], 1001);
        assert_eq!(body["expire_time"], -1);
        assert_eq!(body["groups"], serde_json::json!([{"name": "family"}]));
    }

    #[test]
    fn the_edit_body_has_a_password_only_when_there_is_a_sealed_one() {
        assert!(edit_body(&edit(), None).get("password").is_none());
        let body = edit_body(&edit(), Some("SEALED"));
        assert_eq!(body["password"], "SEALED");
        assert!(!body.to_string().contains("secret-in-the-clear"));
    }

    #[test]
    fn the_delete_check_reads_what_would_be_lost() {
        let check: DeleteCheck =
            serde_json::from_str(r#"{"safespace": false, "snapshot": true}"#).unwrap();
        assert!(check.snapshot);
        assert!(!check.safespace);
        let none: DeleteCheck = serde_json::from_str("{}").unwrap();
        assert_eq!(none, DeleteCheck::default());
    }

    #[test]
    fn a_group_with_and_without_members_parses() {
        use crate::types::users::Group;
        #[derive(serde::Deserialize)]
        struct List {
            list: Vec<Group>,
        }
        let groups = serde_json::from_str::<List>(
            r#"{"list": [
                {"description": "family", "gid": 1000, "groupname": "family",
                 "users": [{"name": "anna", "uid": 1001}], "utime": 1794076280},
                {"description": "", "gid": 1001, "groupname": "empty", "users": null, "utime": 0}
            ]}"#,
        )
        .unwrap()
        .list;
        assert_eq!(groups[0].groupname, "family");
        assert_eq!(groups[0].users[0].name, "anna");
        assert_eq!(groups[0].users[0].uid, 1001);
        assert_eq!(groups[1].users, Vec::new());
    }
}

// ── Storage Manager ─────────────────────────────────────────────────

mod storage {
    use crate::api::storage::{pick_disk, pick_pool, pick_volume};
    use crate::error::UgosError;
    use crate::types::common::ResultWrapper;
    use crate::types::storage::{Disk, Pool, Smart, StorageVolume, TestPlan, TestRecord};

    const POOLS: &str = r#"{"result": [{"name": "pool1", "label": "Storage Pool 1",
        "level": "raid5", "status": 0, "total": 1000, "used": 400, "free": 600,
        "total_disk_num": 2, "progress": 0, "remaining_time": -1, "spare": null,
        "disks": [{"dev_name": "/dev/sda", "label": "Disk 1", "size": 500, "slot": "sata1", "status": 0},
                  {"dev_name": "/dev/sdb", "label": "Disk 2", "size": 500, "slot": "sata2", "status": 0}],
        "trim": {"enable": true, "is_support": true, "next_start_time": "2026-11-18 10:00:00",
                 "scheduled": {"first_run_datetime": "2026-01-18 10:00:00", "interval": 1, "interval_unit": 5}},
        "data_scrub": {"is_cron": false, "last_time": "--", "next_start_time": "--", "process": 0, "status": 0},
        "support_extend": true, "support_delete": true, "volumes": [{"name": "volume1"}]}]}"#;

    #[test]
    fn a_pool_keeps_what_the_commands_show() {
        let pools = serde_json::from_str::<ResultWrapper<Vec<Pool>>>(POOLS)
            .unwrap()
            .result;
        let pool = &pools[0];
        assert_eq!(
            (pool.level.as_str(), pool.total, pool.used),
            ("raid5", 1000, 400)
        );
        assert_eq!(pool.disks.len(), 2);
        assert_eq!(pool.disks[1].slot, "sata2");
        assert!(pool.trim.as_ref().unwrap().enable);
        assert_eq!(pool.data_scrub.as_ref().unwrap().last_time, "--");
        assert_eq!(pool.remaining_time, -1);
    }

    #[test]
    fn a_pool_is_found_by_name_or_label() {
        let pools = serde_json::from_str::<ResultWrapper<Vec<Pool>>>(POOLS)
            .unwrap()
            .result;
        assert_eq!(pick_pool(&pools, "pool1").unwrap().label, "Storage Pool 1");
        assert_eq!(pick_pool(&pools, "storage pool 1").unwrap().name, "pool1");
        assert!(matches!(
            pick_pool(&pools, "pool9"),
            Err(UgosError::NotFound { kind: "pool", .. })
        ));
    }

    #[test]
    fn a_volume_is_found_by_name_label_or_any_mount_path() {
        let volumes = serde_json::from_str::<ResultWrapper<Vec<StorageVolume>>>(
            r#"{"result": [{"name": "volume1", "label": "Volume 1", "poolname": "pool1",
                "mntpath": "/volume1", "mntpaths": ["/volume1", "/home"], "filesystem": "btrfs",
                "total": 100, "used": 40, "available": 60, "is_home": true}]}"#,
        )
        .unwrap()
        .result;
        for which in ["volume1", "VOLUME 1", "/volume1", "/volume1/", "/home"] {
            assert_eq!(
                pick_volume(&volumes, which).unwrap().name,
                "volume1",
                "{which}"
            );
        }
        assert!(pick_volume(&volumes, "/volume2").is_err());
    }

    #[test]
    fn a_disk_is_found_by_name_path_slot_or_label() {
        let disks = serde_json::from_str::<ResultWrapper<Vec<Disk>>>(
            r#"{"result": [{"name": "nvme0n1", "dev_name": "/dev/nvme0n1", "slot": "nvme1",
                "label": "M.2 Hard Drive 1", "size": 8, "temperature": 48, "used_for": "Storage Pool 1"}]}"#,
        )
        .unwrap()
        .result;
        for which in [
            "nvme0n1",
            "/dev/nvme0n1",
            "nvme1",
            "NVME1",
            "m.2 hard drive 1",
        ] {
            assert_eq!(pick_disk(&disks, which).unwrap().name, "nvme0n1", "{which}");
        }
        assert!(matches!(
            pick_disk(&disks, "sda"),
            Err(UgosError::NotFound { kind: "disk", .. })
        ));
    }

    #[test]
    fn a_smart_report_keeps_numbers_and_lists() {
        let smart: Smart = serde_json::from_str(
            r#"{"is_testing": false, "last": {"last_time": 0, "next_time": 1794405600, "status": 1},
                "long_test_cost": 5, "short_test_cost": 5, "progress": 0, "remain_time": 0,
                "report": [{"id": 1, "label": "Temperature", "name": "temperature", "value": 48},
                           {"id": 13, "label": "Temperature sensors", "name": "temperature_sensors", "value": [58, 47, 47]}]}"#,
        )
        .unwrap();
        assert_eq!(smart.last.next_time, 1_794_405_600);
        assert_eq!(smart.report[0].value, 48);
        assert_eq!(smart.report[1].value, serde_json::json!([58, 47, 47]));
    }

    #[test]
    fn test_history_and_plans_parse() {
        let history = serde_json::from_str::<ResultWrapper<Vec<TestRecord>>>(
            r#"{"result": [{"err_code": "", "smart_type": 1, "status": 1, "time": 1793800800, "type": 0}]}"#,
        )
        .unwrap()
        .result;
        assert_eq!(history[0].smart_type, 1);
        let plans = serde_json::from_str::<ResultWrapper<Vec<TestPlan>>>(
            r#"{"result": [{"cron_id": 1, "name": "Test plan 1", "next_time": "2026-11-11 15:00:00",
                "disks": ["/dev/nvme0n1"], "schedule": {"interval": 1, "interval_unit": 6}}]}"#,
        )
        .unwrap()
        .result;
        assert_eq!(plans[0].schedule.interval_unit, 6);
        assert_eq!(plans[0].disks, ["/dev/nvme0n1"]);
    }
}
