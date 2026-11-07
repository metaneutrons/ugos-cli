//! `hw power` and `hw ups`.

use std::io::Write;

use anyhow::{Result, bail};
use ugos_client::UgosClient;
use ugos_client::ops::hw::parse;
use ugos_client::ops::hw::power::{
    PowerChange, add_switch, power as read_power, remove_switch, set_power, switch,
};
use ugos_client::ops::hw::ups::{
    Source, UpsSetting, check_master, disconnect, firmware, mibs, set_ups, ups,
};

use super::hw::show;
use crate::cli::{
    OnOff, OutputFormat, PowerAction, PowerSetArgs, ScheduleAction, UpsAction, UpsActionKind,
    UpsSetArgs, Wake,
};
use crate::output;
use crate::output::hw as rows;

fn flag(value: Option<OnOff>) -> Option<bool> {
    value.map(OnOff::flag)
}

fn power_change(args: &PowerSetArgs) -> Result<PowerChange> {
    Ok(PowerChange {
        power_boot: flag(args.auto_on),
        wake_on: flag(args.wol),
        sleep: args.sleep.as_deref().map(parse::sleep).transpose()?,
        usb_sleep: args.usb_sleep.as_deref().map(parse::sleep).transpose()?,
        strategy: args.wake.map(|w| if matches!(w, Wake::Rw) { 2 } else { 1 }),
        sleep_log: flag(args.sleep_log),
        schedule: flag(args.schedule),
    })
}

/// Run a `hw power` subcommand.
pub async fn power(
    client: &UgosClient,
    action: &PowerAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    let now = match action {
        PowerAction::Show => power_show(client).await?,
        PowerAction::Set(args) => set_power(client, &power_change(args)?).await?,
        PowerAction::Schedule {
            action: ScheduleAction::Add { days, time, action },
        } => {
            let kind = if action.flag() { "open" } else { "close" };
            add_switch(client, switch(&parse::days(days)?, time, kind)?).await?
        }
        PowerAction::Schedule {
            action: ScheduleAction::Rm { position },
        } => remove_switch(client, *position).await?,
    };
    show(w, fmt, &now, &rows::power_rows(&now))
}

async fn power_show(client: &UgosClient) -> Result<ugos_client::types::hw::Power> {
    Ok(read_power(client).await?)
}

fn ups_setting(args: &UpsSetArgs) -> Result<UpsSetting> {
    let source = match (&args.snmp, &args.network, args.usb) {
        (Some(ip), None, false) => Source::Snmp {
            ip: ip.clone(),
            community: args.community.clone().unwrap_or_default(),
            version: args.snmp_version,
            mib: args.mib.clone(),
        },
        (None, Some(master_ip), false) => Source::Network {
            master_ip: master_ip.clone(),
        },
        (None, None, true) => Source::Usb {
            nut_server: args.nut_server,
        },
        _ => bail!("give one of --usb, --snmp and --network"),
    };
    Ok(UpsSetting {
        source,
        protect_type: i64::from(matches!(args.action, UpsActionKind::Shutdown)),
        wait: parse::wait(&args.wait)?,
    })
}

/// Run a `hw ups` subcommand.
pub async fn ups_cmd(
    client: &UgosClient,
    action: &UpsAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    match action {
        UpsAction::Show => {
            let now = ups(client).await?;
            show(w, fmt, &now, &rows::ups_rows(&now))
        }
        UpsAction::Set(args) => {
            let now = set_ups(client, &ups_setting(args)?).await?;
            show(w, fmt, &now, &rows::ups_rows(&now))
        }
        UpsAction::Disconnect => {
            disconnect(client).await?;
            output::print_success(w, "The UPS is disconnected", fmt)
        }
        UpsAction::Mibs => output::print_json(w, &mibs(client).await?),
        UpsAction::Firmware => output::print_json(w, &firmware(client).await?),
        UpsAction::Check { ip } => {
            check_master(client, ip).await?;
            output::print_success(w, &format!("A NUT server answers at {ip}"), fmt)
        }
    }
}
