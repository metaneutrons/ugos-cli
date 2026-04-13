//! Command dispatch — maps CLI actions to API calls and output.

use std::io::Write;

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
pub async fn run(
    client: &UgosClient,
    resource: &Resource,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    match resource {
        Resource::Vm { action } => vm(client, action, fmt, w).await,
        Resource::Network { action } => network(client, action, fmt, w).await,
        Resource::Storage { action } => storage(client, action, fmt, w).await,
        Resource::Image { action } => image(client, action, fmt, w).await,
        Resource::Usb { action } => usb(client, action, fmt, w).await,
        Resource::Vnc { action } => vnc(client, action, fmt, w).await,
        Resource::Ova { action } => ova(client, action, fmt, w).await,
        Resource::Docker { action } => docker(client, action, fmt, w).await,
        Resource::Log { action } => log(client, action, fmt, w).await,
        Resource::Info => info(client, fmt, w).await,
    }
}

async fn vm(
    client: &UgosClient,
    action: &VmAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    match action {
        VmAction::List => {
            let vms = client.vm_list().await?;
            let rows: Vec<output::VmRow> = vms.iter().map(Into::into).collect();
            output::print_list(w, &rows, fmt)?;
        }
        VmAction::Show { name } => {
            let detail = client.vm_show(name).await?;
            match fmt {
                OutputFormat::Table => {
                    let rows = output::vm_detail_rows(&detail);
                    output::print_list(w, &rows, fmt)?;
                }
                OutputFormat::Json => {
                    output::print_json(w, &detail)?;
                }
            }
        }
        VmAction::Start { name } => {
            client.vm_start(name).await?;
            output::print_success(w, &format!("Started {name}"), fmt)?;
        }
        VmAction::Stop { name, force } => {
            client.vm_stop(name, *force).await?;
            output::print_success(w, &format!("Stopped {name}"), fmt)?;
        }
        VmAction::Reboot { name, force } => {
            client.vm_reboot(name, *force).await?;
            output::print_success(w, &format!("Rebooted {name}"), fmt)?;
        }
        VmAction::Delete { name } => {
            client.vm_delete(name).await?;
            output::print_success(w, &format!("Deleted {name}"), fmt)?;
        }
        VmAction::Create {
            name,
            os,
            cores,
            memory,
            disk,
            iso,
            network,
            boot_type,
            storage,
            autostart,
        } => {
            let spec = build_vm_spec(
                name,
                os,
                *cores,
                *memory,
                *disk,
                iso.as_ref(),
                network,
                boot_type,
                storage,
                *autostart,
            )?;
            let uuid = client.vm_create(&spec).await?;
            output::print_success(w, &format!("Created VM {name} ({uuid})"), fmt)?;
        }
        VmAction::Update {
            name,
            cores,
            memory,
            autostart,
            boot_type,
        } => {
            let mut spec = client.vm_show(name).await?;
            if let Some(c) = cores {
                spec.core.value = *c;
            }
            if let Some(m) = memory {
                spec.memory.value = *m * 1024; // MiB → KiB
            }
            if let Some(a) = autostart {
                spec.other_config.auto_matic_start_up = *a;
            }
            if let Some(bt) = boot_type {
                spec.device.boot_type = bt.clone();
            }
            client.vm_update(&spec).await?;
            output::print_success(
                w,
                &format!("Updated VM {}", spec.virtual_machine_display_name),
                fmt,
            )?;
        }
        VmAction::Snapshot { action } => snapshot(client, action, fmt, w).await?,
    }
    Ok(())
}

async fn snapshot(
    client: &UgosClient,
    action: &SnapshotAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    match action {
        SnapshotAction::List { vm } => {
            let snaps = client.snapshot_list(vm).await?;
            let rows: Vec<output::SnapshotRow> = snaps.iter().map(Into::into).collect();
            output::print_list(w, &rows, fmt)?;
        }
        SnapshotAction::Create { vm, name } => {
            client.snapshot_create(vm, name).await?;
            output::print_success(w, &format!("Created snapshot {name}"), fmt)?;
        }
        SnapshotAction::Delete { vm, name } => {
            client.snapshot_delete(vm, name).await?;
            output::print_success(w, &format!("Deleted snapshot {name}"), fmt)?;
        }
        SnapshotAction::Revert { vm, name } => {
            client.snapshot_revert(vm, name).await?;
            output::print_success(w, &format!("Reverted to snapshot {name}"), fmt)?;
        }
        SnapshotAction::Rename {
            vm,
            old_name,
            new_name,
        } => {
            client.snapshot_rename(vm, old_name, new_name).await?;
            output::print_success(w, &format!("Renamed snapshot {old_name} → {new_name}"), fmt)?;
        }
    }
    Ok(())
}

