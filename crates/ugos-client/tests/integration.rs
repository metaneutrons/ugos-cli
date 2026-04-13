//! Integration tests against a real UGOS NAS.
//!
//! Run with: `cargo test -p ugos-client --features integration -- --ignored`
//!
//! Requires env vars: `UGOS_HOST`, `UGOS_USER`, `UGOS_PASSWORD`

#![cfg(feature = "integration")]

use ugos_client::api::docker::DockerApi;
use ugos_client::api::kvm::KvmApi;
use ugos_client::{Credentials, UgosClient};

fn creds() -> (String, Credentials) {
    let host = std::env::var("UGOS_HOST").expect("UGOS_HOST required");
    let creds = Credentials {
        username: std::env::var("UGOS_USER").expect("UGOS_USER required"),
        password: std::env::var("UGOS_PASSWORD").expect("UGOS_PASSWORD required"),
    };
    (host, creds)
}

async fn connect() -> UgosClient {
    let (host, creds) = creds();
    UgosClient::connect(&host, 9443, creds)
        .await
        .expect("failed to connect")
}

#[tokio::test]
#[ignore]
async fn auth_and_session() {
    let client = connect().await;
    let session = client.session().await;
    assert!(!session.token.is_empty(), "token should not be empty");
}

#[tokio::test]
#[ignore]
async fn vm_list() {
    let client = connect().await;
    let vms = client.vm_list().await.expect("vm_list failed");
    // picard always has at least one VM
    assert!(!vms.is_empty(), "expected at least one VM");
    for vm in &vms {
        assert!(!vm.vir_name.is_empty());
        assert!(!vm.vir_display_name.is_empty());
        assert!(
            vm.status == "running" || vm.status == "shutoff",
            "unexpected status: {}",
            vm.status
        );
    }
}

#[tokio::test]
#[ignore]
async fn vm_show() {
    let client = connect().await;
    let vms = client.vm_list().await.expect("vm_list failed");
    let first = &vms[0];
    let detail = client
        .vm_show(&first.vir_display_name)
        .await
        .expect("vm_show failed");
    assert_eq!(detail.virtual_machine_name, first.vir_name);
    assert!(detail.core.value > 0);
    assert!(detail.memory.value > 0);
}

#[tokio::test]
#[ignore]
async fn host_info() {
    let client = connect().await;
    let info = client.host_info().await.expect("host_info failed");
    assert!(info.cores > 0);
    assert!(info.memory > 0);
}

#[tokio::test]
#[ignore]
async fn network_list() {
    let client = connect().await;
    let nets = client.network_list().await.expect("network_list failed");
    assert!(!nets.is_empty());
    assert!(nets.iter().any(|n| n.network_name == "vnet-bridge0"));
}

#[tokio::test]
#[ignore]
async fn storage_list() {
    let client = connect().await;
    let vols = client.storage_list().await.expect("storage_list failed");
    assert!(!vols.is_empty());
    assert!(vols.iter().any(|v| v.name == "volume1"));
}

#[tokio::test]
#[ignore]
async fn image_list() {
    let client = connect().await;
    let imgs = client.image_list().await.expect("image_list failed");
    // May be empty, just ensure it doesn't error
    let _ = imgs;
}

#[tokio::test]
#[ignore]
async fn docker_engine_status() {
    let client = connect().await;
    let status = client
        .docker_engine_status()
        .await
        .expect("docker_engine_status failed");
    assert!(
        status == "online" || status == "offline",
        "unexpected status: {status}"
    );
}

#[tokio::test]
#[ignore]
async fn docker_ps() {
    let client = connect().await;
    let page = client
        .container_list(1, 50)
        .await
        .expect("container_list failed");
    // Just ensure it parses
    let _ = page.total;
}

#[tokio::test]
#[ignore]
async fn docker_images() {
    let client = connect().await;
    let page = client
        .docker_image_list(1, 50)
        .await
        .expect("docker_image_list failed");
    let _ = page.original_total;
}

#[tokio::test]
#[ignore]
async fn docker_mirrors() {
    let client = connect().await;
    let mirrors = client.mirror_list().await.expect("mirror_list failed");
    assert!(!mirrors.is_empty());
    assert!(mirrors.iter().any(|m| m.alias == "DockerHub"));
}
