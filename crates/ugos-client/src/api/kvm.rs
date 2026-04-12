//! KVM virtual machine management API.
//!
//! All methods accept display names or UUIDs. Display names are resolved
//! to UUIDs internally via [`KvmApi::vm_list`].

use crate::client::UgosClient;
use crate::error::{Result, UgosError};
use crate::types::common::ResultWrapper;
use crate::types::kvm::{
    HostInfo, ImageInfo, LogPage, NetworkDetail, NetworkSummary, Snapshot, StorageInfo, UsbDevice,
    VmDetail, VmSummary, VncLink,
};

/// KVM management operations on a UGOS NAS.
#[allow(clippy::module_name_repetitions)]
pub trait KvmApi {
    // ── VM ──────────────────────────────────────────────────────────

    /// List all virtual machines.
    fn vm_list(&self) -> impl Future<Output = Result<Vec<VmSummary>>> + Send;
    /// Show detailed VM configuration.
    fn vm_show(&self, name: &str) -> impl Future<Output = Result<VmDetail>> + Send;
    /// Power on a VM.
    fn vm_start(&self, name: &str) -> impl Future<Output = Result<()>> + Send;
    /// Shut down a VM (graceful or forced).
    fn vm_stop(&self, name: &str, force: bool) -> impl Future<Output = Result<()>> + Send;
    /// Reboot a VM (graceful or forced).
    fn vm_reboot(&self, name: &str, force: bool) -> impl Future<Output = Result<()>> + Send;
    /// Delete a VM.
    fn vm_delete(&self, name: &str) -> impl Future<Output = Result<()>> + Send;
    /// Get host hardware info (CPU cores, memory).
    fn host_info(&self) -> impl Future<Output = Result<HostInfo>> + Send;

    // ── Snapshot ────────────────────────────────────────────────────

    /// List snapshots for a VM.
    fn snapshot_list(&self, vm: &str) -> impl Future<Output = Result<Vec<Snapshot>>> + Send;
    /// Create a snapshot.
    fn snapshot_create(&self, vm: &str, name: &str) -> impl Future<Output = Result<()>> + Send;
    /// Delete a snapshot.
    fn snapshot_delete(&self, vm: &str, name: &str) -> impl Future<Output = Result<()>> + Send;
    /// Revert to a snapshot.
    fn snapshot_revert(&self, vm: &str, name: &str) -> impl Future<Output = Result<()>> + Send;
    /// Rename a snapshot.
    fn snapshot_rename(
        &self,
        vm: &str,
        old: &str,
        new: &str,
    ) -> impl Future<Output = Result<()>> + Send;

    // ── Network ─────────────────────────────────────────────────────

    /// List KVM networks.
    fn network_list(&self) -> impl Future<Output = Result<Vec<NetworkSummary>>> + Send;
    /// Show network details.
    fn network_show(&self, name: &str) -> impl Future<Output = Result<NetworkDetail>> + Send;
    /// Create a KVM network.
    fn network_create(&self, network: &NetworkDetail) -> impl Future<Output = Result<()>> + Send;
    /// Update a KVM network.
    fn network_update(&self, network: &NetworkDetail) -> impl Future<Output = Result<()>> + Send;
    /// Delete a KVM network.
    fn network_delete(&self, name: &str) -> impl Future<Output = Result<()>> + Send;

    // ── Storage ─────────────────────────────────────────────────────

    /// List storage volumes available to KVM.
    fn storage_list(&self) -> impl Future<Output = Result<Vec<StorageInfo>>> + Send;
    /// Check which VMs use a storage volume.
    fn storage_check_usage(
        &self,
        name: &str,
        uuid: &str,
    ) -> impl Future<Output = Result<Vec<String>>> + Send;
    /// Add a storage volume to KVM.
    fn storage_add(&self, name: &str, uuid: &str) -> impl Future<Output = Result<()>> + Send;
    /// Remove a storage volume from KVM.
    fn storage_delete(&self, name: &str, uuid: &str) -> impl Future<Output = Result<()>> + Send;

    // ── Image ───────────────────────────────────────────────────────

