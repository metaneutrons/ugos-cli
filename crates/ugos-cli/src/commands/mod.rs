//! Command dispatch — maps CLI actions to API calls and output.

pub mod vmspec;

use std::io::Write;

use anyhow::Result;
use ugos_client::UgosClient;
use ugos_client::api::docker::DockerApi;
use ugos_client::api::download::DownloadApi;
use ugos_client::api::files::FilesApi;
use ugos_client::api::kvm::KvmApi;
use ugos_client::api::system::SystemApi;

use crate::cli::{
    DockerAction, DownloadAction, FsAction, ImageAction, LogAction, NetworkAction, OutputFormat,
    OvaAction, PassthroughAction, Resource, SnapshotAction, StorageAction, SystemAction, UsbAction,
    VmAction, VncAction,
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
        Resource::Overview => overview(client, fmt, w).await,
        Resource::Passthrough { action } => passthrough(client, action, fmt, w).await,
        Resource::System { action } => system(client, action, fmt, w).await,
        Resource::Download { action } => download(client, action, fmt, w).await,
        Resource::Fs { action } => fs(client, action, fmt, w).await,
    }
}

async fn fs(
    client: &UgosClient,
    action: &FsAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    match action {
        FsAction::Ls { path } => {
            let entries = client.fs_list(path).await?;
            match fmt {
                OutputFormat::Table => {
                    let rows: Vec<output::FileRow> = entries.iter().map(Into::into).collect();
                    output::print_list(w, &rows, fmt)?;
                }
                OutputFormat::Json => output::print_json(w, &entries)?,
            }
        }
        FsAction::Volumes => {
            let vols = client.fs_volumes().await?;
            match fmt {
                OutputFormat::Table => {
                    let rows: Vec<output::VolumeRow> = vols.iter().map(Into::into).collect();
                    output::print_list(w, &rows, fmt)?;
                }
                OutputFormat::Json => output::print_json(w, &vols)?,
            }
        }
        FsAction::Mkdir { path } => {
            client.fs_mkdir(path).await?;
            output::print_success(w, &format!("Created {path}"), fmt)?;
        }
        FsAction::Rm { paths, forever } => {
            client.fs_remove(paths, *forever).await?;
            let where_to = if *forever {
                "permanently"
            } else {
                "to the recycle bin"
            };
            output::print_success(
                w,
                &format!("Deleted {} entries {where_to}", paths.len()),
                fmt,
            )?;
        }
        FsAction::Get { remote, local } => {
            let dest = local
                .clone()
                .unwrap_or_else(|| remote.rsplit('/').next().unwrap_or("download").to_owned());
            let quiet = matches!(fmt, OutputFormat::Json);
            let progress = move |written: u64| {
                if !quiet {
                    let mut err = std::io::stderr();
                    let _ = write!(err, "\rreceived {} MiB", written / 1_048_576);
                    let _ = err.flush();
                }
            };
            let bytes = client
                .fs_download(remote, std::path::Path::new(&dest), &progress)
                .await?;
            if !quiet {
                let _ = writeln!(std::io::stderr());
            }
            output::print_success(w, &format!("Wrote {dest} ({bytes} bytes)"), fmt)?;
        }
        FsAction::Put { local, remote_dir } => {
            let placed = client
                .fs_upload(std::path::Path::new(local), remote_dir)
                .await?;
            output::print_success(w, &format!("Uploaded to {placed}"), fmt)?;
        }
        FsAction::Mv { path, new_name } => {
            client.fs_rename(path, new_name).await?;
            output::print_success(w, &format!("Renamed to {new_name}"), fmt)?;
        }
    }
    Ok(())
}

async fn download(
    client: &UgosClient,
    action: &DownloadAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    match action {
        DownloadAction::List => {
            let mut tasks = client.download_list().await?;
            tasks.extend(client.download_completed().await?);
            match fmt {
                OutputFormat::Table => {
                    let rows: Vec<output::DownloadRow> = tasks.iter().map(Into::into).collect();
                    output::print_list(w, &rows, fmt)?;
                }
                OutputFormat::Json => output::print_json(w, &tasks)?,
            }
        }
        DownloadAction::Add { url, dir } => {
            client.download_add(url, dir.as_deref()).await?;
            output::print_success(w, &format!("Queued {url}"), fmt)?;
        }
        DownloadAction::Check { url } => {
            let status = client.download_check(url).await?;
            let msg = if status == 0 {
                format!("{url} can be downloaded")
            } else {
                format!("{url} was rejected (status {status})")
            };
            output::print_success(w, &msg, fmt)?;
        }
        DownloadAction::Rm {
            id,
            delete_file,
            running,
        } => {
            client.download_remove(id, *delete_file, *running).await?;
            output::print_success(w, &format!("Removed task {id}"), fmt)?;
        }
        DownloadAction::Status => {
            let path = client.download_path().await?;
            let speed = client.download_speed().await?;
            match fmt {
                OutputFormat::Table => {
                    let rows = output::download_status_rows(&path, &speed);
                    output::print_list(w, &rows, fmt)?;
                }
                OutputFormat::Json => {
                    output::print_json(w, &serde_json::json!({"path": path, "speed": speed}))?;
                }
            }
        }
    }
    Ok(())
}

