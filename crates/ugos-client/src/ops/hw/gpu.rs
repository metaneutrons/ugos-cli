//! Dedicated GPUs: listing them and moving one between the NAS and a VM.

use serde_json::{Value, json};

use super::{needs_reboot, reboot};
use crate::client::UgosClient;
use crate::error::{Result, UgosError};
use crate::types::hw::{Gpu, GpuList};

/// The dedicated GPUs.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn gpus(client: &UgosClient) -> Result<Vec<Gpu>> {
    let list: GpuList = client.get("hardware/gpu/list").await?;
    Ok(list.result)
}

/// Whether the passthrough disclaimer was accepted.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn disclaimer_accepted(client: &UgosClient) -> Result<bool> {
    let reply: Value = client.get("hardware/gpu/statement").await?;
    Ok(reply.get("agree").and_then(Value::as_bool).unwrap_or(false))
}

/// Where a GPU works.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuMode {
    /// For the NAS.
    Nas,
    /// For a VM, by VFIO passthrough.
    Vfio,
}

/// Switch a GPU to a mode.
///
/// The NAS restarts afterwards, as the web UI does itself, so this needs
/// `force`. For passthrough the web UI first makes the user accept a
/// disclaimer; `accept_disclaimer` is that acceptance.
///
/// # Errors
///
/// Returns [`UgosError::NeedsForce`] without `force`, and
/// [`UgosError::Refused`] for an unknown or abnormal GPU, or for passthrough
/// without the disclaimer accepted.
pub async fn set_mode(
    client: &UgosClient,
    gpu_id: &str,
    mode: GpuMode,
    accept_disclaimer: bool,
    force: bool,
) -> Result<()> {
    let all = gpus(client).await?;
    let gpu = all
        .iter()
        .find(|g| g.gpu_id == gpu_id || g.gpu_name == gpu_id)
        .ok_or_else(|| UgosError::NotFound {
            kind: "GPU",
            name: gpu_id.into(),
        })?;
    if gpu.gpu_state == 3 {
        return Err(UgosError::Refused(
            "the GPU is abnormal; its mode cannot be switched".into(),
        ));
    }
    let state = if mode == GpuMode::Vfio { 2 } else { 1 };
    if gpu.gpu_mode == state {
        return Err(UgosError::Refused("the GPU is in that mode already".into()));
    }
    if !force {
        return Err(needs_reboot("switching the mode of a GPU"));
    }
    if mode == GpuMode::Vfio && !disclaimer_accepted(client).await? {
        if !accept_disclaimer {
            return Err(UgosError::Refused(
                "passthrough needs the disclaimer accepted; read it in the web UI and accept it"
                    .into(),
            ));
        }
        let _: Value = client
            .post(
                "hardware/gpu/statement/agree",
                &json!([{ "gpu_id": gpu.gpu_id, "mode": 2 }]),
            )
            .await?;
    }
    let _: Value = client
        .post(
            "hardware/gpu/setmode",
            &json!([{ "gpu_id": gpu.gpu_id, "state": state }]),
        )
        .await?;
    reboot(client).await
}