async fn network(
    client: &UgosClient,
    action: &NetworkAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    match action {
        NetworkAction::List => {
            let nets = client.network_list().await?;
            let rows: Vec<output::NetworkRow> = nets.iter().map(Into::into).collect();
            output::print_list(w, &rows, fmt)?;
        }
        NetworkAction::Show { name } => {
            let detail = client.network_show(name).await?;
            match fmt {
                OutputFormat::Table => {
                    let rows = output::net_detail_rows(&detail);
                    output::print_list(w, &rows, fmt)?;
                }
                OutputFormat::Json => {
                    output::print_json(w, &detail)?;
                }
            }
        }
        NetworkAction::Create {
            name,
            net_type,
            interface,
        } => {
            let net = ugos_client::types::kvm::NetworkDetail {
                network_uuid: String::new(),
                network_name: name.clone(),
                network_type: net_type.clone(),
                network_mode: net_type.clone(),
                mapping_network: interface.clone(),
                ..Default::default()
            };
            client.network_create(&net).await?;
            output::print_success(w, &format!("Created network {name}"), fmt)?;
        }
        NetworkAction::Update { name, interface } => {
            let mut net = client.network_show(name).await?;
            if let Some(iface) = interface {
                net.mapping_network = iface.clone();
            }
            client.network_update(&net).await?;
            output::print_success(w, &format!("Updated network {name}"), fmt)?;
        }
        NetworkAction::Delete { name } => {
            client.network_delete(name).await?;
            output::print_success(w, &format!("Deleted network {name}"), fmt)?;
        }
    }
    Ok(())
}

async fn storage(
    client: &UgosClient,
    action: &StorageAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    match action {
        StorageAction::List => {
            let vols = client.storage_list().await?;
            let rows: Vec<output::StorageRow> = vols.iter().map(Into::into).collect();
            output::print_list(w, &rows, fmt)?;
        }
        StorageAction::Usage { name, uuid } => {
            let vms = client.storage_check_usage(name, uuid).await?;
            if vms.is_empty() {
                output::print_success(w, "No VMs using this storage", fmt)?;
            } else {
                output::print_success(w, &format!("VMs using storage: {}", vms.join(", ")), fmt)?;
            }
        }
        StorageAction::Add { name, uuid } => {
            client.storage_add(name, uuid).await?;
            output::print_success(w, &format!("Added storage {name}"), fmt)?;
        }
        StorageAction::Delete { name, uuid } => {
            client.storage_delete(name, uuid).await?;
            output::print_success(w, &format!("Deleted storage {name}"), fmt)?;
        }
    }
    Ok(())
}

async fn image(
    client: &UgosClient,
    action: &ImageAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    match action {
        ImageAction::List => {
            let imgs = client.image_list().await?;
            let rows: Vec<output::ImageRow> = imgs.iter().map(Into::into).collect();
            output::print_list(w, &rows, fmt)?;
        }
        ImageAction::Delete {
            file_name,
            image_name,
        } => {
            client.image_delete(file_name, image_name).await?;
            output::print_success(w, &format!("Deleted image {image_name}"), fmt)?;
        }
        ImageAction::Usage { name } => {
            let vms = client.image_check_usage(name).await?;
            if vms.is_empty() {
                output::print_success(w, "No VMs using this image", fmt)?;
            } else {
                output::print_success(w, &format!("VMs using image: {}", vms.join(", ")), fmt)?;
            }
        }
    }
    Ok(())
}

async fn usb(
    client: &UgosClient,
    action: &UsbAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    match action {
        UsbAction::List { vm } => {
            let devs = client.usb_list(vm).await?;
            let rows: Vec<output::UsbRow> = devs.iter().map(Into::into).collect();
            output::print_list(w, &rows, fmt)?;
        }
    }
    Ok(())
}

