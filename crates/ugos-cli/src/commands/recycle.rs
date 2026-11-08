//! `recycle`: the recycle bins of shared folders and user folders.

use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Result, bail};
use ugos_client::UgosClient;
use ugos_client::ops::hw::parse::unix_utc;
use ugos_client::ops::recycle::plan::{self, Kind, PlanSpec};
use ugos_client::ops::recycle::{self, Place};

use super::hw::show;
use super::share::with_force_hint;
use crate::cli::{OnOff, OutputFormat, PlanAction, PlanArgs, PlanUnit, RecycleAction, RecycleRoot};
use crate::output;
use crate::output::recycle as rows;

const fn place(root: RecycleRoot) -> Place {
    match root {
        RecycleRoot::Personal => Place::Personal,
        RecycleRoot::Common => Place::Common,
        RecycleRoot::Users => Place::Users,
    }
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_secs()).ok())
        .unwrap_or(0)
}

async fn plan_spec(client: &UgosClient, a: &PlanArgs) -> Result<PlanSpec> {
    let kind = match (a.retention_days, a.every, &a.first_run) {
        (Some(days), None, None) => Kind::Retention { days },
        (None, Some(every), Some(first)) => Kind::Timed {
            first_run: unix_utc(first)?,
            every,
            unit: match a.unit {
                Some(PlanUnit::Months) => 5,
                _ => 4,
            },
        },
        _ => bail!("give --retention-days, or --every with --unit and --first-run"),
    };
    let mut shares = Vec::new();
    for name in &a.share {
        shares.push(recycle::find(client, name).await?.share_id);
    }
    let mut users = Vec::new();
    for name in &a.user {
        users.push(recycle::find(client, name).await?.user_id);
    }
    Ok(PlanSpec {
        name: a.name.clone(),
        kind,
        time: a.time.clone(),
        all: a.all,
        shares,
        users,
    })
}

async fn plan_cmd(
    client: &UgosClient,
    action: &PlanAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    match action {
        PlanAction::List => {
            let list = plan::list(client).await?;
            let table: Vec<rows::PlanRow> = list.iter().map(Into::into).collect();
            output::print_list(w, &table, fmt)
        }
        PlanAction::Choices => {
            let c = plan::choices(client).await?;
            let mut table: Vec<rows::ChoiceRow> = c
                .share
                .iter()
                .map(|t| rows::ChoiceRow::new("share", t))
                .collect();
            table.extend(c.user.iter().map(|t| rows::ChoiceRow::new("user", t)));
            output::print_list(w, &table, fmt)
        }
        PlanAction::Add(a) => {
            let p = plan::add(client, &plan_spec(client, a).await?, now()).await?;
            output::print_list(w, &[rows::PlanRow::from(&p)], fmt)
        }
        PlanAction::Edit { id, plan: a } => {
            let p = plan::edit(client, *id, &plan_spec(client, a).await?, now()).await?;
            output::print_list(w, &[rows::PlanRow::from(&p)], fmt)
        }
        PlanAction::Remove { id } => {
            plan::remove(client, *id).await?;
            writeln!(w, "The plan {id} is deleted")?;
            Ok(())
        }
    }
}

async fn files(
    client: &UgosClient,
    place: &Place,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    let list = recycle::list(client, place).await?;
    let table: Vec<rows::FileRow> = list.iter().map(Into::into).collect();
    output::print_list(w, &table, fmt)
}

async fn settings_cmd(
    client: &UgosClient,
    action: &RecycleAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    match action {
        RecycleAction::Config { desktop, folder } => {
            let c = recycle::set_conf(client, desktop.map(OnOff::flag), folder.map(OnOff::flag))
                .await?;
            show(w, fmt, &c, &rows::conf_rows(&c))
        }
        RecycleAction::List => {
            let all = recycle::bins(client).await?;
            let table: Vec<rows::BinRow> = all
                .recycle_shares
                .iter()
                .chain(&all.recycle_users)
                .map(Into::into)
                .collect();
            output::print_list(w, &table, fmt)
        }
        RecycleAction::Enable { name, admin_only } => {
            let b = recycle::switch(client, name, true, admin_only.map(OnOff::flag), false).await?;
            output::print_list(w, &[rows::BinRow::from(&b)], fmt)
        }
        RecycleAction::Disable { name, force } => {
            let b = recycle::switch(client, name, false, None, *force)
                .await
                .map_err(with_force_hint)?;
            output::print_list(w, &[rows::BinRow::from(&b)], fmt)
        }
        _ => {
            let c = recycle::conf(client).await?;
            show(w, fmt, &c, &rows::conf_rows(&c))
        }
    }
}

async fn content_cmd(
    client: &UgosClient,
    action: &RecycleAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    match action {
        RecycleAction::Ls { path, root } => {
            let at = match (path, root) {
                (Some(p), _) => Place::Path(p.clone()),
                (None, Some(r)) => place(*r),
                (None, None) => Place::Common,
            };
            files(client, &at, fmt, w).await
        }
        RecycleAction::Restore { paths } => {
            recycle::restore(client, paths).await?;
            writeln!(w, "Restored to where the files were deleted from")?;
            Ok(())
        }
        RecycleAction::Empty { paths, root, force } => {
            let at = root.map(place);
            recycle::empty(client, paths, at.as_ref(), *force)
                .await
                .map_err(with_force_hint)?;
            writeln!(w, "Emptying started")?;
            Ok(())
        }
        RecycleAction::Delete { paths, force } => {
            recycle::delete_forever(client, paths, *force)
                .await
                .map_err(with_force_hint)?;
            writeln!(w, "Deleted for good")?;
            Ok(())
        }
        _ => Ok(()),
    }
}

/// Run a `recycle` subcommand.
pub async fn recycle(
    client: &UgosClient,
    action: &RecycleAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    match action {
        RecycleAction::Ls { .. }
        | RecycleAction::Restore { .. }
        | RecycleAction::Empty { .. }
        | RecycleAction::Delete { .. } => content_cmd(client, action, fmt, w).await,
        RecycleAction::Plan { action } => plan_cmd(client, action, fmt, w).await,
        _ => settings_cmd(client, action, fmt, w).await,
    }
}
