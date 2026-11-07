//! `update`, `backup`, `index` and the about and power commands of `system`.

use std::io::Write;
use std::time::{Duration, Instant};

use anyhow::{Result, bail};
use ugos_client::UgosClient;
use ugos_client::ops::hw::{poweroff, reboot};
use ugos_client::ops::update::backup::{self, RestoreService};
use ugos_client::ops::update::firmware::{self, ConfigChange, Interval};
use ugos_client::ops::update::{about, index};

use super::hw::show;
use super::share::with_force_hint;
use crate::cli::{
    BackupAction, ForceArgs, IndexAction, OutputFormat, UpdateAction, UpdateConfigArgs,
};
use crate::output;
use crate::output::update as rows;

fn config_change(args: &UpdateConfigArgs) -> Result<ConfigChange> {
    let interval = match (args.every, args.unit, args.default_interval) {
        (Some(value), Some(unit), false) => Some(Interval::Every {
            value,
            unit: unit.code(),
        }),
        (None, _, true) => Some(Interval::Default),
        (None, None, false) => None,
        _ => bail!("--every needs --unit, and --default-interval stands alone"),
    };
    Ok(ConfigChange {
        auto_update: args.auto.map(crate::cli::AutoUpdate::code),
        interval,
    })
}

async fn firmware_state(client: &UgosClient, fmt: OutputFormat, w: &mut impl Write) -> Result<()> {
    let status = firmware::status(client).await?;
    let flag = firmware::is_new(client).await?;
    show(
        w,
        fmt,
        &serde_json::json!({ "status": status, "newer_exists": flag.has_version }),
        &rows::firmware_rows(&status, &flag),
    )
}

/// Wait for a manual install: back off, give up after a deadline, and take a
/// dropped connection after the install as the restart it is.
async fn wait_for_install(client: &UgosClient, w: &mut impl Write) -> Result<()> {
    let deadline = Instant::now() + Duration::from_mins(45);
    let mut pause = Duration::from_secs(2);
    let mut seen_install = false;
    while Instant::now() < deadline {
        tokio::time::sleep(pause).await;
        pause = (pause * 2).min(Duration::from_secs(15));
        match firmware::manual_status(client).await {
            Ok(s) if s.error_code != 0 => bail!("the install failed with code {}", s.error_code),
            Ok(s) if s.current_stage == "complete" => {
                writeln!(w, "The firmware is installed; the NAS restarts")?;
                return Ok(());
            }
            Ok(s) => {
                seen_install = true;
                writeln!(w, "Installing: {}%", s.process.max(0))?;
            }
            // The NAS goes down to restart once it is done.
            Err(_) if seen_install => {
                writeln!(w, "The NAS stopped answering: it restarts")?;
                return Ok(());
            }
            Err(_) => {}
        }
    }
    bail!("the install did not finish in 45 minutes; check `update progress`")
}

/// Run an `update` subcommand.
pub async fn update(
    client: &UgosClient,
    action: &UpdateAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    match action {
        UpdateAction::Status => firmware_state(client, fmt, w).await,
        UpdateAction::Config(args) => {
            let change = config_change(args)?;
            let got = if change == ConfigChange::default() {
                firmware::config(client).await?
            } else {
                firmware::set_config(client, &change, args.force)
                    .await
                    .map_err(with_force_hint)?
            };
            show(w, fmt, &got, &rows::firmware_config_rows(&got))
        }
        UpdateAction::Install(a) => {
            let _ = firmware::install(client, a.force)
                .await
                .map_err(with_force_hint)?;
            writeln!(w, "The install started; the NAS restarts when it is done")?;
            Ok(())
        }
        UpdateAction::Upload(a) => {
            firmware::upload(client, &a.file, a.force, &|done, all| {
                let _ = writeln!(std::io::stderr(), "Sent chunk {done} of {all}");
            })
            .await
            .map_err(with_force_hint)?;
            wait_for_install(client, w).await
        }
        UpdateAction::Key(a) => {
            firmware::import_key(client, &a.file, a.force)
                .await
                .map_err(with_force_hint)?;
            writeln!(w, "The key is trusted")?;
            Ok(())
        }
        UpdateAction::Clean => {
            firmware::clean(client).await?;
            writeln!(w, "The stored failure is cleared")?;
            Ok(())
        }
        UpdateAction::Progress => {
            let s = firmware::manual_status(client).await?;
            show(w, fmt, &s, &rows::manual_rows(&s))
        }
    }
}