async fn vnc(
    client: &UgosClient,
    action: &VncAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    match action {
        VncAction::List { vm } => {
            let links = client.vnc_list(vm).await?;
            let rows: Vec<output::VncRow> = links.iter().map(Into::into).collect();
            output::print_list(w, &rows, fmt)?;
        }
        VncAction::Generate { vm, source_url } => {
            let link = client.vnc_generate(vm, source_url).await?;
            output::print_success(w, &format!("VNC link: {link}"), fmt)?;
        }
    }
    Ok(())
}

async fn log(
    client: &UgosClient,
    action: &LogAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    match action {
        LogAction::List { page, page_size } => {
            let result = client.log_search(*page, *page_size).await?;
            let rows: Vec<output::LogRow> = result.list.iter().map(Into::into).collect();
            output::print_list(w, &rows, fmt)?;
        }
        LogAction::Operators => {
            let ops = client.log_operators().await?;
            output::print_success(w, &format!("Operators: {}", ops.join(", ")), fmt)?;
        }
    }
    Ok(())
}

async fn ova(
    client: &UgosClient,
    action: &OvaAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
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
            output::print_success(w, &format!("Exported {vm} to {ova_path}"), fmt)?;
        }
        OvaAction::Parse { ova_path } => {
            let detail = client.ova_parse(ova_path).await?;
            match fmt {
                OutputFormat::Table => {
                    let rows = output::vm_detail_rows(&detail);
                    output::print_list(w, &rows, fmt)?;
                }
                OutputFormat::Json => {
                    output::print_json(w, &detail)?;
                }
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
async fn docker(
    client: &UgosClient,
    action: &DockerAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
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
                    output::print_list(w, &rows, fmt)?;
                }
                OutputFormat::Json => {
                    output::print_json(w, &ov)?;
                }
            }
        }
        DockerAction::Status => {
            let status = client.docker_engine_status().await?;
            output::print_success(w, &format!("Docker engine: {status}"), fmt)?;
        }
        DockerAction::Ps { page, page_size } => {
            let result = client.container_list(*page, *page_size).await?;
            let containers = result.result.unwrap_or_default();
            let rows: Vec<output::ContainerRow> = containers.iter().map(Into::into).collect();
            output::print_list(w, &rows, fmt)?;
        }
        DockerAction::Start { id } => {
            client.container_start(id).await?;
            output::print_success(w, &format!("Started {id}"), fmt)?;
        }
        DockerAction::Show { id } => {
            let detail = client.container_show(id).await?;
            output::print_json(w, &detail)?;
        }
        DockerAction::Create {
            name,
            image,
            port,
            env,
            volume,
            restart,
            network,
            privileged,
            memory,
            cpus,
        } => {
            let spec = build_container_spec(
                name,
                image,
                port,
                env,
                volume,
                restart,
                network,
                *privileged,
                memory.as_ref(),
                cpus.as_ref(),
            )?;
            client.container_create(&spec).await?;
            output::print_success(w, &format!("Created container {name}"), fmt)?;
        }
        DockerAction::Stop { id } => {
            client.container_stop(id).await?;
            output::print_success(w, &format!("Stopped {id}"), fmt)?;
        }
        DockerAction::Restart { id } => {
            client.container_restart(id).await?;
            output::print_success(w, &format!("Restarted {id}"), fmt)?;
        }
        DockerAction::Kill { id } => {
            client.container_kill(id).await?;
            output::print_success(w, &format!("Killed {id}"), fmt)?;
        }
        DockerAction::Rm { id } => {
            client.container_remove(id).await?;
            output::print_success(w, &format!("Removed {id}"), fmt)?;
        }
        DockerAction::Images { page, page_size } => {
            let result = client.docker_image_list(*page, *page_size).await?;
            let images = result.result.unwrap_or_default();
            let rows: Vec<output::DockerImageRow> = images.iter().map(Into::into).collect();
            output::print_list(w, &rows, fmt)?;
        }
        DockerAction::Search { name } => {
            let images = client.docker_image_search(name, 1, 20).await?;
            let rows: Vec<output::DockerImageRow> = images.iter().map(Into::into).collect();
            output::print_list(w, &rows, fmt)?;
        }
        DockerAction::Pull { image, tag } => {
            client.docker_image_download(image, tag).await?;
            output::print_success(w, &format!("Pulling {image}:{tag}"), fmt)?;
        }
        DockerAction::Rmi { id } => {
            client.docker_image_delete(id).await?;
            output::print_success(w, &format!("Deleted image {id}"), fmt)?;
        }
        DockerAction::Export { id, path } => {
            client.docker_image_export(id, path).await?;
            output::print_success(w, &format!("Exporting image {id} to {path}"), fmt)?;
        }
        DockerAction::LoadUrl { url } => {
            client.docker_image_load_url(url).await?;
            output::print_success(w, &format!("Loading image from {url}"), fmt)?;
        }
        DockerAction::LoadPath { path } => {
            client.docker_image_load_path(path).await?;
            output::print_success(w, &format!("Loading image from {path}"), fmt)?;
        }
        DockerAction::Mirrors => {
            let mirrors = client.mirror_list().await?;
            let rows: Vec<output::MirrorRow> = mirrors.iter().map(Into::into).collect();
            output::print_list(w, &rows, fmt)?;
        }
        DockerAction::MirrorAdd { alias, address } => {
            client.mirror_add(alias, address).await?;
            output::print_success(w, &format!("Added mirror {alias}"), fmt)?;
        }
        DockerAction::MirrorDelete { id } => {
            client.mirror_delete(*id).await?;
            output::print_success(w, &format!("Deleted mirror {id}"), fmt)?;
        }
        DockerAction::MirrorSwitch { id } => {
            client.mirror_switch(*id).await?;
            output::print_success(w, &format!("Switched to mirror {id}"), fmt)?;
        }
        DockerAction::Logs { id, lines } => {
            let logs = client.container_logs(id, *lines).await?;
            output::print_json(w, &logs)?;
        }
        DockerAction::Clone { id, name } => {
            client.container_clone(id, name).await?;
            output::print_success(w, &format!("Cloned {id} as {name}"), fmt)?;
        }
        DockerAction::Batch { action, ids } => {
            client.container_batch(ids, action).await?;
            output::print_success(w, &format!("{action} {} containers", ids.len()), fmt)?;
        }
        DockerAction::Compose { project } => {
            let data = client.compose_containers(project).await?;
            output::print_json(w, &data)?;
        }
        DockerAction::ProxyGet => {
            let proxy = client.docker_proxy_get().await?;
            output::print_json(w, &proxy)?;
        }
        DockerAction::ProxySet { json } => {
            let proxy: serde_json::Value =
                serde_json::from_str(json).map_err(|e| anyhow::anyhow!("invalid JSON: {e}"))?;
            client.docker_proxy_set(&proxy).await?;
            output::print_success(w, "Updated HTTP proxy", fmt)?;
        }
    }
    Ok(())
}

