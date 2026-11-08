//! SAN Manager: the logic behind the `ugos_san_*` tools.
//!
//! The same operations as `ugos-cli`. What cuts a connection, destroys data
//! or cannot be undone needs `force`, and the model is told to ask the user
//! first. CHAP passwords are parameters here: gate or switch off the tools if
//! a model should not handle them.

use rmcp::schemars;
use serde::Serialize;
use ugos_client::UgosClient;
use ugos_client::error::UgosError;
use ugos_client::ops::san::check;
use ugos_client::ops::san::group::{self, GroupChange};
use ugos_client::ops::san::lun::{self, Access, LunChange, LunSpec};
use ugos_client::ops::san::setting::{self, Chap, SettingChange};
use ugos_client::ops::san::target::{self, Protection, TargetChange, TargetSpec};

use crate::manage::failure;

fn json<T: Serialize>(result: Result<T, UgosError>) -> String {
    match result {
        Ok(value) => serde_json::to_string_pretty(&value).unwrap_or_default(),
        Err(e) => failure(&e),
    }
}

fn done(result: Result<(), UgosError>, text: &str) -> String {
    result.map_or_else(|e| failure(&e), |()| text.to_owned())
}

fn refused(message: &str) -> UgosError {
    UgosError::Refused(message.into())
}

/// CHAP.
#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
pub struct ChapParam {
    /// `off`, `default` (targets only), `one_way` or `mutual`.
    pub mode: String,
    /// CHAP user, letters and digits, up to 12.
    #[serde(default)]
    pub user: Option<String>,
    /// CHAP password, 12 to 16 letters and digits.
    #[serde(default)]
    pub password: Option<String>,
    /// The target's CHAP user, for mutual CHAP.
    #[serde(default)]
    pub mutual_user: Option<String>,
    /// The target's CHAP password, different from the first.
    #[serde(default)]
    pub mutual_password: Option<String>,
}

fn credentials(p: &ChapParam) -> Result<Chap, UgosError> {
    let get = |v: &Option<String>, what: &str| {
        v.clone()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| refused(&format!("{what} is needed")))
    };
    match p.mode.as_str() {
        "off" => Ok(Chap::Off),
        "one_way" => Ok(Chap::OneWay {
            user: get(&p.user, "user")?,
            password: get(&p.password, "password")?,
        }),
        "mutual" => Ok(Chap::Mutual {
            user: get(&p.user, "user")?,
            password: get(&p.password, "password")?,
            mutual_user: get(&p.mutual_user, "mutual_user")?,
            mutual_password: get(&p.mutual_password, "mutual_password")?,
        }),
        _ => Err(refused("the CHAP mode is off, one_way or mutual here")),
    }
}

fn protection(p: &ChapParam) -> Result<Protection, UgosError> {
    match p.mode.as_str() {
        "default" => Ok(Protection::Default),
        "off" => Ok(Protection::None),
        _ => Ok(Protection::Custom(credentials(p)?)),
    }
}

