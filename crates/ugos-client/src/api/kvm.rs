//! KVM virtual machine management API.
//!
//! All methods accept display names or UUIDs. Display names are resolved
//! to UUIDs internally via [`KvmApi::vm_list`].

use crate::client::UgosClient;
use crate::error::{Result, UgosError};
use crate::types::common::ResultWrapper;
use crate::types::kvm::{
    HostInfo, ImageInfo, LogPage, NetworkDetail, NetworkSummary, Overview, Snapshot, StorageInfo,
    StorageUsage, UsbDevice, VmDetail, VmSummary, VncLink,
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
    /// Create a new VM from a `VmDetail` spec.
    ///
    /// `storage_uuid` is filled in from `storage_name` when empty. The UUID in
    /// `virtual_machine_name` is ignored by UGOS, which assigns its own; the
    /// assigned one is returned, or an empty string if the new VM cannot be
    /// found in the listing afterwards.
    fn vm_create(&self, spec: &VmDetail) -> impl Future<Output = Result<String>> + Send;
    /// Update an existing VM (must be shut off).
    fn vm_update(&self, spec: &VmDetail) -> impl Future<Output = Result<()>> + Send;
    /// Get host hardware info (CPU cores, memory).
    fn host_info(&self) -> impl Future<Output = Result<HostInfo>> + Send;
    /// Get host load and every VM in one call.
    fn overview(&self) -> impl Future<Output = Result<Overview>> + Send;
    /// Check whether a VM display name is already taken.
    fn vm_name_taken(&self, display_name: &str) -> impl Future<Output = Result<bool>> + Send;
    /// Check whether the host can back a given amount of guest memory.
    fn check_memory(&self, bytes: i64) -> impl Future<Output = Result<MemoryStatus>> + Send;

    // ── Snapshot ────────────────────────────────────────────────────

    /// List snapshots for a VM.
    fn snapshot_list(&self, vm: &str) -> impl Future<Output = Result<Vec<Snapshot>>> + Send;
    /// Create a snapshot. UGOS names it and returns that name.
    fn snapshot_create(&self, vm: &str) -> impl Future<Output = Result<String>> + Send;
    /// Delete a snapshot by its internal name.
    fn snapshot_delete(&self, vm: &str, name: &str) -> impl Future<Output = Result<()>> + Send;
    /// Revert to a snapshot, optionally taking one of the current state first.
    fn snapshot_revert(
        &self,
        vm: &str,
        name: &str,
        snapshot_first: bool,
    ) -> impl Future<Output = Result<()>> + Send;
    /// Set a snapshot's description.
    fn snapshot_describe(
        &self,
        vm: &str,
        name: &str,
        description: &str,
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
    /// Check whether a network name is already taken.
    fn network_name_taken(&self, name: &str) -> impl Future<Output = Result<bool>> + Send;
    /// List the VMs attached to a network.
    fn network_check_usage(&self, name: &str) -> impl Future<Output = Result<Vec<String>>> + Send;

    // ── Storage ─────────────────────────────────────────────────────

    /// List storage volumes available to KVM, including how many VMs each holds.
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
    /// List volume usage, broken down per VM.
    fn storage_usage_list(&self) -> impl Future<Output = Result<Vec<StorageUsage>>> + Send;

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

    /// Upload an ISO image, sending it in chunks.
    ///
    /// `progress` is called after each chunk with `(finished, total)`. Returns
    /// the file name the image was stored under.
    fn image_upload(
        &self,
        path: &std::path::Path,
        iso_name: &str,
        progress: &(dyn Fn(usize, usize) + Send + Sync),
    ) -> impl Future<Output = Result<String>> + Send;

    // `kvm/image/RenameImage` exists but is deliberately not wrapped: it
    // answers `successful` to every body tried so far and renames nothing.
    // The web UI never calls it either, so the field names are unknown.

    /// Download an ISO from a URL into a temporary file and upload it.
    ///
    /// `progress` reports both phases: while downloading it is called with
    /// `(bytes so far, 0)`, while uploading with `(finished chunk, total
    /// chunks)`. A total of `0` therefore marks the download phase.
    fn image_upload_url(
        &self,
        url: &str,
        iso_name: &str,
        progress: &(dyn Fn(usize, usize) + Send + Sync),
    ) -> impl Future<Output = Result<String>> + Send;

    /// Register an image that already sits on the NAS, without uploading it.
    fn image_register(
        &self,
        path: &str,
        image_name: &str,
        file_name: &str,
    ) -> impl Future<Output = Result<()>> + Send;

    // ── USB / PCI ───────────────────────────────────────────────────

    /// List USB devices for a VM.
    fn usb_list(&self, vm: &str) -> impl Future<Output = Result<Vec<UsbDevice>>> + Send;
    /// List PCI devices available for passthrough.
    fn passthrough_devices(&self) -> impl Future<Output = Result<Vec<serde_json::Value>>> + Send;

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

    // ── OVA ─────────────────────────────────────────────────────────

    /// Export a VM as an OVA file.
    fn ova_export(
        &self,
        vm: &str,
        storage_name: &str,
        storage_uuid: &str,
        ova_path: &str,
    ) -> impl Future<Output = Result<()>> + Send;

    /// Parse an OVA file and return the VM configuration it contains.
    fn ova_parse(&self, ova_path: &str) -> impl Future<Output = Result<VmDetail>> + Send;
}

/// How a requested amount of guest memory relates to the host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryStatus {
    /// The host can back it comfortably.
    Fits,
    /// Above what is currently free; UGOS warns but allows it.
    Tight,
    /// More than the host has at all.
    TooMuch,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct MemoryCheck {
    #[serde(default)]
    memory_status: i64,
}

/// Turn a failed create into something a human can act on.
///
/// UGOS answers `3000, Fail to create virtual machine` whatever the cause, so
/// the validators the web UI uses on its form are asked afterwards.
async fn explain_create_failure(
    client: &UgosClient,
    spec: &VmDetail,
    original: UgosError,
) -> UgosError {
    let name = &spec.virtual_machine_display_name;
    if client.vm_name_taken(name).await.unwrap_or(false) {
        return UgosError::OperationFailed(format!("a VM named '{name}' already exists"));
    }
    // memory is KiB in the spec, bytes in the check.
    if matches!(
        client.check_memory(spec.memory.value * 1024).await,
        Ok(MemoryStatus::TooMuch)
    ) {
        return UgosError::OperationFailed(format!(
            "{} MiB of memory is more than this NAS has",
            spec.memory.value / 1024
        ));
    }
    original
}

/// Explain a refused power-on where the code alone says nothing.
async fn explain_start_failure(client: &UgosClient, vm: &str, original: UgosError) -> UgosError {
    let Ok(detail) = client.vm_show(vm).await else {
        return original;
    };
    match client.check_memory(detail.memory.value * 1024).await {
        Ok(MemoryStatus::TooMuch) => UgosError::OperationFailed(format!(
            "{} needs {} MiB, more than this NAS has",
            vm,
            detail.memory.value / 1024
        )),
        Ok(MemoryStatus::Tight) => UgosError::OperationFailed(format!(
            "{original}. {} needs {} MiB, more than is free right now",
            vm,
            detail.memory.value / 1024
        )),
        _ => original,
    }
}

/// Chunk size the web UI uses for image uploads.
const CHUNK_SIZE: u64 = 10 * 1024 * 1024;

/// Find a free image file name, appending `-2`, `-3`, … when taken.
async fn unique_file_name(client: &UgosClient, wanted: &str) -> Result<String> {
    let images: ResultWrapper<Vec<ImageInfo>> = client.get("kvm/image/ShowImageList").await?;
    let taken: Vec<&str> = images.result.iter().map(|i| i.file_name.as_str()).collect();
    if !taken.contains(&wanted) {
        return Ok(wanted.to_owned());
    }
    let (stem, ext) = wanted.rsplit_once('.').unwrap_or((wanted, ""));
    for n in 2..1000 {
        let candidate = if ext.is_empty() {
            format!("{stem}-{n}")
        } else {
            format!("{stem}-{n}.{ext}")
        };
        if !taken.contains(&candidate.as_str()) {
            return Ok(candidate);
        }
    }
    Err(UgosError::Encryption(format!(
        "no free file name for '{wanted}'"
    )))
}

fn ensure_nonempty(size: u64) -> Result<()> {
    if size == 0 {
        return Err(UgosError::Encryption("file is empty".to_owned()));
    }
    Ok(())
}

/// Response of `GenerateSnapshot`, which names the snapshot itself.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotCreated {
    #[serde(default)]
    snapshot_display_name: String,
}

