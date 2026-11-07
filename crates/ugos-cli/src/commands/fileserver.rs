//! The file services: `smb`, `ftp`, `nfs`, `rsync`, `webdav`.

use std::io::Write;

use anyhow::Result;
use ugos_client::UgosClient;
use ugos_client::ops::fileserver::advanced::{self, AdvancedChange};
use ugos_client::ops::fileserver::fields::split_setting;
use ugos_client::ops::fileserver::smb;
use ugos_client::ops::fileserver::{Change, Config, Service, apply, config, stop};

use super::secret;
use super::share::with_force_hint;
use crate::cli::{
    ApplyArgs, DiscoveryArgs, OutputFormat, RsyncAction, RsyncArgs, ServiceAction, SmbAction,
    StopArgs,
};
use crate::output;
use crate::output::fileserver as rows;

fn show(w: &mut impl Write, fmt: OutputFormat, service: Service, got: &Config) -> Result<()> {
    match fmt {
        OutputFormat::Table => output::print_list(w, &rows::config_rows(service, got), fmt),
        OutputFormat::Json => output::print_json(w, got),
    }
}

fn change(args: &ApplyArgs, password: Option<String>) -> Result<Change> {
    let settings = args
        .settings
        .iter()
        .map(|s| split_setting(s))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Change {
        settings,
        password,
        force: args.force,
    })
}

async fn run(
    client: &UgosClient,
    service: Service,
    enable: bool,
    change: &Change,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    let got = apply(client, service, enable, change)
        .await
        .map_err(with_force_hint)?;
    show(w, fmt, service, &got)
}

async fn stopped(
    client: &UgosClient,
    service: Service,
    args: &StopArgs,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    let got = stop(client, service, args.force)
        .await
        .map_err(with_force_hint)?;
    show(w, fmt, service, &got)
}

fn settings(w: &mut impl Write, fmt: OutputFormat, service: Service) -> Result<()> {
    output::print_list(w, &rows::setting_rows(service), fmt)
}

/// Run a subcommand of `ftp`, `nfs` or `webdav`.
pub async fn service(
    client: &UgosClient,
    service: Service,
    action: &ServiceAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    match action {
        ServiceAction::Status => show(w, fmt, service, &config(client, service).await?),
        ServiceAction::Settings => settings(w, fmt, service),
        ServiceAction::Start(a) => run(client, service, true, &change(a, None)?, fmt, w).await,
        ServiceAction::Set(a) => run(client, service, false, &change(a, None)?, fmt, w).await,
        ServiceAction::Stop(a) => stopped(client, service, a, fmt, w).await,
    }
}

fn rsync_change(args: &RsyncArgs, needs_password: bool) -> Result<Change> {
    let password = if needs_password {
        Some(secret::read(
            "rsync password",
            "UGOS_RSYNC_PASSWORD",
            "--password-stdin",
            args.password_stdin,
        )?)
    } else {
        None
    };
    change(&args.apply, password)
}

/// Whether the settings name an account, which is the case that needs the
/// password.
fn names_account(args: &RsyncArgs) -> bool {
    args.password_stdin
        || args
            .apply
            .settings
            .iter()
            .any(|s| s.trim_start().starts_with("account="))
        || std::env::var_os("UGOS_RSYNC_PASSWORD").is_some()
}

/// Run an `rsync` subcommand.
pub async fn rsync(
    client: &UgosClient,
    action: &RsyncAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    let service = Service::Rsync;
    match action {
        RsyncAction::Status => show(w, fmt, service, &config(client, service).await?),
        RsyncAction::Settings => settings(w, fmt, service),
        RsyncAction::Start(a) => {
            run(
                client,
                service,
                true,
                &rsync_change(a, names_account(a))?,
                fmt,
                w,
            )
            .await
        }
        RsyncAction::Set(a) => {
            run(
                client,
                service,
                false,
                &rsync_change(a, names_account(a))?,
                fmt,
                w,
            )
            .await
        }
        RsyncAction::Stop(a) => stopped(client, service, a, fmt, w).await,
    }
}

async fn discovery(
    client: &UgosClient,
    args: &DiscoveryArgs,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    let time_machine = args.time_machine.as_ref().map(|names| {
        names
            .iter()
            .filter(|n| !n.eq_ignore_ascii_case("none"))
            .cloned()
            .collect::<Vec<_>>()
    });
    let change = AdvancedChange {
        wsdd2: args.wsdd2.map(crate::cli::OnOff::flag),
        bonjour: args.bonjour.map(crate::cli::OnOff::flag),
        upnp: args.upnp.map(crate::cli::OnOff::flag),
        time_machine,
    };
    let got = if change == AdvancedChange::default() {
        advanced::get(client).await?
    } else {
        advanced::apply(client, &change, args.force)
            .await
            .map_err(with_force_hint)?
    };
    match fmt {
        OutputFormat::Table => output::print_list(w, &rows::advanced_rows(&got), fmt),
        OutputFormat::Json => output::print_json(w, &got),
    }
}

/// Run an `smb` subcommand.
pub async fn smb(
    client: &UgosClient,
    action: &SmbAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    let service = Service::Smb;
    match action {
        SmbAction::Status => {
            show(w, fmt, service, &config(client, service).await?)?;
            if matches!(fmt, OutputFormat::Table) {
                writeln!(w)?;
                output::print_list(w, &rows::advanced_rows(&advanced::get(client).await?), fmt)?;
            }
            Ok(())
        }
        SmbAction::Settings => settings(w, fmt, service),
        SmbAction::Start(a) => run(client, service, true, &change(a, None)?, fmt, w).await,
        SmbAction::Set(a) => run(client, service, false, &change(a, None)?, fmt, w).await,
        SmbAction::Stop(a) => stopped(client, service, a, fmt, w).await,
        SmbAction::Discovery(a) => discovery(client, a, fmt, w).await,
        SmbAction::ClearCache(a) => {
            smb::clear_cache(client, a.force)
                .await
                .map_err(with_force_hint)?;
            writeln!(w, "The SMB cache is cleared; the network service restarts")?;
            Ok(())
        }
    }
}