/// A target, LUN or group by name or id.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SanNameParam {
    /// Name or id.
    pub name: String,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// Delete something.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SanDeleteParam {
    /// Name or id.
    pub name: String,
    /// Targets: delete the mapped LUN and its data as well.
    #[serde(default)]
    pub with_lun: bool,
    /// Go on although data is destroyed or connections are cut. Ask the user
    /// first.
    #[serde(default)]
    pub force: bool,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// Disable a target, or list its connections.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SanTargetParam {
    /// Name or id. For connections, empty means all targets.
    #[serde(default)]
    pub name: String,
    /// Go on although initiators are connected. Ask the user first.
    #[serde(default)]
    pub force: bool,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// The global setting.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SanSettingParam {
    /// The iSCSI port, 1001 to 65535.
    #[serde(default)]
    pub port: Option<i64>,
    /// The start of every IQN.
    #[serde(default)]
    pub iqn_prefix: Option<String>,
    /// The default CHAP, `off`, `one_way` or `mutual`.
    #[serde(default)]
    pub chap: Option<ChapParam>,
    /// Go on although a new port restarts the service or credentials change.
    /// Ask the user first.
    #[serde(default)]
    pub force: bool,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// Make a target.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TargetCreateParam {
    /// Name, letters, digits, `_` and `-`, up to 32.
    pub name: String,
    /// The LUN to map, by name or id; it must not be mapped yet.
    pub lun: String,
    /// The IQN; the next free one when not given.
    #[serde(default)]
    pub iqn: Option<String>,
    /// CHAP; none when not given.
    #[serde(default)]
    pub chap: Option<ChapParam>,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// Change a target.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TargetEditParam {
    /// The target, by name or id.
    pub name: String,
    /// A new name.
    #[serde(default)]
    pub new_name: Option<String>,
    /// A new IQN.
    #[serde(default)]
    pub iqn: Option<String>,
    /// Several connections per initiator.
    #[serde(default)]
    pub multi_conn: Option<bool>,
    /// New CHAP.
    #[serde(default)]
    pub chap: Option<ChapParam>,
    /// Header digest.
    #[serde(default)]
    pub head_crc: Option<bool>,
    /// Data digest.
    #[serde(default)]
    pub data_crc: Option<bool>,
    /// Apply CHAP and digest changes to connected initiators now, dropping
    /// their sessions.
    #[serde(default)]
    pub apply_now: bool,
    /// Go on although initiators are connected. Ask the user first.
    #[serde(default)]
    pub force: bool,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// Make a LUN.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct LunCreateParam {
    /// Name, letters, digits, `_` and `-`, up to 32.
    pub name: String,
    /// Size in whole GB or TB, like `100G` or `2T`.
    pub size: String,
    /// The volume, by name or label.
    pub volume: String,
    /// A thin LUN, which takes space as it is written. Else thick.
    #[serde(default)]
    pub thin: bool,
    /// A description, up to 32 characters.
    #[serde(default)]
    pub desc: String,
    /// Go on although a thin LUN is larger than the free space. Ask the user
    /// first.
    #[serde(default)]
    pub force: bool,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// Change or enlarge a LUN.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct LunEditParam {
    /// The LUN, by name or id.
    pub name: String,
    /// A new name.
    #[serde(default)]
    pub new_name: Option<String>,
    /// A new description.
    #[serde(default)]
    pub desc: Option<String>,
    /// A larger size, like `200G`. It cannot be made smaller again.
    #[serde(default)]
    pub size: Option<String>,
    /// Force unit access.
    #[serde(default)]
    pub fua: Option<bool>,
    /// `all` groups, or `groups` for the ones in `groups`.
    #[serde(default)]
    pub access: Option<String>,
    /// Groups with access, as `name` (read-write) or `name=ro`.
    #[serde(default)]
    pub groups: Vec<String>,
    /// Go on although a size or access change cannot be undone. Ask the user
    /// first.
    #[serde(default)]
    pub force: bool,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// Make a group.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GroupCreateParam {
    /// Name, letters, digits, `_` and `-`, up to 32.
    pub name: String,
    /// Initiator IQNs, at least one.
    pub iqns: Vec<String>,
    /// A description, up to 32 characters.
    #[serde(default)]
    pub desc: String,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// Change a group.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GroupEditParam {
    /// Name or id.
    pub name: String,
    /// A new name.
    #[serde(default)]
    pub new_name: Option<String>,
    /// A new description.
    #[serde(default)]
    pub desc: Option<String>,
    /// Initiator IQNs to add.
    #[serde(default)]
    pub add: Vec<String>,
    /// Initiator IQNs to remove.
    #[serde(default)]
    pub remove: Vec<String>,
    /// Go on although LUNs are bound to the group. Ask the user first.
    #[serde(default)]
    pub force: bool,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

pub async fn setting_show(c: &UgosClient) -> String {
    json(setting::get(c).await)
}

pub async fn setting_set(c: &UgosClient, p: &SanSettingParam) -> String {
    let run = async {
        let chap = p.chap.as_ref().map(credentials).transpose()?;
        let change = SettingChange {
            port: p.port,
            iqn_prefix: p.iqn_prefix.clone(),
            chap,
        };
        setting::set(c, &change, p.force).await
    };
    json(run.await)
}

pub async fn target_list(c: &UgosClient) -> String {
    json(target::list(c).await)
}

pub async fn target_show(c: &UgosClient, p: &SanNameParam) -> String {
    json(target::find(c, &p.name).await)
}

pub async fn target_predict(c: &UgosClient) -> String {
    json(target::predict(c).await)
}

