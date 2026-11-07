//! `san`: iSCSI targets, LUNs, initiator groups and the global setting.

use std::io::Write;

use anyhow::{Result, bail};
use ugos_client::UgosClient;
use ugos_client::ops::san::check;
use ugos_client::ops::san::group::{self, GroupChange};
use ugos_client::ops::san::lun::{self, Access, LunChange, LunSpec};
use ugos_client::ops::san::setting::{self, Chap, SettingChange};
use ugos_client::ops::san::target::{self, Protection, TargetChange, TargetSpec};

use super::hw::show;
use super::secret;
use super::share::with_force_hint;
use crate::cli::{
    AccessMode, ChapArgs, ChapMode, LunAction, LunCreateArgs, LunEditArgs, OnOff, OutputFormat,
    SanAction, SanGroupAction, SettingAction, TargetAction, TargetCreateArgs, TargetEditArgs,
};
use crate::output;
use crate::output::san as rows;

fn credentials(args: &ChapArgs) -> Result<Chap> {
    let user = args.chap_user.clone().unwrap_or_default();
    let password = |from_stdin: bool| {
        secret::read(
            "CHAP password",
            "UGOS_CHAP_PASSWORD",
            "--chap-password-stdin",
            from_stdin,
        )
    };
    match args.chap {
        Some(ChapMode::OneWay) => {
            if user.is_empty() {
                bail!("one-way CHAP needs --chap-user");
            }
            Ok(Chap::OneWay {
                user,
                password: password(args.chap_password_stdin)?,
            })
        }
        Some(ChapMode::Mutual) => {
            let Some(mutual_user) = args.chap_mutual_user.clone() else {
                bail!("mutual CHAP needs --chap-user and --chap-mutual-user");
            };
            if user.is_empty() {
                bail!("mutual CHAP needs --chap-user and --chap-mutual-user");
            }
            let first = password(args.chap_password_stdin)?;
            let second = secret::read(
                "mutual CHAP password",
                "UGOS_CHAP_MUTUAL_PASSWORD",
                "",
                false,
            )?;
            Ok(Chap::Mutual {
                user,
                password: first,
                mutual_user,
                mutual_password: second,
            })
        }
        _ => Ok(Chap::Off),
    }
}

fn protection(args: &ChapArgs) -> Result<Option<Protection>> {
    Ok(match args.chap {
        None => None,
        Some(ChapMode::Off) => Some(Protection::None),
        Some(ChapMode::Default) => Some(Protection::Default),
        Some(_) => Some(Protection::Custom(credentials(args)?)),
    })
}

async fn setting_cmd(
    client: &UgosClient,
    action: &SettingAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    let got = match action {
        SettingAction::Show => setting::get(client).await?,
        SettingAction::Set(a) => {
            let chap = match a.chap.chap {
                None => None,
                Some(ChapMode::Default) => {
                    bail!("the default CHAP of the setting is off, one-way or mutual")
                }
                Some(_) => Some(credentials(&a.chap)?),
            };
            let change = SettingChange {
                port: a.iscsi_port,
                iqn_prefix: a.iqn_prefix.clone(),
                chap,
            };
            setting::set(client, &change, a.force)
                .await
                .map_err(with_force_hint)?
        }
    };
    show(w, fmt, &got, &rows::setting_rows(&got))
}

async fn target_create(
    client: &UgosClient,
    a: &TargetCreateArgs,
) -> Result<ugos_client::types::san::Target> {
    let spec = TargetSpec {
        name: a.name.clone(),
        iqn: a.iqn.clone(),
        lun: a.lun.clone(),
        protection: protection(&a.chap)?.unwrap_or(Protection::None),
    };
    Ok(target::create(client, &spec).await?)
}

async fn target_edit(
    client: &UgosClient,
    a: &TargetEditArgs,
) -> Result<ugos_client::types::san::Target> {
    let crc = match (a.head_crc, a.data_crc) {
        (None, None) => None,
        (h, d) => {
            let current = target::find(client, &a.name).await?;
            Some((
                h.map_or(current.enable_head_crc, OnOff::flag),
                d.map_or(current.enable_data_crc, OnOff::flag),
            ))
        }
    };
    let change = TargetChange {
        name: a.new_name.clone(),
        iqn: a.iqn.clone(),
        multi_conn: a.multi_conn.map(OnOff::flag),
        protection: protection(&a.chap)?,
        crc,
        apply_now: a.apply_now,
    };
    target::edit(client, &a.name, &change, a.force)
        .await
        .map_err(with_force_hint)
}

