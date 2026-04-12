//! Command dispatch — maps CLI actions to API calls and output.

use anyhow::Result;
use ugos_client::UgosClient;
use ugos_client::api::docker::DockerApi;
use ugos_client::api::kvm::KvmApi;

use crate::cli::{
    DockerAction, ImageAction, LogAction, NetworkAction, OutputFormat, OvaAction, Resource,
    SnapshotAction, StorageAction, UsbAction, VmAction, VncAction,
};
use crate::output;

/// Dispatch a parsed CLI command.
///
/// # Errors
///
/// Returns an error on API or output failure.
pub async fn run(client: &UgosClient, resource: &Resource, fmt: OutputFormat) -> Result<()> {
    match resource {
        Resource::Vm { action } => vm(client, action, fmt).await,
        Resource::Network { action } => network(client, action, fmt).await,
        Resource::Storage { action } => storage(client, action, fmt).await,
        Resource::Image { action } => image(client, action, fmt).await,
        Resource::Usb { action } => usb(client, action, fmt).await,
        Resource::Vnc { action } => vnc(client, action, fmt).await,
        Resource::Ova { action } => ova(client, action, fmt).await,
        Resource::Docker { action } => docker(client, action, fmt).await,
        Resource::Log { action } => log(client, action, fmt).await,
        Resource::Info => info(client, fmt).await,
    }
}

async fn vm(client: &UgosClient, action: &VmAction, fmt: OutputFormat) -> Result<()> {
    match action {
        VmAction::List => {
            let vms = client.vm_list().await?;
            let rows: Vec<output::VmRow> = vms.iter().map(Into::into).collect();
            output::print_list(&rows, fmt)?;
        }
        VmAction::Show { name } => {
            let detail = client.vm_show(name).await?;
            match fmt {
                OutputFormat::Table => {
                    let rows = output::vm_detail_rows(&detail);
                    output::print_list(&rows, fmt)?;
                }
                OutputFormat::Json => {
                    #[allow(clippy::print_stdout)]
                    {
                        println!("{}", serde_json::to_string_pretty(&detail)?);
                    }
                }
            }
        }
        VmAction::Start { name } => {
            client.vm_start(name).await?;
            output::print_success(&format!("Started {name}"), fmt);
        }
        VmAction::Stop { name, force } => {
            client.vm_stop(name, *force).await?;
            output::print_success(&format!("Stopped {name}"), fmt);
        }
        VmAction::Reboot { name, force } => {
            client.vm_reboot(name, *force).await?;
            output::print_success(&format!("Rebooted {name}"), fmt);
        }
        VmAction::Delete { name } => {
            client.vm_delete(name).await?;
            output::print_success(&format!("Deleted {name}"), fmt);
        }
        VmAction::Create { file } => {
            let json = std::fs::read_to_string(file)
                .map_err(|e| anyhow::anyhow!("reading {file}: {e}"))?;
            let spec: ugos_client::types::kvm::VmDetail =
                serde_json::from_str(&json).map_err(|e| anyhow::anyhow!("parsing VM spec: {e}"))?;
            let uuid = client.vm_create(&spec).await?;
            output::print_success(&format!("Created VM {uuid}"), fmt);
        }
        VmAction::Update { file } => {
            let json = std::fs::read_to_string(file)
                .map_err(|e| anyhow::anyhow!("reading {file}: {e}"))?;
            let spec: ugos_client::types::kvm::VmDetail =
                serde_json::from_str(&json).map_err(|e| anyhow::anyhow!("parsing VM spec: {e}"))?;
            client.vm_update(&spec).await?;
            output::print_success(
                &format!("Updated VM {}", spec.virtual_machine_display_name),
                fmt,
            );
        }
        VmAction::Snapshot { action } => snapshot(client, action, fmt).await?,
    }
    Ok(())
}

async fn snapshot(client: &UgosClient, action: &SnapshotAction, fmt: OutputFormat) -> Result<()> {
    match action {
        SnapshotAction::List { vm } => {
            let snaps = client.snapshot_list(vm).await?;
            let rows: Vec<output::SnapshotRow> = snaps.iter().map(Into::into).collect();
            output::print_list(&rows, fmt)?;
        }
        SnapshotAction::Create { vm, name } => {
            client.snapshot_create(vm, name).await?;
            output::print_success(&format!("Created snapshot {name}"), fmt);
        }
        SnapshotAction::Delete { vm, name } => {
            client.snapshot_delete(vm, name).await?;
            output::print_success(&format!("Deleted snapshot {name}"), fmt);
        }
        SnapshotAction::Revert { vm, name } => {
            client.snapshot_revert(vm, name).await?;
            output::print_success(&format!("Reverted to snapshot {name}"), fmt);
        }
        SnapshotAction::Rename {
            vm,
            old_name,
            new_name,
        } => {
            client.snapshot_rename(vm, old_name, new_name).await?;
            output::print_success(&format!("Renamed snapshot {old_name} → {new_name}"), fmt);
        }
    }
    Ok(())
}

