//! `pool`, `volume` and `disk`: the Storage Manager, read only.

use std::io::Write;

use anyhow::Result;
use ugos_client::UgosClient;
use ugos_client::api::storage::{StorageApi, pick_pool, pick_volume};

use crate::cli::{DiskAction, OutputFormat, PoolAction, VolumeAction};
use crate::output;
use crate::output::nas_storage as rows;

/// Run a `pool` subcommand.
pub async fn pool(
    client: &UgosClient,
    action: &PoolAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    let pools = client.pools().await?;
    match action {
        PoolAction::List => match fmt {
            OutputFormat::Table => {
                let table: Vec<rows::PoolRow> = pools.iter().map(Into::into).collect();
                output::print_list(w, &table, fmt)
            }
            OutputFormat::Json => output::print_json(w, &pools),
        },
        PoolAction::Show { pool } => {
            let found = pick_pool(&pools, pool)?;
            match fmt {
                OutputFormat::Table => output::print_list(w, &rows::pool_rows(&found), fmt),
                OutputFormat::Json => output::print_json(w, &found),
            }
        }
    }
}

/// Run a `volume` subcommand.
pub async fn volume(
    client: &UgosClient,
    action: &VolumeAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    let volumes = client.volumes().await?;
    match action {
        VolumeAction::List => match fmt {
            OutputFormat::Table => {
                let table: Vec<rows::VolumeRow> = volumes.iter().map(Into::into).collect();
                output::print_list(w, &table, fmt)
            }
            OutputFormat::Json => output::print_json(w, &volumes),
        },
        VolumeAction::Show { volume } => {
            let found = pick_volume(&volumes, volume)?;
            match fmt {
                OutputFormat::Table => output::print_list(w, &rows::volume_rows(&found), fmt),
                OutputFormat::Json => output::print_json(w, &found),
            }
        }
    }
}

/// Run a `disk` subcommand.
pub async fn disk(
    client: &UgosClient,
    action: &DiskAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    match action {
        DiskAction::List => disk_list(client, fmt, w).await,
        DiskAction::Show { disk } => disk_show(client, disk, fmt, w).await,
        DiskAction::Smart { disk } => disk_smart(client, disk, fmt, w).await,
        DiskAction::History { disk } => disk_history(client, disk, fmt, w).await,
        DiskAction::Plans => disk_plans(client, fmt, w).await,
    }
}

async fn disk_list(client: &UgosClient, fmt: OutputFormat, w: &mut impl Write) -> Result<()> {
    let disks = client.disks().await?;
    match fmt {
        OutputFormat::Table => {
            let table: Vec<rows::DiskRow> = disks.iter().map(Into::into).collect();
            output::print_list(w, &table, fmt)
        }
        OutputFormat::Json => output::print_json(w, &disks),
    }
}

async fn disk_show(
    client: &UgosClient,
    which: &str,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    let found = client.disk_find(which).await?;
    let attribute = client.disk_attribute(&found.dev_name).await?;
    match fmt {
        OutputFormat::Table => output::print_list(w, &rows::disk_rows(&found, &attribute), fmt),
        OutputFormat::Json => output::print_json(
            w,
            &serde_json::json!({ "disk": found, "attribute": attribute }),
        ),
    }
}

async fn disk_smart(
    client: &UgosClient,
    which: &str,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    let found = client.disk_find(which).await?;
    let smart = client.disk_smart(&found.dev_name).await?;
    match fmt {
        OutputFormat::Table => output::print_list(w, &rows::smart_rows(&smart), fmt),
        OutputFormat::Json => output::print_json(w, &smart),
    }
}

async fn disk_history(
    client: &UgosClient,
    which: &str,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    let found = client.disk_find(which).await?;
    let history = client.disk_history(&found.dev_name).await?;
    match fmt {
        OutputFormat::Table => {
            let table: Vec<rows::HistoryRow> = history.iter().map(Into::into).collect();
            output::print_list(w, &table, fmt)
        }
        OutputFormat::Json => output::print_json(w, &history),
    }
}

async fn disk_plans(client: &UgosClient, fmt: OutputFormat, w: &mut impl Write) -> Result<()> {
    let plans = client.disk_plans().await?;
    match fmt {
        OutputFormat::Table => {
            let table: Vec<rows::PlanRow> = plans.iter().map(Into::into).collect();
            output::print_list(w, &table, fmt)
        }
        OutputFormat::Json => output::print_json(w, &plans),
    }
}