async fn target_cmd(
    client: &UgosClient,
    action: &TargetAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    let one = |t: &ugos_client::types::san::Target, w: &mut dyn Write| -> Result<()> {
        let mut w = w;
        show(&mut w, fmt, t, &rows::target_rows(t))
    };
    match action {
        TargetAction::List => {
            let list = target::list(client).await?;
            let table: Vec<rows::TargetRow> = list.iter().map(Into::into).collect();
            output::print_list(w, &table, fmt)
        }
        TargetAction::Show { name } => one(&target::find(client, name).await?, w),
        TargetAction::Predict => {
            let (name, iqn) = target::predict(client).await?;
            writeln!(w, "{name}\n{iqn}")?;
            Ok(())
        }
        TargetAction::Connections { name } => {
            let list = target::connections(client, name.as_deref().unwrap_or("")).await?;
            let table: Vec<rows::ConnectionRow> = list.iter().map(Into::into).collect();
            output::print_list(w, &table, fmt)
        }
        TargetAction::Create(a) => one(&target_create(client, a).await?, w),
        TargetAction::Edit(a) => one(&target_edit(client, a).await?, w),
        TargetAction::Enable { name } => one(&target::enable(client, name).await?, w),
        TargetAction::Disable { name, force } => one(
            &target::disable(client, name, *force)
                .await
                .map_err(with_force_hint)?,
            w,
        ),
        TargetAction::Delete {
            name,
            with_lun,
            force,
        } => {
            target::delete(client, name, *with_lun, *force)
                .await
                .map_err(with_force_hint)?;
            writeln!(w, "The target {name} is deleted")?;
            Ok(())
        }
    }
}

fn group_access(args: &LunEditArgs) -> Option<Access> {
    match args.access {
        None if args.groups.is_empty() => None,
        Some(AccessMode::All) => Some(Access::All),
        _ => {
            let mut out = Vec::new();
            for g in &args.groups {
                let (name, mode) = g.split_once('=').unwrap_or((g.as_str(), "rw"));
                out.push((name.to_owned(), mode.to_owned()));
            }
            Some(Access::Groups(out))
        }
    }
}

async fn lun_create(
    client: &UgosClient,
    a: &LunCreateArgs,
) -> Result<ugos_client::types::san::Lun> {
    let (size, unit) = check::size(&a.size)?;
    let spec = LunSpec {
        name: a.name.clone(),
        desc: a.desc.clone(),
        size,
        unit,
        volume: a.volume.clone(),
        thin: a.thin,
    };
    lun::create(client, &spec, a.force)
        .await
        .map_err(with_force_hint)
}

async fn lun_edit(client: &UgosClient, a: &LunEditArgs) -> Result<ugos_client::types::san::Lun> {
    let change = LunChange {
        name: a.new_name.clone(),
        desc: a.desc.clone(),
        size: a.size.as_deref().map(check::size).transpose()?,
        fua: a.fua.map(OnOff::flag),
        access: group_access(a),
    };
    lun::edit(client, &a.name, &change, a.force)
        .await
        .map_err(with_force_hint)
}

async fn lun_cmd(
    client: &UgosClient,
    action: &LunAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    match action {
        LunAction::List => {
            let list = lun::list(client).await?;
            let table: Vec<rows::LunRow> = list.iter().map(Into::into).collect();
            output::print_list(w, &table, fmt)
        }
        LunAction::Show { name } => {
            let l = lun::find(client, name).await?;
            show(w, fmt, &l, &rows::lun_rows(&l))
        }
        LunAction::Create(a) => {
            let l = lun_create(client, a).await?;
            show(w, fmt, &l, &rows::lun_rows(&l))
        }
        LunAction::Edit(a) => {
            let l = lun_edit(client, a).await?;
            show(w, fmt, &l, &rows::lun_rows(&l))
        }
        LunAction::Delete { name, force } => {
            lun::delete(client, name, *force)
                .await
                .map_err(with_force_hint)?;
            writeln!(w, "The LUN {name} is deleted")?;
            Ok(())
        }
    }
}

async fn group_cmd(
    client: &UgosClient,
    action: &SanGroupAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    match action {
        SanGroupAction::List => {
            let list = group::list(client).await?;
            let table: Vec<rows::GroupRow> = list.iter().map(Into::into).collect();
            output::print_list(w, &table, fmt)
        }
        SanGroupAction::Create { name, iqns, desc } => {
            let g = group::create(client, name, desc, iqns).await?;
            output::print_list(w, &[rows::GroupRow::from(&g)], fmt)
        }
        SanGroupAction::Edit {
            name,
            new_name,
            desc,
            add,
            remove,
            force,
        } => {
            let change = GroupChange {
                name: new_name.clone(),
                desc: desc.clone(),
                add: add.clone(),
                remove: remove.clone(),
            };
            let g = group::edit(client, name, &change, *force)
                .await
                .map_err(with_force_hint)?;
            output::print_list(w, &[rows::GroupRow::from(&g)], fmt)
        }
        SanGroupAction::Delete { name, force } => {
            group::delete(client, name, *force)
                .await
                .map_err(with_force_hint)?;
            writeln!(w, "The group {name} is deleted")?;
            Ok(())
        }
    }
}

/// Run a `san` subcommand.
pub async fn san(
    client: &UgosClient,
    action: &SanAction,
    fmt: OutputFormat,
    w: &mut impl Write,
) -> Result<()> {
    match action {
        SanAction::Setting { action } => setting_cmd(client, action, fmt, w).await,
        SanAction::Target { action } => target_cmd(client, action, fmt, w).await,
        SanAction::Lun { action } => lun_cmd(client, action, fmt, w).await,
        SanAction::Group { action } => group_cmd(client, action, fmt, w).await,
    }
}