async fn system(
    client: &UgosClient,
    action: &SystemAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    match action {
        SystemAction::Info => {
            let info = client.machine_info().await?;
            match fmt {
                OutputFormat::Table => {
                    let rows = output::machine_info_rows(&info);
                    output::print_list(w, &rows, fmt)?;
                }
                OutputFormat::Json => output::print_json(w, &info)?,
            }
        }
        SystemAction::Stat => {
            let stats = client.system_stats().await?;
            match fmt {
                OutputFormat::Table => {
                    let rows = output::system_stat_rows(&stats);
                    output::print_list(w, &rows, fmt)?;
                }
                OutputFormat::Json => output::print_json(w, &stats)?,
            }
        }
        SystemAction::Processes { limit } => {
            let procs = client.processes().await?;
            match fmt {
                OutputFormat::Table => {
                    let rows = output::process_rows(&procs, *limit);
                    output::print_list(w, &rows, fmt)?;
                }
                OutputFormat::Json => output::print_json(w, &procs)?,
            }
        }
        SystemAction::Services => {
            let svcs = client.services().await?;
            match fmt {
                OutputFormat::Table => {
                    let rows = output::service_rows(&svcs);
                    output::print_list(w, &rows, fmt)?;
                }
                OutputFormat::Json => output::print_json(w, &svcs)?,
            }
        }
    }
    Ok(())
}

async fn overview(client: &UgosClient, fmt: OutputFormat, w: &mut impl Write) -> Result<()> {
    let ov = client.overview().await?;
    match fmt {
        OutputFormat::Table => {
            let rows = output::overview_rows(&ov);
            output::print_list(w, &rows, fmt)?;
        }
        OutputFormat::Json => output::print_json(w, &ov)?,
    }
    Ok(())
}