async fn network(client: &UgosClient, action: &NetworkAction, fmt: OutputFormat) -> Result<()> {
    match action {
        NetworkAction::List => {
            let nets = client.network_list().await?;
            let rows: Vec<output::NetworkRow> = nets.iter().map(Into::into).collect();
            output::print_list(&rows, fmt)?;
        }
        NetworkAction::Show { name } => {
            let detail = client.network_show(name).await?;
            match fmt {
                OutputFormat::Table => {
                    let rows = output::net_detail_rows(&detail);
                    output::print_list(&rows, fmt)?;
                }
                OutputFormat::Json => {
                    #[allow(clippy::print_stdout)]
                    {
                        println!("{}", serde_json::to_string_pretty(&detail)?);
                    }
                }
            }
        }
        NetworkAction::Delete { name } => {
            client.network_delete(name).await?;
            output::print_success(&format!("Deleted network {name}"), fmt);
        }
    }
    Ok(())
}

async fn storage(client: &UgosClient, action: &StorageAction, fmt: OutputFormat) -> Result<()> {
    match action {
        StorageAction::List => {
            let vols = client.storage_list().await?;
            let rows: Vec<output::StorageRow> = vols.iter().map(Into::into).collect();
            output::print_list(&rows, fmt)?;
        }
        StorageAction::Usage { name, uuid } => {
            let vms = client.storage_check_usage(name, uuid).await?;
            if vms.is_empty() {
                output::print_success("No VMs using this storage", fmt);
            } else {
                output::print_success(&format!("VMs using storage: {}", vms.join(", ")), fmt);
            }
        }
        StorageAction::Add { name, uuid } => {
            client.storage_add(name, uuid).await?;
            output::print_success(&format!("Added storage {name}"), fmt);
        }
        StorageAction::Delete { name, uuid } => {
            client.storage_delete(name, uuid).await?;
            output::print_success(&format!("Deleted storage {name}"), fmt);
        }
    }
    Ok(())
}

async fn image(client: &UgosClient, action: &ImageAction, fmt: OutputFormat) -> Result<()> {
    match action {
        ImageAction::List => {
            let imgs = client.image_list().await?;
            let rows: Vec<output::ImageRow> = imgs.iter().map(Into::into).collect();
            output::print_list(&rows, fmt)?;
        }
        ImageAction::Delete {
            file_name,
            image_name,
        } => {
            client.image_delete(file_name, image_name).await?;
            output::print_success(&format!("Deleted image {image_name}"), fmt);
        }
        ImageAction::Usage { name } => {
            let vms = client.image_check_usage(name).await?;
            if vms.is_empty() {
                output::print_success("No VMs using this image", fmt);
            } else {
                output::print_success(&format!("VMs using image: {}", vms.join(", ")), fmt);
            }
        }
    }
    Ok(())
}

async fn usb(client: &UgosClient, action: &UsbAction, fmt: OutputFormat) -> Result<()> {
    match action {
        UsbAction::List { vm } => {
            let devs = client.usb_list(vm).await?;
            let rows: Vec<output::UsbRow> = devs.iter().map(Into::into).collect();
            output::print_list(&rows, fmt)?;
        }
    }
    Ok(())
}

async fn vnc(client: &UgosClient, action: &VncAction, fmt: OutputFormat) -> Result<()> {
    match action {
        VncAction::List { vm } => {
            let links = client.vnc_list(vm).await?;
            let rows: Vec<output::VncRow> = links.iter().map(Into::into).collect();
            output::print_list(&rows, fmt)?;
        }
        VncAction::Generate { vm, source_url } => {
            let link = client.vnc_generate(vm, source_url).await?;
            output::print_success(&format!("VNC link: {link}"), fmt);
        }
    }
    Ok(())
}

async fn log(client: &UgosClient, action: &LogAction, fmt: OutputFormat) -> Result<()> {
    match action {
        LogAction::List { page, page_size } => {
            let result = client.log_search(*page, *page_size).await?;
            let rows: Vec<output::LogRow> = result.list.iter().map(Into::into).collect();
            output::print_list(&rows, fmt)?;
        }
        LogAction::Operators => {
            let ops = client.log_operators().await?;
            output::print_success(&format!("Operators: {}", ops.join(", ")), fmt);
        }
    }
    Ok(())
}