    /// List ISO/disk images.
    fn image_list(&self) -> impl Future<Output = Result<Vec<ImageInfo>>> + Send;
    /// Delete an image.
    fn image_delete(
        &self,
        file_name: &str,
        image_name: &str,
    ) -> impl Future<Output = Result<()>> + Send;
    /// Check which VMs use an image.
    fn image_check_usage(&self, name: &str) -> impl Future<Output = Result<Vec<String>>> + Send;
    /// Check if an image name is available.
    fn image_check_name(&self, name: &str) -> impl Future<Output = Result<bool>> + Send;

    // ── USB ─────────────────────────────────────────────────────────

    /// List USB devices for a VM.
    fn usb_list(&self, vm: &str) -> impl Future<Output = Result<Vec<UsbDevice>>> + Send;

    // ── VNC ─────────────────────────────────────────────────────────

    /// List VNC links for a VM.
    fn vnc_list(&self, vm: &str) -> impl Future<Output = Result<Vec<VncLink>>> + Send;
    /// Generate a noVNC link for a VM.
    fn vnc_generate(
        &self,
        vm: &str,
        source_url: &str,
    ) -> impl Future<Output = Result<String>> + Send;

    // ── Logs ────────────────────────────────────────────────────────

    /// Search KVM logs.
    fn log_search(&self, page: u32, page_size: u32)
    -> impl Future<Output = Result<LogPage>> + Send;
    /// Get all operator usernames from logs.
    fn log_operators(&self) -> impl Future<Output = Result<Vec<String>>> + Send;

    // ── Session ─────────────────────────────────────────────────────

    /// Send a heartbeat to keep the session alive.
    fn heartbeat(&self) -> impl Future<Output = Result<()>> + Send;
}

// ── Name resolution ─────────────────────────────────────────────────

async fn resolve_vm(client: &UgosClient, name: &str) -> Result<(String, String)> {
    let vms: ResultWrapper<Vec<VmSummary>> = client.get("kvm/manager/ShowLocalVirtualList").await?;
    let vm = vms
        .result
        .iter()
        .find(|v| v.vir_name == name || v.vir_display_name.eq_ignore_ascii_case(name))
        .ok_or_else(|| UgosError::NotFound {
            kind: "VM",
            name: name.to_owned(),
        })?;
    Ok((vm.vir_name.clone(), vm.vir_display_name.clone()))
}

// ── Implementation ──────────────────────────────────────────────────

impl KvmApi for UgosClient {
    // ── VM ──────────────────────────────────────────────────────────

    async fn vm_list(&self) -> Result<Vec<VmSummary>> {
        let resp: ResultWrapper<Vec<VmSummary>> =
            self.get("kvm/manager/ShowLocalVirtualList").await?;
        Ok(resp.result)
    }

    async fn vm_show(&self, name: &str) -> Result<VmDetail> {
        let (uuid, _) = resolve_vm(self, name).await?;
        self.get_with_params("kvm/manager/ShowLocalVirtualMachine", &[("name", &uuid)])
            .await
    }

    async fn vm_start(&self, name: &str) -> Result<()> {
        let (uuid, display) = resolve_vm(self, name).await?;
        let _: ResultWrapper<String> = self
            .get_with_params(
                "kvm/manager/PowerOn",
                &[
                    ("name", uuid.as_str()),
                    ("virtualMachineDisplayName", display.as_str()),
                ],
            )
            .await?;
        Ok(())
    }

    async fn vm_stop(&self, name: &str, force: bool) -> Result<()> {
        let (uuid, display) = resolve_vm(self, name).await?;
        let endpoint = if force {
            "kvm/manager/ForcedShutdown"
        } else {
            "kvm/manager/Shutdown"
        };
        let _: ResultWrapper<String> = self
            .get_with_params(
                endpoint,
                &[
                    ("name", uuid.as_str()),
                    ("virtualMachineDisplayName", display.as_str()),
                ],
            )
            .await?;
        Ok(())
    }