async fn passthrough(
    client: &UgosClient,
    action: &PassthroughAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    match action {
        PassthroughAction::List => {
            let devices = client.passthrough_devices().await?;
            output::print_json(w, &devices)?;
            let _ = fmt;
        }
    }
    Ok(())
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
        VmAction::Create(args) => {
            let spec = vmspec::build(args)?;
            if args.dry_run {
                output::print_json(w, &spec)?;
            } else {
                let uuid = client.vm_create(&spec).await?;
                let msg = if uuid.is_empty() {
                    format!("Created VM {}", args.name)
                } else {
                    format!("Created VM {} ({uuid})", args.name)
                };
                output::print_success(w, &msg, fmt)?;
            }
        }
        VmAction::Update(args) => {
            let current = client.vm_show(&args.name).await?;
            let spec = vmspec::update(&current, args)?;
            if args.dry_run {
                output::print_json(w, &spec)?;
            } else {
                client.vm_update(&spec).await?;
                output::print_success(
                    w,
                    &format!("Updated VM {}", spec.virtual_machine_display_name),
                    fmt,
                )?;
            }
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
        SnapshotAction::Create { vm } => {
            let name = client.snapshot_create(vm).await?;
            output::print_success(w, &format!("Created snapshot {name}"), fmt)?;
        }
        SnapshotAction::Delete { vm, name } => {
            client.snapshot_delete(vm, name).await?;
            output::print_success(w, &format!("Deleted snapshot {name}"), fmt)?;
        }
        SnapshotAction::Revert {
            vm,
            name,
            snapshot_first,
        } => {
            client.snapshot_revert(vm, name, *snapshot_first).await?;
            output::print_success(w, &format!("Reverted to snapshot {name}"), fmt)?;
        }
        SnapshotAction::Describe {
            vm,
            name,
            description,
        } => {
            client.snapshot_describe(vm, name, description).await?;
            output::print_success(w, &format!("Described snapshot {name}"), fmt)?;
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
        StorageAction::Df => {
            let usage = client.storage_usage_list().await?;
            match fmt {
                OutputFormat::Table => {
                    let rows = output::storage_usage_rows(&usage);
                    output::print_list(w, &rows, fmt)?;
                }
                OutputFormat::Json => output::print_json(w, &usage)?,
            }
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
        ImageAction::Register { path, name } => {
            let image_name = name.clone().unwrap_or_else(|| default_iso_name(path));
            let file_name = path.rsplit('/').next().unwrap_or(path).to_owned();
            client.image_register(path, &image_name, &file_name).await?;
            output::print_success(w, &format!("Registered {image_name}"), fmt)?;
        }
        ImageAction::Upload { source, name } => {
            let iso_name = name.clone().unwrap_or_else(|| default_iso_name(source));
            // Progress goes to stderr so that piping `-o json` stays clean.
            let quiet = matches!(fmt, OutputFormat::Json);
            let progress = move |done: usize, total: usize| {
                if quiet {
                    return;
                }
                let mut err = std::io::stderr();
                let _ = if total == 0 {
                    write!(err, "\rdownloading {} MiB", done / 1_048_576)
                } else {
                    write!(err, "\ruploading chunk {done}/{total}   ")
                };
                let _ = err.flush();
            };

            let file_name = if source.starts_with("http://") || source.starts_with("https://") {
                client.image_upload_url(source, &iso_name, &progress).await
            } else {
                client
                    .image_upload(std::path::Path::new(source), &iso_name, &progress)
                    .await
            }?;
            if !quiet {
                let mut err = std::io::stderr();
                let _ = writeln!(err);
            }
            output::print_success(w, &format!("Uploaded {iso_name} ({file_name})"), fmt)?;
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
        DockerAction::ProjectLs => {
            let projects = client.project_list().await?;
            let rows: Vec<output::ComposeProjectRow> = projects.iter().map(Into::into).collect();
            output::print_list(w, &rows, fmt)?;
        }
        DockerAction::ProjectShow { name } => {
            let project = client.project_show(name).await?;
            output::print_json(w, &project)?;
        }
        DockerAction::ProjectCreate {
            name,
            file,
            path,
            run,
        } => {
            let content = std::fs::read_to_string(file)
                .map_err(|e| anyhow::anyhow!("reading {file}: {e}"))?;
            let path = if let Some(p) = path {
                p.clone()
            } else {
                let shared = client.project_shared_folder().await?;
                format!("{}/{name}", shared.trim_end_matches('/'))
            };
            client.project_create(name, &path, &content, *run).await?;
            output::print_success(w, &format!("Created project {name} at {path}"), fmt)?;
        }
        DockerAction::ProjectStart { name } => {
            client.project_start(name).await?;
            output::print_success(w, &format!("Started project {name}"), fmt)?;
        }
        DockerAction::ProjectStop { name } => {
            client.project_stop(name).await?;
            output::print_success(w, &format!("Stopped project {name}"), fmt)?;
        }
        DockerAction::ProjectRestart { name } => {
            client.project_restart(name).await?;
            output::print_success(w, &format!("Restarted project {name}"), fmt)?;
        }
        DockerAction::ProjectRm { name, del_images } => {
            client.project_remove(name, *del_images).await?;
            output::print_success(w, &format!("Removed project {name}"), fmt)?;
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

/// Live-captured default for `bridge` mode; UGOS sends this subnet
/// explicitly even though it's the engine default. Only "bridge" and "host"
/// are accepted by `build_container_spec`, and "host" needs no subnet entry.
fn default_subnet_settings(network: &str) -> Vec<ugos_client::types::docker::SubnetSetting> {
    if network == "bridge" {
        vec![ugos_client::types::docker::SubnetSetting {
            network_name: "bridge".to_owned(),
            subnet: "172.17.0.0/16".to_owned(),
        }]
    } else {
        vec![]
    }
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
    let img_ver = image.split_once(':').map_or("latest", |(_, t)| t);

    let port_mapping: Vec<ugos_client::types::docker::PortMapping> = ports
        .iter()
        .map(|p| {
            let (mapping, proto) = p
                .split_once('/')
                .map_or((p.as_str(), "tcp"), |(m, pr)| (m, pr));
            let (host, container) = mapping.split_once(':').unwrap_or(("0", mapping));
            ugos_client::types::docker::PortMapping {
                nas_port: host.parse::<i64>().unwrap_or(0),
                container_port: container.parse::<i64>().unwrap_or(0),
                port_type: proto.to_owned(),
            }
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
        image_name: image.to_owned(),
        image_version: img_ver.to_owned(),
        tag: image.to_owned(),
        container_name: name.to_owned(),
        cpu_limit,
        mem_limit,
        no_restrictions: mem_limit == 0 && cpu_limit == 0,
        network_mode: network.to_owned(),
        hardware_acceleration: false,
        gpu_ids: vec![],
        subnet_settings: default_subnet_settings(network),
        privileged_mode: privileged,
        abnormal_reset: restart != "no",
        run_container: true,
        port_mapping,
        volumes: if vols.is_empty() { None } else { Some(vols) },
        environment_variables: env_vars,
        container_run_command: vec![],
        perm_and_func: None,
        project_name: String::new(),
        image_id: String::new(),
    })
}

/// Derive an image display name from a path or URL: file name without its
/// extension.
fn default_iso_name(source: &str) -> String {
    let last = source.rsplit(['/', '\\']).next().unwrap_or(source);
    last.rsplit_once('.')
        .map_or(last, |(stem, _)| stem)
        .to_owned()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
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
        assert_eq!(s.image_name, "nginx:latest");
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
}