async fn ova(client: &UgosClient, action: &OvaAction, fmt: OutputFormat) -> Result<()> {
    match action {
        OvaAction::Export {
            vm,
            storage_name,
            storage_uuid,
            ova_path,
        } => {
            client
                .ova_export(vm, storage_name, storage_uuid, ova_path)
                .await?;
            output::print_success(&format!("Exported {vm} to {ova_path}"), fmt);
        }
        OvaAction::Parse { ova_path } => {
            let detail = client.ova_parse(ova_path).await?;
            match fmt {
                OutputFormat::Table => {
                    let rows = output::vm_detail_rows(&detail);
                    output::print_list(&rows, fmt)?;
                }
                OutputFormat::Json => {
                    #[allow(clippy::print_stdout)]
                    {
                        println!("{}", serde_json::to_string_pretty(&detail)?);
                    }
                }
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
async fn docker(client: &UgosClient, action: &DockerAction, fmt: OutputFormat) -> Result<()> {
    match action {
        DockerAction::Overview => {
            let ov = client.docker_overview().await?;
            match fmt {
                OutputFormat::Table => {
                    let rows = vec![
                        output::HostInfoRow {
                            field: "Containers".into(),
                            value: format!(
                                "{} ({} running)",
                                ov.container_count, ov.run_container_count
                            ),
                        },
                        output::HostInfoRow {
                            field: "Images".into(),
                            value: ov.image_count.to_string(),
                        },
                        output::HostInfoRow {
                            field: "CPU".into(),
                            value: format!(
                                "{}% (containers: {}%)",
                                ov.cpu_used, ov.container_cpu_used
                            ),
                        },
                        output::HostInfoRow {
                            field: "Memory".into(),
                            value: format!(
                                "{} / {}",
                                output::format_gib(ov.memory_used),
                                output::format_gib(ov.total_memory)
                            ),
                        },
                        output::HostInfoRow {
                            field: "Engine".into(),
                            value: if ov.status { "running" } else { "stopped" }.into(),
                        },
                    ];
                    output::print_list(&rows, fmt)?;
                }
                OutputFormat::Json => {
                    #[allow(clippy::print_stdout)]
                    {
                        println!("{}", serde_json::to_string_pretty(&ov)?);
                    }
                }
            }
        }
        DockerAction::Status => {
            let status = client.docker_engine_status().await?;
            output::print_success(&format!("Docker engine: {status}"), fmt);
        }
        DockerAction::Ps { page, page_size } => {
            let result = client.container_list(*page, *page_size).await?;
            let containers = result.result.unwrap_or_default();
            let rows: Vec<output::ContainerRow> = containers.iter().map(Into::into).collect();
            output::print_list(&rows, fmt)?;
        }
        DockerAction::Start { id } => {
            client.container_start(id).await?;
            output::print_success(&format!("Started {id}"), fmt);
        }
        DockerAction::Stop { id } => {
            client.container_stop(id).await?;
            output::print_success(&format!("Stopped {id}"), fmt);
        }
        DockerAction::Restart { id } => {
            client.container_restart(id).await?;
            output::print_success(&format!("Restarted {id}"), fmt);
        }
        DockerAction::Kill { id } => {
            client.container_kill(id).await?;
            output::print_success(&format!("Killed {id}"), fmt);
        }
        DockerAction::Rm { id } => {
            client.container_remove(id).await?;
            output::print_success(&format!("Removed {id}"), fmt);
        }
        DockerAction::Images { page, page_size } => {
            let result = client.docker_image_list(*page, *page_size).await?;
            let images = result.result.unwrap_or_default();
            let rows: Vec<output::DockerImageRow> = images.iter().map(Into::into).collect();
            output::print_list(&rows, fmt)?;
        }
        DockerAction::Search { name } => {
            let images = client.docker_image_search(name, 1, 20).await?;
            let rows: Vec<output::DockerImageRow> = images.iter().map(Into::into).collect();
            output::print_list(&rows, fmt)?;
        }
        DockerAction::Pull { image, tag } => {
            client.docker_image_download(image, tag).await?;
            output::print_success(&format!("Pulling {image}:{tag}"), fmt);
        }
        DockerAction::Rmi { id } => {
            client.docker_image_delete(id).await?;
            output::print_success(&format!("Deleted image {id}"), fmt);
        }
        DockerAction::Mirrors => {
            let mirrors = client.mirror_list().await?;
            let rows: Vec<output::MirrorRow> = mirrors.iter().map(Into::into).collect();
            output::print_list(&rows, fmt)?;
        }
    }
    Ok(())
}

async fn info(client: &UgosClient, fmt: OutputFormat) -> Result<()> {
    let host = client.host_info().await?;
    match fmt {
        OutputFormat::Table => {
            let rows = output::host_info_rows(&host);
            output::print_list(&rows, fmt)?;
        }
        OutputFormat::Json => {
            #[allow(clippy::print_stdout)]
            {
                println!("{}", serde_json::to_string_pretty(&host)?);
            }
        }
    }
    Ok(())
}