async fn info(client: &UgosClient, fmt: OutputFormat, w: &mut impl Write) -> Result<()> {
    let host = client.host_info().await?;
    match fmt {
        OutputFormat::Table => {
            let rows = output::host_info_rows(&host);
            output::print_list(w, &rows, fmt)?;
        }
        OutputFormat::Json => {
            output::print_json(w, &host)?;
        }
    }
    Ok(())
}

// ── Builder helpers ─────────────────────────────────────────────────

fn parse_mem_limit(s: &str) -> i64 {
    let s = s.trim().to_lowercase();
    s.strip_suffix('g')
        .map(|n| n.parse::<i64>().unwrap_or(0) * 1024 * 1024 * 1024)
        .or_else(|| {
            s.strip_suffix('m')
                .map(|n| n.parse::<i64>().unwrap_or(0) * 1024 * 1024)
        })
        .unwrap_or_else(|| s.parse::<i64>().unwrap_or(0))
}

#[allow(clippy::too_many_arguments, clippy::similar_names)]
fn build_container_spec(
    name: &str,
    image: &str,
    ports: &[String],
    envs: &[String],
    volumes: &[String],
    restart: &str,
    network: &str,
    privileged: bool,
    memory: Option<&String>,
    cpus: Option<&f64>,
) -> Result<ugos_client::types::docker::ContainerDetail> {
    // ── Validate ────────────────────────────────────────────────────
    anyhow::ensure!(!name.is_empty(), "container name cannot be empty");
    anyhow::ensure!(
        image
            .chars()
            .all(|c| c.is_alphanumeric() || "/:._-".contains(c)),
        "invalid image name: {image}"
    );
    for p in ports {
        let mapping = p.split('/').next().unwrap_or(p);
        let parts: Vec<&str> = mapping.split(':').collect();
        anyhow::ensure!(
            parts.len() == 2 && parts[0].parse::<u16>().is_ok() && parts[1].parse::<u16>().is_ok(),
            "invalid port mapping '{p}', expected host:container (e.g. 8080:80)"
        );
    }
    for e in envs {
        anyhow::ensure!(e.contains('='), "invalid env var '{e}', expected KEY=VALUE");
    }
    for v in volumes {
        anyhow::ensure!(
            v.contains(':'),
            "invalid volume '{v}', expected host_path:container_path"
        );
    }
    anyhow::ensure!(
        matches!(restart, "no" | "always" | "unless-stopped"),
        "invalid restart policy '{restart}', expected: no, always, unless-stopped"
    );
    anyhow::ensure!(
        matches!(network, "bridge" | "host"),
        "invalid network mode '{network}', expected: bridge, host"
    );
    if let Some(c) = cpus {
        anyhow::ensure!(*c >= 0.0, "CPU limit cannot be negative");
    }

    // ── Build ───────────────────────────────────────────────────────
    let (img_name, img_ver) = image
        .split_once(':')
        .map_or((image, "latest"), |(n, t)| (n, t));

    let port_mapping: Vec<serde_json::Value> = ports
        .iter()
        .map(|p| {
            let (mapping, proto) = p
                .split_once('/')
                .map_or((p.as_str(), "tcp"), |(m, pr)| (m, pr));
            let (host, container) = mapping.split_once(':').unwrap_or(("0", mapping));
            serde_json::json!({
                "hostPort": host.parse::<u16>().unwrap_or(0),
                "containerPort": container.parse::<u16>().unwrap_or(0),
                "protocol": proto
            })
        })
        .collect();

    let env_vars: Vec<ugos_client::types::docker::EnvVar> = envs
        .iter()
        .filter_map(|e| {
            e.split_once('=')
                .map(|(k, v)| ugos_client::types::docker::EnvVar {
                    variable: k.to_owned(),
                    price: v.to_owned(),
                })
        })
        .collect();

    let vols: Vec<serde_json::Value> = volumes
        .iter()
        .filter_map(|v| {
            v.split_once(':')
                .map(|(host, ctr)| serde_json::json!({"hostPath": host, "containerPath": ctr}))
        })
        .collect();

    let mem_limit = memory.map_or(0, |s| parse_mem_limit(s));
    #[allow(clippy::cast_possible_truncation)]
    let cpu_limit = cpus.map_or(0, |c| (*c * 100.0) as i64);

    Ok(ugos_client::types::docker::ContainerDetail {
        image_name: img_name.to_owned(),
        image_version: img_ver.to_owned(),
        tag: image.to_owned(),
        container_name: name.to_owned(),
        cpu_limit,
        mem_limit,
        no_restrictions: mem_limit == 0 && cpu_limit == 0,
        network_mode: network.to_owned(),
        hardware_acceleration: false,
        privileged_mode: privileged,
        abnormal_reset: restart != "no",
        run_container: true,
        port_mapping,
        volumes: if vols.is_empty() { None } else { Some(vols) },
        environment_variables: env_vars,
        container_run_command: vec![],
        perm_and_func: vec![],
        project_name: String::new(),
        image_id: String::new(),
    })
}