    async fn vm_reboot(&self, name: &str, force: bool) -> Result<()> {
        let (uuid, display) = resolve_vm(self, name).await?;
        if force {
            let _: ResultWrapper<String> = self
                .get_with_params(
                    "kvm/manager/ForcedRestart",
                    &[
                        ("name", uuid.as_str()),
                        ("virtualMachineDisplayName", display.as_str()),
                    ],
                )
                .await?;
        } else {
            let _: ResultWrapper<String> = self
                .get_with_params("kvm/manager/Reboot", &[("name", uuid.as_str())])
                .await?;
        }
        Ok(())
    }

    async fn vm_delete(&self, name: &str) -> Result<()> {
        let (uuid, display) = resolve_vm(self, name).await?;
        let _: ResultWrapper<String> = self
            .get_with_params(
                "kvm/manager/DeleteVirtualMachine",
                &[
                    ("name", uuid.as_str()),
                    ("virtualMachineDisplayName", display.as_str()),
                ],
            )
            .await?;
        Ok(())
    }

    async fn host_info(&self) -> Result<HostInfo> {
        self.get("kvm/manager/ShowNativeInfo").await
    }

    // ── Snapshot ────────────────────────────────────────────────────

    async fn snapshot_list(&self, vm: &str) -> Result<Vec<Snapshot>> {
        let (uuid, _) = resolve_vm(self, vm).await?;
        let resp: ResultWrapper<Vec<Snapshot>> = self
            .get_with_params("kvm/manager/ShowListSnapshot", &[("name", uuid.as_str())])
            .await?;
        Ok(resp.result)
    }

    async fn snapshot_create(&self, vm: &str, name: &str) -> Result<()> {
        let (uuid, display) = resolve_vm(self, vm).await?;
        let _: ResultWrapper<String> = self
            .get_with_params(
                "kvm/manager/GenerateSnapshot",
                &[
                    ("name", name),
                    ("virName", uuid.as_str()),
                    ("virtualMachineDisplayName", display.as_str()),
                ],
            )
            .await?;
        Ok(())
    }

    async fn snapshot_delete(&self, vm: &str, name: &str) -> Result<()> {
        let (uuid, _) = resolve_vm(self, vm).await?;
        let _: ResultWrapper<String> = self
            .get_with_params(
                "kvm/manager/DeleteSnapshot",
                &[("name", name), ("virName", uuid.as_str())],
            )
            .await?;
        Ok(())
    }

    async fn snapshot_revert(&self, _vm: &str, name: &str) -> Result<()> {
        let _: ResultWrapper<String> = self
            .get_with_params("kvm/manager/RevertSnapshot", &[("name", name)])
            .await?;
        Ok(())
    }

    async fn snapshot_rename(&self, _vm: &str, old: &str, new: &str) -> Result<()> {
        let body = serde_json::json!({"name": old, "displayName": new});
        let _: ResultWrapper<String> = self.post("kvm/manager/RenameSnapshot", &body).await?;
        Ok(())
    }

    // ── Network ─────────────────────────────────────────────────────

    async fn network_list(&self) -> Result<Vec<NetworkSummary>> {
        let resp: ResultWrapper<Vec<NetworkSummary>> =
            self.get("kvm/network/ShowNetworkList").await?;
        Ok(resp.result)
    }

    async fn network_show(&self, name: &str) -> Result<NetworkDetail> {
        let resp: ResultWrapper<NetworkDetail> = self
            .get_with_params("kvm/network/GetNetworkByName", &[("name", name)])
            .await?;
        Ok(resp.result)
    }

    async fn network_create(&self, network: &NetworkDetail) -> Result<()> {
        let _: ResultWrapper<String> = self.post("kvm/network/CreateNetwork", network).await?;
        Ok(())
    }

    async fn network_update(&self, network: &NetworkDetail) -> Result<()> {
        let _: ResultWrapper<String> = self.post("kvm/network/UpdateNetwork", network).await?;
        Ok(())
    }

    async fn network_delete(&self, name: &str) -> Result<()> {
        let _: ResultWrapper<String> = self
            .get_with_params("kvm/network/DeleteNetwork", &[("name", name)])
            .await?;
        Ok(())
    }

    // ── Storage ─────────────────────────────────────────────────────

    async fn storage_list(&self) -> Result<Vec<StorageInfo>> {
        let resp: ResultWrapper<Vec<StorageInfo>> = self.get("kvm/storage/ShowStorageList").await?;
        Ok(resp.result)
    }