pub async fn target_connections(c: &UgosClient, p: &SanTargetParam) -> String {
    json(target::connections(c, &p.name).await)
}

pub async fn target_create(c: &UgosClient, p: &TargetCreateParam) -> String {
    let run = async {
        let protection = p.chap.as_ref().map_or(Ok(Protection::None), protection)?;
        let spec = TargetSpec {
            name: p.name.clone(),
            iqn: p.iqn.clone(),
            lun: p.lun.clone(),
            protection,
        };
        target::create(c, &spec).await
    };
    json(run.await)
}

pub async fn target_edit(client: &UgosClient, p: &TargetEditParam) -> String {
    let c = client;
    let run = async {
        let crc = match (p.head_crc, p.data_crc) {
            (None, None) => None,
            (head, data) => {
                let found = target::find(c, &p.name).await?;
                Some((
                    head.unwrap_or(found.enable_head_crc),
                    data.unwrap_or(found.enable_data_crc),
                ))
            }
        };
        let change = TargetChange {
            name: p.new_name.clone(),
            iqn: p.iqn.clone(),
            multi_conn: p.multi_conn,
            protection: p.chap.as_ref().map(protection).transpose()?,
            crc,
            apply_now: p.apply_now,
        };
        target::edit(c, &p.name, &change, p.force).await
    };
    json(run.await)
}

pub async fn target_enable(c: &UgosClient, p: &SanNameParam) -> String {
    json(target::enable(c, &p.name).await)
}

pub async fn target_disable(c: &UgosClient, p: &SanTargetParam) -> String {
    json(target::disable(c, &p.name, p.force).await)
}

pub async fn target_delete(c: &UgosClient, p: &SanDeleteParam) -> String {
    done(
        target::delete(c, &p.name, p.with_lun, p.force).await,
        "The target is deleted",
    )
}

pub async fn lun_list(c: &UgosClient) -> String {
    json(lun::list(c).await)
}

pub async fn lun_show(c: &UgosClient, p: &SanNameParam) -> String {
    json(lun::find(c, &p.name).await)
}

pub async fn lun_create(c: &UgosClient, p: &LunCreateParam) -> String {
    let run = async {
        let (size, unit) = check::size(&p.size)?;
        let spec = LunSpec {
            name: p.name.clone(),
            desc: p.desc.clone(),
            size,
            unit,
            volume: p.volume.clone(),
            thin: p.thin,
        };
        lun::create(c, &spec, p.force).await
    };
    json(run.await)
}

fn access(p: &LunEditParam) -> Option<Access> {
    match p.access.as_deref() {
        Some("all") => Some(Access::All),
        None if p.groups.is_empty() => None,
        _ => Some(Access::Groups(
            p.groups
                .iter()
                .map(|g| {
                    let (n, m) = g.split_once('=').unwrap_or((g.as_str(), "rw"));
                    (n.to_owned(), m.to_owned())
                })
                .collect(),
        )),
    }
}

pub async fn lun_edit(c: &UgosClient, p: &LunEditParam) -> String {
    let run = async {
        let change = LunChange {
            name: p.new_name.clone(),
            desc: p.desc.clone(),
            size: p.size.as_deref().map(check::size).transpose()?,
            fua: p.fua,
            access: access(p),
        };
        lun::edit(c, &p.name, &change, p.force).await
    };
    json(run.await)
}

pub async fn lun_delete(c: &UgosClient, p: &SanDeleteParam) -> String {
    done(lun::delete(c, &p.name, p.force).await, "The LUN is deleted")
}

pub async fn group_list(c: &UgosClient) -> String {
    json(group::list(c).await)
}

pub async fn group_create(c: &UgosClient, p: &GroupCreateParam) -> String {
    json(group::create(c, &p.name, &p.desc, &p.iqns).await)
}

pub async fn group_edit(c: &UgosClient, p: &GroupEditParam) -> String {
    let change = GroupChange {
        name: p.new_name.clone(),
        desc: p.desc.clone(),
        add: p.add.clone(),
        remove: p.remove.clone(),
    };
    json(group::edit(c, &p.name, &change, p.force).await)
}

pub async fn group_delete(c: &UgosClient, p: &SanDeleteParam) -> String {
    done(
        group::delete(c, &p.name, p.force).await,
        "The group is deleted",
    )
}