/// Look up a snapshot's display name, which the delete endpoint wants
/// alongside the internal name.
async fn snapshot_display_name(client: &UgosClient, vm: &str, name: &str) -> Result<String> {
    let (uuid, _) = resolve_vm(client, vm).await?;
    let snaps: ResultWrapper<Vec<Snapshot>> = client
        .get_with_params("kvm/manager/ShowListSnapshot", &[("name", uuid.as_str())])
        .await?;
    snaps
        .result
        .iter()
        .find(|s| s.name == name || s.display_name == name)
        .map(|s| s.display_name.clone())
        .ok_or_else(|| UgosError::NotFound {
            kind: "snapshot",
            name: name.to_owned(),
        })
}

// ── Name resolution ─────────────────────────────────────────────────

/// Fill in `storage_uuid` from `storage_name` when the caller left it empty.
///
/// `CreateVirtualMachine` rejects a body without `storageUUID`, and the name
/// is the only part a human can reasonably supply.
async fn resolve_storage_uuid(client: &UgosClient, spec: &mut VmDetail) -> Result<()> {
    if !spec.storage_uuid.is_empty() {
        return Ok(());
    }
    let volumes: ResultWrapper<Vec<StorageInfo>> =
        client.get("kvm/storage/ShowStorageList").await?;
    let volume = volumes
        .result
        .iter()
        .find(|v| v.name == spec.storage_name || v.label == spec.storage_name)
        .ok_or_else(|| UgosError::NotFound {
            kind: "storage volume",
            name: spec.storage_name.clone(),
        })?;
    spec.storage_uuid.clone_from(&volume.uuid);
    volume.name.clone_into(&mut spec.storage_name);
    Ok(())
}

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
        let started: Result<ResultWrapper<String>> = self
            .get_with_params(
                "kvm/manager/PowerOn",
                &[
                    ("name", uuid.as_str()),
                    ("virtualMachineDisplayName", display.as_str()),
                ],
            )
            .await;
        if let Err(e) = started {
            return Err(explain_start_failure(self, name, e).await);
        }
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

    async fn vm_create(&self, spec: &VmDetail) -> Result<String> {
        let mut spec = spec.clone();
        resolve_storage_uuid(self, &mut spec).await?;
        let created: Result<ResultWrapper<String>> =
            self.post("kvm/manager/CreateVirtualMachine", &spec).await;
        if let Err(e) = created {
            return Err(explain_create_failure(self, &spec, e).await);
        }

        // UGOS assigns the UUID itself and ignores whatever the body carried,
        // so the only way to learn it is to look the VM up again.
        let vms: ResultWrapper<Vec<VmSummary>> =
            self.get("kvm/manager/ShowLocalVirtualList").await?;
        Ok(vms
            .result
            .iter()
            .filter(|v| v.vir_display_name == spec.virtual_machine_display_name)
            .max_by_key(|v| v.create_time)
            .map(|v| v.vir_name.clone())
            .unwrap_or_default())
    }

    async fn vm_update(&self, spec: &VmDetail) -> Result<()> {
        let mut spec = spec.clone();
        resolve_storage_uuid(self, &mut spec).await?;
        let _: ResultWrapper<String> = self.post("kvm/manager/UpdateVirtualMachine", &spec).await?;
        Ok(())
    }

    async fn host_info(&self) -> Result<HostInfo> {
        self.get("kvm/manager/ShowNativeInfo").await
    }

    async fn overview(&self) -> Result<Overview> {
        self.get("kvm/manager/ShowOverview").await
    }

    async fn vm_name_taken(&self, display_name: &str) -> Result<bool> {
        // `result: true` means the name is in use, not that it is free.
        let body = serde_json::json!({"name": "", "virtualMachineDisplayName": display_name});
        let resp: ResultWrapper<bool> = self.post("kvm/manager/CheckVirName", &body).await?;
        Ok(resp.result)
    }

    async fn check_memory(&self, bytes: i64) -> Result<MemoryStatus> {
        let resp: MemoryCheck = self
            .get_with_params(
                "kvm/manager/CheckResource",
                &[("memory", &bytes.to_string())],
            )
            .await?;
        Ok(match resp.memory_status {
            0 => MemoryStatus::Fits,
            1 => MemoryStatus::Tight,
            _ => MemoryStatus::TooMuch,
        })
    }

    // ── Snapshot ────────────────────────────────────────────────────

    async fn snapshot_list(&self, vm: &str) -> Result<Vec<Snapshot>> {
        let (uuid, _) = resolve_vm(self, vm).await?;
        let resp: ResultWrapper<Vec<Snapshot>> = self
            .get_with_params("kvm/manager/ShowListSnapshot", &[("name", uuid.as_str())])
            .await?;
        Ok(resp.result)
    }

    async fn snapshot_create(&self, vm: &str) -> Result<String> {
        // `name` is the VM's UUID here, not a snapshot name: UGOS picks the
        // snapshot name itself and reports it back.
        let (uuid, display) = resolve_vm(self, vm).await?;
        let resp: SnapshotCreated = self
            .get_with_params(
                "kvm/manager/GenerateSnapshot",
                &[
                    ("name", uuid.as_str()),
                    ("virtualMachineDisplayName", display.as_str()),
                ],
            )
            .await?;
        Ok(resp.snapshot_display_name)
    }

    async fn snapshot_delete(&self, vm: &str, name: &str) -> Result<()> {
        // Both parameters describe the snapshot; the display name comes from
        // the listing.
        let display = snapshot_display_name(self, vm, name).await?;
        let _: ResultWrapper<String> = self
            .get_with_params(
                "kvm/manager/DeleteSnapshot",
                &[
                    ("name", name),
                    ("virtualMachineDisplayName", display.as_str()),
                ],
            )
            .await?;
        Ok(())
    }

    async fn snapshot_revert(&self, vm: &str, name: &str, snapshot_first: bool) -> Result<()> {
        let (uuid, _) = resolve_vm(self, vm).await?;
        let _: ResultWrapper<String> = self
            .get_with_params(
                "kvm/manager/RevertSnapshot",
                &[
                    ("virtualMachineName", uuid.as_str()),
                    ("snapshotName", name),
                    (
                        "createSnapshot",
                        if snapshot_first { "true" } else { "false" },
                    ),
                ],
            )
            .await?;
        Ok(())
    }

    async fn snapshot_describe(&self, vm: &str, name: &str, description: &str) -> Result<()> {
        let _ = snapshot_display_name(self, vm, name).await?;
        let body = serde_json::json!({"name": name, "description": description});
        let _: ResultWrapper<String> = self.post("kvm/manager/EditSnapshot", &body).await?;
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
        let created: Result<ResultWrapper<String>> =
            self.post("kvm/network/CreateNetwork", network).await;
        if let Err(e) = created {
            if self
                .network_name_taken(&network.network_name)
                .await
                .unwrap_or(false)
            {
                return Err(UgosError::OperationFailed(format!(
                    "a network named '{}' already exists",
                    network.network_name
                )));
            }
            return Err(e);
        }
        Ok(())
    }

    async fn network_update(&self, network: &NetworkDetail) -> Result<()> {
        let _: ResultWrapper<String> = self.post("kvm/network/UpdateNetwork", network).await?;
        Ok(())
    }

    async fn network_name_taken(&self, name: &str) -> Result<bool> {
        let body = serde_json::json!({"name": name});
        let resp: ResultWrapper<bool> = self.post("kvm/network/CheckName", &body).await?;
        Ok(resp.result)
    }

    async fn network_check_usage(&self, name: &str) -> Result<Vec<String>> {
        let resp: ResultWrapper<Vec<String>> = self
            .get_with_params("kvm/network/CheckNetwork", &[("name", name)])
            .await?;
        Ok(resp.result)
    }

    async fn network_delete(&self, name: &str) -> Result<()> {
        let deleted: Result<ResultWrapper<String>> = self
            .get_with_params("kvm/network/DeleteNetwork", &[("name", name)])
            .await;
        if let Err(e) = deleted {
            if let Ok(vms) = self.network_check_usage(name).await
                && !vms.is_empty()
            {
                return Err(UgosError::OperationFailed(format!(
                    "network '{name}' is still attached to: {}",
                    vms.join(", ")
                )));
            }
            return Err(e);
        }
        Ok(())
    }

    // ── Storage ─────────────────────────────────────────────────────

    async fn storage_list(&self) -> Result<Vec<StorageInfo>> {
        // ShowLocalStorageList over ShowStorageList: same volumes and fields,
        // plus virCount.
        let resp: ResultWrapper<Vec<StorageInfo>> =
            self.get("kvm/storage/ShowLocalStorageList").await?;
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

    async fn storage_usage_list(&self) -> Result<Vec<StorageUsage>> {
        let resp: ResultWrapper<Vec<StorageUsage>> =
            self.get("kvm/storage/ShowLocalStorageUsageList").await?;
        Ok(resp.result)
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
                &[("fileName", file_name), ("imageName", image_name)],
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

    async fn image_upload(
        &self,
        path: &std::path::Path,
        iso_name: &str,
        progress: &(dyn Fn(usize, usize) + Send + Sync),
    ) -> Result<String> {
        use std::io::Read;

        let wanted = path.file_name().map_or_else(
            || format!("{iso_name}.iso"),
            |n| n.to_string_lossy().into_owned(),
        );
        // UGOS answers 9999 when the file name is taken, which is why the web
        // UI uses random names. A suffix keeps the name readable instead.
        let file_name = unique_file_name(self, &wanted).await?;

        let mut file = std::fs::File::open(path)
            .map_err(|e| UgosError::Encryption(format!("opening '{}': {e}", path.display())))?;
        let size = file
            .metadata()
            .map_err(|e| UgosError::Encryption(format!("reading file size: {e}")))?
            .len();
        ensure_nonempty(size)?;

        // The web UI splits uploads into 10 MiB parts and numbers them from 0.
        let total = usize::try_from(size.div_ceil(CHUNK_SIZE))
            .map_err(|_| UgosError::Encryption("file too large to chunk".to_owned()))?;
        let chunk_len = usize::try_from(CHUNK_SIZE)
            .map_err(|_| UgosError::Encryption("chunk size unsupported here".to_owned()))?;
        let mut buf = vec![0u8; chunk_len];

        for index in 0..total {
            let mut filled = 0;
            while filled < buf.len() {
                let n = file
                    .read(&mut buf[filled..])
                    .map_err(|e| UgosError::Encryption(format!("reading chunk {index}: {e}")))?;
                if n == 0 {
                    break;
                }
                filled += n;
            }

            let part = reqwest::multipart::Part::bytes(buf[..filled].to_vec())
                .file_name("blob")
                .mime_str("application/octet-stream")
                .map_err(|e| UgosError::Encryption(format!("building chunk: {e}")))?;
            let form = reqwest::multipart::Form::new()
                .text("isoName", iso_name.to_owned())
                .text("fileName", file_name.clone())
                .text("size", size.to_string())
                .text("chunks", total.to_string())
                .text("chunk", index.to_string())
                .part("file", part);

            let _: ResultWrapper<String> = self.post_multipart("kvm/image/UploadUpk", form).await?;
            progress(index + 1, total);
        }

        Ok(file_name)
    }

    async fn image_upload_url(
        &self,
        url: &str,
        iso_name: &str,
        progress: &(dyn Fn(usize, usize) + Send + Sync),
    ) -> Result<String> {
        use std::io::Write;

        let file_name = url
            .rsplit('/')
            .next()
            .filter(|n| !n.is_empty())
            .map_or_else(|| format!("{iso_name}.iso"), ToOwned::to_owned);
        let temp = std::env::temp_dir().join(&file_name);

        // A separate client: the authenticated one carries NAS cookies and
        // trusts invalid certificates, neither of which belongs on a download
        // from an arbitrary host.
        let plain = reqwest::Client::builder()
            .build()
            .map_err(|e| UgosError::Encryption(format!("HTTP client build: {e}")))?;
        let mut resp = plain.get(url).send().await?.error_for_status()?;

        let mut out = std::fs::File::create(&temp)
            .map_err(|e| UgosError::Encryption(format!("creating '{}': {e}", temp.display())))?;
        let mut downloaded = 0usize;
        while let Some(chunk) = resp.chunk().await? {
            out.write_all(&chunk)
                .map_err(|e| UgosError::Encryption(format!("writing download: {e}")))?;
            downloaded += chunk.len();
            progress(downloaded, 0);
        }
        drop(out);

        let result = self.image_upload(&temp, iso_name, progress).await;
        let _ = std::fs::remove_file(&temp);
        result
    }

    async fn image_register(&self, path: &str, image_name: &str, file_name: &str) -> Result<()> {
        let body = serde_json::json!({
            "path": path,
            "imageName": image_name,
            "fileName": file_name,
        });
        let _: ResultWrapper<String> = self.post("kvm/image/UploadPath", &body).await?;
        Ok(())
    }

    // ── USB / PCI ───────────────────────────────────────────────────

    async fn passthrough_devices(&self) -> Result<Vec<serde_json::Value>> {
        let resp: ResultWrapper<Vec<serde_json::Value>> =
            self.get("kvm/passthrough/devices").await?;
        Ok(resp.result)
    }

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

    // ── OVA ─────────────────────────────────────────────────────────

    async fn ova_export(
        &self,
        vm: &str,
        storage_name: &str,
        storage_uuid: &str,
        ova_path: &str,
    ) -> Result<()> {
        let (uuid, _) = resolve_vm(self, vm).await?;
        let body = serde_json::json!({
            "virtualName": uuid,
            "storageName": storage_name,
            "storageUUID": storage_uuid,
            "ovaPath": ova_path,
        });
        let _: ResultWrapper<String> = self.post("kvm/manager/ExportOVA", &body).await?;
        Ok(())
    }

    async fn ova_parse(&self, ova_path: &str) -> Result<VmDetail> {
        let body = serde_json::json!({"ovaPath": ova_path});
        self.post("kvm/manager/ParseOvaFile", &body).await
    }
}
