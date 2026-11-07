//! `group list`.

use std::io::Write;

use anyhow::Result;
use ugos_client::UgosClient;
use ugos_client::api::users::UsersApi;

use crate::cli::{GroupAction, OutputFormat};
use crate::output;

/// Run a `group` subcommand.
pub async fn group(
    client: &UgosClient,
    action: &GroupAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    match action {
        GroupAction::List => {
            let groups = client.group_list().await?;
            match fmt {
                OutputFormat::Table => {
                    let rows: Vec<output::GroupRow> = groups.iter().map(Into::into).collect();
                    output::print_list(w, &rows, fmt)
                }
                OutputFormat::Json => output::print_json(w, &groups),
            }
        }
    }
}
