//! `docker network-ls`, `network-show`, `updates` and `updates-set`.

use std::io::Write;

use anyhow::{Result, bail};
use ugos_client::UgosClient;
use ugos_client::ops::dockernet;
use ugos_client::ops::dockerupdates::{self, ConfigChange};

use crate::cli::{DockerAction, OnOff, OutputFormat};
use crate::output;
use crate::output::docker::{MemberRow, NetworkRow, network_rows, update_rows};

/// Run one of the network and update commands.
pub async fn run(
    client: &UgosClient,
    action: &DockerAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    match action {
        DockerAction::NetworkLs => {
            let rows: Vec<NetworkRow> = dockernet::list(client)
                .await?
                .iter()
                .map(Into::into)
                .collect();
            output::print_list(w, &rows, fmt)
        }
        DockerAction::NetworkShow { name } => {
            let detail = dockernet::show(client, name).await?;
            if fmt == OutputFormat::Json {
                return output::print_json(w, &detail);
            }
            output::print_list(w, &network_rows(&detail.network), fmt)?;
            let members: Vec<MemberRow> = detail.members.iter().map(Into::into).collect();
            if members.is_empty() {
                writeln!(w, "No containers attached.")?;
            } else {
                writeln!(w)?;
                output::print_list(w, &members, fmt)?;
            }
            Ok(())
        }
        DockerAction::Updates => {
            let status = dockerupdates::status(client).await?;
            match fmt {
                OutputFormat::Table => output::print_list(w, &update_rows(&status), fmt),
                OutputFormat::Json => output::print_json(w, &status),
            }
        }
        DockerAction::UpdatesSet {
            reminder,
            plan,
            time,
        } => {
            let change = ConfigChange {
                reminder: reminder.map(OnOff::flag),
                plan: plan.map(OnOff::flag),
                time: time.clone(),
            };
            let now = dockerupdates::set_config(client, &change).await?;
            match fmt {
                OutputFormat::Table => {
                    writeln!(w, "Updated the image update settings")?;
                    let status = dockerupdates::status(client).await?;
                    output::print_list(w, &update_rows(&status), fmt)
                }
                OutputFormat::Json => output::print_json(w, &now),
            }
        }
        _ => bail!("not a network or update command"),
    }
}