#[allow(clippy::too_many_arguments, clippy::similar_names)]
fn build_vm_spec(
    name: &str,
    os: &str,
    cores: i64,
    memory_mib: i64,
    disk_mib: i64,
    iso: Option<&String>,
    network: &str,
    boot_type: &str,
    storage: &str,
    autostart: bool,
) -> Result<ugos_client::types::kvm::VmDetail> {
    use ugos_client::types::kvm::{
        VmDetail, VmDevice, VmDisk, VmImage, VmNetwork, VmOtherConfig, VmResource,
    };

    // ── Validate ────────────────────────────────────────────────────
    anyhow::ensure!(!name.is_empty(), "VM name cannot be empty");
    anyhow::ensure!(
        matches!(os, "linux" | "windows" | "other"),
        "invalid OS type '{os}', expected: linux, windows, other"
    );
    anyhow::ensure!(cores > 0, "cores must be > 0, got {cores}");
    anyhow::ensure!(memory_mib > 0, "memory must be > 0 MiB, got {memory_mib}");
    anyhow::ensure!(disk_mib > 0, "disk must be > 0 MiB, got {disk_mib}");
    anyhow::ensure!(
        matches!(boot_type, "uefi" | "bios"),
        "invalid boot type '{boot_type}', expected: uefi, bios"
    );

    // ── Build ───────────────────────────────────────────────────────

    let memory_kib = memory_mib * 1024;
    let disk_bytes = disk_mib * 1024 * 1024;

    let images = iso
        .map(|path| {
            vec![VmImage {
                path: path.clone(),
                dev: "hda".into(),
                order: 2,
            }]
        })
        .unwrap_or_default();

    Ok(VmDetail {
        virtual_machine_name: String::new(), // vm_create generates UUID
        virtual_machine_display_name: name.to_owned(),
        system_type: os.to_owned(),
        system_version: String::new(),
        core: VmResource { value: cores },
        memory: VmResource { value: memory_kib },
        images,
        dists: vec![VmDisk {
            bus: "virtio".into(),
            size: disk_bytes,
            dev: "vda".into(),
            path: String::new(),
            order: 1,
        }],
        networks: vec![VmNetwork {
            name: network.to_owned(),
            mac_address: String::new(),
            nic_type: "virtio".into(),
        }],
        device: VmDevice {
            usb_controller: 2,
            usb_devices: vec![],
            graphics_card: "virtio".into(),
            boot_type: boot_type.to_owned(),
        },
        other_config: VmOtherConfig {
            auto_matic_start_up: autostart,
            keyboard_language: "en".into(),
            share_directory: vec![],
        },
        storage_name: storage.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_mem_limit ─────────────────────────────────────────────

    #[test]
    fn parse_mem_limit_megabytes() {
        assert_eq!(parse_mem_limit("512m"), 512 * 1024 * 1024);
    }

    #[test]
    fn parse_mem_limit_gigabytes() {
        assert_eq!(parse_mem_limit("2g"), 2 * 1024 * 1024 * 1024);
    }

    #[test]
    fn parse_mem_limit_raw_bytes() {
        assert_eq!(parse_mem_limit("1048576"), 1_048_576);
    }

    #[test]
    fn parse_mem_limit_garbage() {
        assert_eq!(parse_mem_limit("abc"), 0);
    }

    #[test]
    fn parse_mem_limit_case_insensitive() {
        assert_eq!(parse_mem_limit("1G"), 1024 * 1024 * 1024);
        assert_eq!(parse_mem_limit("256M"), 256 * 1024 * 1024);
    }

    // ── build_container_spec validation ─────────────────────────────

    #[test]
    fn container_spec_valid() {
        let spec = build_container_spec(
            "test",
            "nginx:latest",
            &["8080:80".into()],
            &["FOO=bar".into()],
            &["/data:/data".into()],
            "no",
            "bridge",
            false,
            None,
            None,
        );
        assert!(spec.is_ok());
        let s = spec.unwrap();
        assert_eq!(s.container_name, "test");
        assert_eq!(s.image_name, "nginx");
        assert_eq!(s.image_version, "latest");
        assert_eq!(s.tag, "nginx:latest");
        assert!(s.no_restrictions);
        assert!(!s.abnormal_reset);
    }

    #[test]
    fn container_spec_image_no_tag() {
        let spec = build_container_spec(
            "test",
            "nginx",
            &[],
            &[],
            &[],
            "no",
            "bridge",
            false,
            None,
            None,
        )
        .unwrap();
        assert_eq!(spec.image_name, "nginx");
        assert_eq!(spec.image_version, "latest");
    }

    #[test]
    fn container_spec_bad_port() {
        let err = build_container_spec(
            "test",
            "nginx",
            &["abc:def".into()],
            &[],
            &[],
            "no",
            "bridge",
            false,
            None,
            None,
        );
        assert!(err.is_err());
        assert!(
            err.unwrap_err()
                .to_string()
                .contains("invalid port mapping")
        );
    }

    #[test]
    fn container_spec_bad_env() {
        let err = build_container_spec(
            "test",
            "nginx",
            &[],
            &["NOEQUALS".into()],
            &[],
            "no",
            "bridge",
            false,
            None,
            None,
        );
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("invalid env var"));
    }

    #[test]
    fn container_spec_bad_volume() {
        let err = build_container_spec(
            "test",
            "nginx",
            &[],
            &[],
            &["nocolon".into()],
            "no",
            "bridge",
            false,
            None,
            None,
        );
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("invalid volume"));
    }

    #[test]
    fn container_spec_bad_restart() {
        let err = build_container_spec(
            "test",
            "nginx",
            &[],
            &[],
            &[],
            "bogus",
            "bridge",
            false,
            None,
            None,
        );
        assert!(err.is_err());
        assert!(
            err.unwrap_err()
                .to_string()
                .contains("invalid restart policy")
        );
    }

    #[test]
    fn container_spec_bad_network() {
        let err = build_container_spec(
            "test",
            "nginx",
            &[],
            &[],
            &[],
            "no",
            "overlay",
            false,
            None,
            None,
        );
        assert!(err.is_err());
        assert!(
            err.unwrap_err()
                .to_string()
                .contains("invalid network mode")
        );
    }

    #[test]
    fn container_spec_memory_limit() {
        let mem = "512m".to_string();
        let spec = build_container_spec(
            "test",
            "nginx",
            &[],
            &[],
            &[],
            "no",
            "bridge",
            false,
            Some(&mem),
            None,
        )
        .unwrap();
        assert_eq!(spec.mem_limit, 512 * 1024 * 1024);
        assert!(!spec.no_restrictions);
    }

    #[test]
    fn container_spec_restart_unless_stopped() {
        let spec = build_container_spec(
            "test",
            "nginx",
            &[],
            &[],
            &[],
            "unless-stopped",
            "bridge",
            false,
            None,
            None,
        )
        .unwrap();
        assert!(spec.abnormal_reset);
    }

    // ── build_vm_spec validation ────────────────────────────────────

    #[test]
    fn vm_spec_valid() {
        let spec = build_vm_spec(
            "TestVM",
            "linux",
            4,
            8192,
            51200,
            None,
            "vnet-bridge0",
            "uefi",
            "volume1",
            false,
        );
        assert!(spec.is_ok());
        let s = spec.unwrap();
        assert_eq!(s.virtual_machine_display_name, "TestVM");
        assert_eq!(s.core.value, 4);
        assert_eq!(s.memory.value, 8192 * 1024);
        assert_eq!(s.dists[0].size, 51200 * 1024 * 1024);
        assert!(!s.other_config.auto_matic_start_up);
    }

    #[test]
    fn vm_spec_with_iso() {
        let iso = "/volume1/iso/ubuntu.iso".to_string();
        let spec = build_vm_spec(
            "TestVM",
            "linux",
            2,
            4096,
            20480,
            Some(&iso),
            "vnet-bridge0",
            "uefi",
            "volume1",
            true,
        )
        .unwrap();
        assert_eq!(spec.images.len(), 1);
        assert_eq!(spec.images[0].path, "/volume1/iso/ubuntu.iso");
        assert!(spec.other_config.auto_matic_start_up);
    }

    #[test]
    fn vm_spec_bad_os() {
        let err = build_vm_spec(
            "Test",
            "freebsd",
            2,
            4096,
            20480,
            None,
            "vnet-bridge0",
            "uefi",
            "volume1",
            false,
        );
        assert!(err.unwrap_err().to_string().contains("invalid OS type"));
    }

    #[test]
    fn vm_spec_zero_cores() {
        let err = build_vm_spec(
            "Test",
            "linux",
            0,
            4096,
            20480,
            None,
            "vnet-bridge0",
            "uefi",
            "volume1",
            false,
        );
        assert!(err.unwrap_err().to_string().contains("cores must be > 0"));
    }

    #[test]
    fn vm_spec_zero_memory() {
        let err = build_vm_spec(
            "Test",
            "linux",
            2,
            0,
            20480,
            None,
            "vnet-bridge0",
            "uefi",
            "volume1",
            false,
        );
        assert!(err.unwrap_err().to_string().contains("memory must be > 0"));
    }

    #[test]
    fn vm_spec_bad_boot_type() {
        let err = build_vm_spec(
            "Test",
            "linux",
            2,
            4096,
            20480,
            None,
            "vnet-bridge0",
            "grub",
            "volume1",
            false,
        );
        assert!(err.unwrap_err().to_string().contains("invalid boot type"));
    }

    #[test]
    fn vm_spec_empty_name() {
        let err = build_vm_spec(
            "",
            "linux",
            2,
            4096,
            20480,
            None,
            "vnet-bridge0",
            "uefi",
            "volume1",
            false,
        );
        assert!(
            err.unwrap_err()
                .to_string()
                .contains("VM name cannot be empty")
        );
    }
}