async fn restore_cmd(
    client: &UgosClient,
    a: &crate::cli::RestoreArgs,
    w: &mut impl Write,
) -> Result<()> {
    let _ = backup::restore(client, &a.file, &a.services, a.force)
        .await
        .map_err(with_force_hint)?;
    writeln!(
        w,
        "The restore started; follow it with `backup progress --restore`"
    )?;
    Ok(())
}

fn reset_message(w: &mut impl Write, what: &str, addresses: &[String]) -> Result<()> {
    writeln!(w, "{what}")?;
    if !addresses.is_empty() {
        writeln!(w, "The NAS may now be at: {}", addresses.join(", "))?;
    }
    Ok(())
}

/// Run a `backup` subcommand.
pub async fn backup(
    client: &UgosClient,
    action: &BackupAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    match action {
        BackupAction::Save { file } => {
            let size = backup::save_local(client, file).await?;
            writeln!(w, "Saved {size} bytes to {}", file.display())?;
            Ok(())
        }
        BackupAction::Info => {
            let info = backup::cloud_info(client).await?;
            let scheduled = backup::cloud_scheduled(client).await?;
            show(w, fmt, &info, &rows::backup_rows(&info, scheduled))
        }
        BackupAction::Services => {
            let list: Vec<RestoreService> = backup::services(client).await?;
            let table: Vec<rows::RestoreRow> = list.iter().map(Into::into).collect();
            output::print_list(w, &table, fmt)
        }
        BackupAction::Restore(a) => restore_cmd(client, a, w).await,
        BackupAction::Progress { restore } => {
            let p = backup::progress(client, *restore).await?;
            show(w, fmt, &p, &rows::progress_rows(p.as_ref()))
        }
        BackupAction::ResetNetwork(ForceArgs { force }) => {
            let ips = backup::reset_network(client, *force)
                .await
                .map_err(with_force_hint)?;
            reset_message(w, "The network settings are reset", &ips)
        }
        BackupAction::ResetFactory(ForceArgs { force }) => {
            let ips = backup::reset_factory(client, *force)
                .await
                .map_err(with_force_hint)?;
            reset_message(w, "The factory reset started", &ips)
        }
    }
}

/// Run an `index` subcommand.
pub async fn index(
    client: &UgosClient,
    action: &IndexAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    match action {
        IndexAction::Status => {
            let t = index::status(client).await?;
            show(w, fmt, &t, &rows::thumb_rows(&t))
        }
        IndexAction::Ranges => {
            let list = index::ranges(client).await?;
            let table: Vec<rows::RangeRow> = list.list.iter().map(Into::into).collect();
            output::print_list(w, &table, fmt)
        }
        IndexAction::Pause { hours } => {
            let t = index::pause(client, *hours).await?;
            show(w, fmt, &t, &rows::thumb_rows(&t))
        }
        IndexAction::Resume => {
            let t = index::resume(client).await?;
            show(w, fmt, &t, &rows::thumb_rows(&t))
        }
        IndexAction::Regenerate {
            all,
            range,
            missing_only,
        } => {
            let t = index::regenerate(client, *all, *missing_only, range).await?;
            show(w, fmt, &t, &rows::thumb_rows(&t))
        }
    }
}

/// `system about`.
pub async fn about_cmd(client: &UgosClient, fmt: OutputFormat, w: &mut impl Write) -> Result<()> {
    let machine = about::machine(client).await?;
    let (_, end) = about::repair(client).await?;
    show(w, fmt, &machine, &rows::about_rows(&machine, end))
}

/// `system ports`.
pub async fn ports_cmd(client: &UgosClient, fmt: OutputFormat, w: &mut impl Write) -> Result<()> {
    let list = about::ports(client).await?;
    let table: Vec<rows::PortRow> = list.iter().map(Into::into).collect();
    output::print_list(w, &table, fmt)
}

/// `system reboot` and `system poweroff`.
pub async fn power_cmd(
    client: &UgosClient,
    off: bool,
    force: bool,
    w: &mut impl Write,
) -> Result<()> {
    if !force {
        return Err(with_force_hint(ugos_client::error::UgosError::NeedsForce(
            if off {
                "this powers the NAS off"
            } else {
                "this restarts the NAS"
            }
            .into(),
        )));
    }
    if off {
        poweroff(client).await?;
        writeln!(w, "The NAS powers off")?;
    } else {
        reboot(client).await?;
        writeln!(w, "The NAS restarts")?;
    }
    Ok(())
}
