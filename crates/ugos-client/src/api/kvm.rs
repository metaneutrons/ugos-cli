//! KVM virtual machine management API.
//!
//! All methods accept display names or UUIDs. Display names are resolved
//! to UUIDs internally via [`KvmApi::vm_list`].

use crate::client::UgosClient;
use crate::error::{Result, UgosError};
use crate::types::common::ResultWrapper;
use crate::types::kvm::{
    HostInfo, ImageInfo, NetworkDetail, NetworkSummary, Snapshot, StorageInfo, UsbDevice, VmDetail,
    VmSummary,
};

/// KVM management operations on a UGOS NAS.
pub trait KvmApi {
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

    /// List KVM networks.
    fn network_list(&self) -> impl Future<Output = Result<Vec<NetworkSummary>>> + Send;

    /// Show network details.
    fn network_show(&self, name: &str) -> impl Future<Output = Result<NetworkDetail>> + Send;

    /// List storage volumes available to KVM.
    fn storage_list(&self) -> impl Future<Output = Result<Vec<StorageInfo>>> + Send;

    /// List ISO/disk images.
    fn image_list(&self) -> impl Future<Output = Result<Vec<ImageInfo>>> + Send;

    /// List USB devices for a VM.
    fn usb_list(&self, vm: &str) -> impl Future<Output = Result<Vec<UsbDevice>>> + Send;
}

// ── Name resolution ─────────────────────────────────────────────────

/// Resolve a display name or UUID to `(uuid, display_name)`.
///
/// If `name` looks like a UUID (contains a hyphen), it searches by `vir_name`.
/// Otherwise it searches by `vir_display_name`.
async fn resolve_vm(client: &UgosClient, name: &str) -> Result<(String, String)> {
    let vms: ResultWrapper<Vec<VmSummary>> = client.get("kvm/manager/ShowLocalVirtualList").await?;

    let vm = vms
        .result
        .iter()
        .find(|v| v.vir_name == name || v.vir_display_name.eq_ignore_ascii_case(name));

    let vm = vm.ok_or_else(|| UgosError::NotFound {
        kind: "VM",
        name: name.to_owned(),
    })?;

    Ok((vm.vir_name.clone(), vm.vir_display_name.clone()))
}

// ── Implementation ──────────────────────────────────────────────────

impl KvmApi for UgosClient {
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

    async fn storage_list(&self) -> Result<Vec<StorageInfo>> {
        let resp: ResultWrapper<Vec<StorageInfo>> = self.get("kvm/storage/ShowStorageList").await?;
        Ok(resp.result)
    }

    async fn image_list(&self) -> Result<Vec<ImageInfo>> {
        let resp: ResultWrapper<Vec<ImageInfo>> = self.get("kvm/image/ShowImageList").await?;
        Ok(resp.result)
    }

    async fn usb_list(&self, vm: &str) -> Result<Vec<UsbDevice>> {
        let (uuid, _) = resolve_vm(self, vm).await?;
        let resp: ResultWrapper<Vec<UsbDevice>> = self
            .get_with_params("kvm/usb/USBList", &[("vmName", uuid.as_str())])
            .await?;
        Ok(resp.result)
    }
}