    async fn storage_check_usage(&self, name: &str, uuid: &str) -> Result<Vec<String>> {
        let resp: ResultWrapper<Vec<String>> = self
            .get_with_params(
                "kvm/storage/CheckStorage",
                &[("name", name), ("uuid", uuid)],
            )
            .await?;
        Ok(resp.result)
    }

    async fn storage_add(&self, name: &str, uuid: &str) -> Result<()> {
        let body = serde_json::json!({"storageName": name, "storageUUID": uuid});
        let _: ResultWrapper<String> = self.post("kvm/storage/AddStorage", &body).await?;
        Ok(())
    }

    async fn storage_delete(&self, name: &str, uuid: &str) -> Result<()> {
        let _: ResultWrapper<String> = self
            .get_with_params(
                "kvm/storage/DeleteStorage",
                &[("name", name), ("uuid", uuid)],
            )
            .await?;
        Ok(())
    }

    // ── Image ───────────────────────────────────────────────────────

    async fn image_list(&self) -> Result<Vec<ImageInfo>> {
        let resp: ResultWrapper<Vec<ImageInfo>> = self.get("kvm/image/ShowImageList").await?;
        Ok(resp.result)
    }

    async fn image_delete(&self, file_name: &str, image_name: &str) -> Result<()> {
        let _: ResultWrapper<String> = self
            .get_with_params(
                "kvm/image/DeleteImage",
                &[("fileName", file_name), ("name", image_name)],
            )
            .await?;
        Ok(())
    }

    async fn image_check_usage(&self, name: &str) -> Result<Vec<String>> {
        let resp: ResultWrapper<Vec<String>> = self
            .get_with_params("kvm/image/CheckImageUsage", &[("name", name)])
            .await?;
        Ok(resp.result)
    }

    async fn image_check_name(&self, name: &str) -> Result<bool> {
        let resp: ResultWrapper<bool> = self
            .get_with_params("kvm/image/CheckImageName", &[("name", name)])
            .await?;
        Ok(resp.result)
    }

    // ── USB ─────────────────────────────────────────────────────────

    async fn usb_list(&self, vm: &str) -> Result<Vec<UsbDevice>> {
        let (uuid, _) = resolve_vm(self, vm).await?;
        let resp: ResultWrapper<Vec<UsbDevice>> = self
            .get_with_params("kvm/usb/USBList", &[("vmName", uuid.as_str())])
            .await?;
        Ok(resp.result)
    }

    // ── VNC ─────────────────────────────────────────────────────────

    async fn vnc_list(&self, vm: &str) -> Result<Vec<VncLink>> {
        let (uuid, _) = resolve_vm(self, vm).await?;
        let resp: ResultWrapper<Vec<VncLink>> = self
            .get_with_params("kvm/vnc/ListAllLink", &[("virName", uuid.as_str())])
            .await?;
        Ok(resp.result)
    }

    async fn vnc_generate(&self, vm: &str, source_url: &str) -> Result<String> {
        let (uuid, _) = resolve_vm(self, vm).await?;
        let body = serde_json::json!({"virName": uuid, "type": 0, "sourceUrl": source_url});
        let resp: ResultWrapper<String> = self.post("kvm/vnc/GenerateNoVNClink", &body).await?;
        Ok(resp.result)
    }

    // ── Logs ────────────────────────────────────────────────────────

    async fn log_search(&self, page: u32, page_size: u32) -> Result<LogPage> {
        let body = serde_json::json!({
            "pageNum": page,
            "pageSize": page_size,
            "operator": "",
            "startTime": "",
            "endTime": "",
            "createTimeSort": "desc",
            "operatorSort": ""
        });
        self.post("kvm/logs/PageSearchLogs", &body).await
    }

    async fn log_operators(&self) -> Result<Vec<String>> {
        let resp: ResultWrapper<Vec<String>> = self.get("kvm/logs/GetAllOperator").await?;
        Ok(resp.result)
    }

    // ── Session ─────────────────────────────────────────────────────

    async fn heartbeat(&self) -> Result<()> {
        let _: serde_json::Value = self.get("verify/heartbeat").await?;
        Ok(())
    }
}
