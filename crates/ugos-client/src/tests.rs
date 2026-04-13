//! Unit tests for the UGOS client library.

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
    let json = r#"{
        "name": "snap-1234",
        "displayName": "Before Update",
        "isCurrent": true,
        "createTime": 1776033624
    }"#;
    let s: Snapshot = serde_json::from_str(json).unwrap();
    assert_eq!(s.display_name, "Before Update");
    assert!(s.is_current);
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
    assert!(u.used_by.is_empty());
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
