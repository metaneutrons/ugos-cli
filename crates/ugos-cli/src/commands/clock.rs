//! `time`, `lang`, `terminal`, and the host name and finder of `system`.

use std::io::Write;

use anyhow::{Result, anyhow};
use ugos_client::UgosClient;
use ugos_client::ops::hw::clock::{
    ClockMode, TimeChange, add_server, finder, language, languages, remove_server, servers,
    set_finder, set_hostname, set_language, set_time, time,
};
use ugos_client::ops::hw::parse;
use ugos_client::ops::hw::terminal::{TerminalChange, set_terminal, terminal};

use super::hw::show;
use super::share::with_force_hint;
use crate::cli::{
    LangAction, OnOff, OutputFormat, ServerAction, TerminalAction, TerminalSetArgs, TimeAction,
};
use crate::output;
use crate::output::hw as rows;

/// Run a `time` subcommand.
pub async fn time_cmd(
    client: &UgosClient,
    action: &TimeAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    match action {
        TimeAction::Show => {
            let now = time(client).await?;
            show(w, fmt, &now, &rows::time_rows(&now))
        }
        TimeAction::Zones => {
            let zones = time(client).await?.zones;
            match fmt {
                OutputFormat::Table => {
                    let table: Vec<rows::ZoneRow> = zones.iter().map(Into::into).collect();
                    output::print_list(w, &table, fmt)
                }
                OutputFormat::Json => output::print_json(w, &zones),
            }
        }
        TimeAction::Set {
            timezone,
            server,
            manual_utc,
            force,
        } => {
            let mode = match (server, manual_utc) {
                (Some(host), None) => Some(ClockMode::Server(host.clone())),
                (None, Some(text)) => Some(ClockMode::Manual(parse::unix_utc(text)?)),
                _ => None,
            };
            let change = TimeChange {
                timezone: timezone.clone(),
                mode,
            };
            let now = set_time(client, &change, *force)
                .await
                .map_err(with_force_hint)?;
            show(w, fmt, &now, &rows::time_rows(&now))
        }
        TimeAction::Server { action } => server_cmd(client, action, fmt, w).await,
    }
}

async fn server_cmd(
    client: &UgosClient,
    action: &ServerAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    let list = match action {
        ServerAction::List => servers(client).await?,
        ServerAction::Add { host } => add_server(client, host).await?,
        ServerAction::Rm { host } => remove_server(client, host).await?,
    };
    match fmt {
        OutputFormat::Table => {
            for host in &list {
                writeln!(w, "{host}")?;
            }
            Ok(())
        }
        OutputFormat::Json => output::print_json(w, &list),
    }
}

/// Run a `lang` subcommand.
pub async fn lang_cmd(
    client: &UgosClient,
    action: &LangAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    match action {
        LangAction::Show => output::print_success(w, &language(client).await?, fmt),
        LangAction::List => {
            let list = languages(client).await?;
            match fmt {
                OutputFormat::Table => {
                    let table: Vec<rows::LanguageRow> = list.iter().map(Into::into).collect();
                    output::print_list(w, &table, fmt)
                }
                OutputFormat::Json => output::print_json(w, &list),
            }
        }
        LangAction::Set { key } => {
            let now = set_language(client, key).await?;
            output::print_success(w, &format!("The default language is {now}"), fmt)
        }
    }
}

fn terminal_change(args: &TerminalSetArgs) -> TerminalChange {
    let on = |v: Option<OnOff>| v.map(OnOff::flag);
    TerminalChange {
        ssh: on(args.ssh),
        ssh_port: args.ssh_port,
        sftp: on(args.sftp),
        ssh_encryption: args.ssh_encryption,
        ssh_duration: args.ssh_duration,
        telnet: on(args.telnet),
        telnet_port: args.telnet_port,
        lan_only: on(args.lan_only),
    }
}

/// Run a `terminal` subcommand.
pub async fn terminal_cmd(
    client: &UgosClient,
    action: &TerminalAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    let now = match action {
        TerminalAction::Show => terminal(client).await?,
        TerminalAction::Set(args) => set_terminal(client, &terminal_change(args), args.force)
            .await
            .map_err(with_force_hint)?,
    };
    let table = rows::terminal_rows(&now);
    match fmt {
        OutputFormat::Table => output::print_list(w, &table, fmt),
        OutputFormat::Json => output::print_json(
            w,
            &serde_json::json!({ "ssh": now.ssh, "telnet": now.telnet }),
        ),
    }
}

/// `system hostname [NAME]`.
pub async fn hostname(
    client: &UgosClient,
    name: Option<&str>,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    let shown = if let Some(name) = name {
        set_hostname(client, name).await?
    } else {
        let info: serde_json::Value = client.get("sysinfo/machine/common").await?;
        info["common"]["nas_name"]
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| anyhow!("UGOS gave no name"))?
    };
    output::print_success(w, &shown, fmt)
}

/// `system finder`.
pub async fn finder_cmd(
    client: &UgosClient,
    improvement: Option<OnOff>,
    web_find: Option<OnOff>,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    let now = if improvement.is_none() && web_find.is_none() {
        finder(client).await?
    } else {
        set_finder(
            client,
            improvement.map(OnOff::flag),
            web_find.map(OnOff::flag),
        )
        .await?
    };
    show(w, fmt, &now, &rows::finder_rows(&now))
}
