//! Recycle bins: the logic behind the `ugos_recycle_*` tools.
//!
//! The same operations as `ugos-cli`. Emptying a bin and deleting from it are
//! for good and need `force`, and the model is told to ask the user first.
//! Paths must be inside a `#recycle` folder.

use rmcp::schemars;
use serde::Serialize;
use ugos_client::UgosClient;
use ugos_client::error::UgosError;
use ugos_client::ops::hw::parse::unix_utc;
use ugos_client::ops::recycle::plan::{self, Kind, PlanSpec};
use ugos_client::ops::recycle::{self, Place};

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

/// A bin.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct BinParam {
    /// The share or user, or its folder.
    pub name: String,
    /// Switching off: go on although the bin holds files. Ask the user first.
    #[serde(default)]
    pub force: bool,
    /// Switching on: only administrators see what was deleted (shares).
    #[serde(default)]
    pub admin_only: Option<bool>,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// The switches.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct RecycleConfParam {
    /// A shortcut on the desktop.
    #[serde(default)]
    pub desktop: Option<bool>,
    /// The recycle folder shown inside folders.
    #[serde(default)]
    pub folder: Option<bool>,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// List a bin.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct RecycleLsParam {
    /// A folder inside a bin, like `/volume1/docs/#recycle`.
    #[serde(default)]
    pub path: Option<String>,
    /// Or a root: `personal`, `common` or `users` (common when neither).
    #[serde(default)]
    pub root: Option<String>,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// Paths inside bins.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct RecyclePathsParam {
    /// Paths inside a recycle bin.
    #[serde(default)]
    pub paths: Vec<String>,
    /// Empty only: every bin of a root, `common` or `users`, instead of paths.
    #[serde(default)]
    pub root: Option<String>,
    /// Go on although this deletes for good. Ask the user first.
    #[serde(default)]
    pub force: bool,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

/// A cleanup plan.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PlanParam {
    /// The plan to replace or remove.
    #[serde(default)]
    pub id: Option<i64>,
    /// A label.
    #[serde(default)]
    pub name: String,
    /// Delete what is older than this many days (1 to 365), every day.
    #[serde(default)]
    pub retention_days: Option<i64>,
    /// Empty the bins every this many days or months, with `unit` and
    /// `first_run`.
    #[serde(default)]
    pub every: Option<i64>,
    /// `days` or `months`.
    #[serde(default)]
    pub unit: Option<String>,
    /// First run of a timed plan, `YYYY-MM-DD HH:MM:SS` in UTC.
    #[serde(default)]
    pub first_run: Option<String>,
    /// Time of day of a retention plan, `HH:MM:SS`.
    #[serde(default)]
    pub time: Option<String>,
    /// Cover every bin.
    #[serde(default)]
    pub all: bool,
    /// Shares, by name.
    #[serde(default)]
    pub shares: Vec<String>,
    /// Users, by name.
    #[serde(default)]
    pub users: Vec<String>,
    /// Target NAS name or host. Required when multiple targets are configured.
    #[serde(default)]
    pub target: String,
}

fn root(name: &str) -> Result<Place, UgosError> {
    match name {
        "personal" => Ok(Place::Personal),
        "common" => Ok(Place::Common),
        "users" => Ok(Place::Users),
        _ => Err(refused("the root is personal, common or users")),
    }
}

pub async fn status(c: &UgosClient) -> String {
    json(recycle::conf(c).await)
}

pub async fn config(c: &UgosClient, p: &RecycleConfParam) -> String {
    json(recycle::set_conf(c, p.desktop, p.folder).await)
}

pub async fn list(c: &UgosClient) -> String {
    json(recycle::bins(c).await)
}

pub async fn enable(c: &UgosClient, p: &BinParam) -> String {
    json(recycle::switch(c, &p.name, true, p.admin_only, false).await)
}

pub async fn disable(c: &UgosClient, p: &BinParam) -> String {
    json(recycle::switch(c, &p.name, false, None, p.force).await)
}

pub async fn ls(c: &UgosClient, p: &RecycleLsParam) -> String {
    let run = async {
        let place = match (&p.path, &p.root) {
            (Some(path), _) => Place::Path(path.clone()),
            (None, Some(r)) => root(r)?,
            (None, None) => Place::Common,
        };
        recycle::list(c, &place).await
    };
    json(run.await)
}

pub async fn restore(c: &UgosClient, p: &RecyclePathsParam) -> String {
    done(
        recycle::restore(c, &p.paths).await,
        "Restored to where the files were deleted from",
    )
}

pub async fn empty(c: &UgosClient, p: &RecyclePathsParam) -> String {
    let run = async {
        let at = p.root.as_deref().map(root).transpose()?;
        recycle::empty(c, &p.paths, at.as_ref(), p.force).await
    };
    done(run.await, "Emptying started")
}

pub async fn delete(c: &UgosClient, p: &RecyclePathsParam) -> String {
    done(
        recycle::delete_forever(c, &p.paths, p.force).await,
        "Deleted for good",
    )
}

pub async fn plan_list(c: &UgosClient) -> String {
    json(plan::list(c).await)
}

pub async fn plan_choices(c: &UgosClient) -> String {
    json(plan::choices(c).await)
}

async fn spec(c: &UgosClient, p: &PlanParam) -> Result<PlanSpec, UgosError> {
    let kind = match (p.retention_days, p.every, &p.first_run) {
        (Some(days), None, None) => Kind::Retention { days },
        (None, Some(every), Some(first)) => Kind::Timed {
            first_run: unix_utc(first)?,
            every,
            unit: if p.unit.as_deref() == Some("months") {
                5
            } else {
                4
            },
        },
        _ => {
            return Err(refused(
                "give retention_days, or every with unit and first_run",
            ));
        }
    };
    let mut shares = Vec::new();
    for name in &p.shares {
        shares.push(recycle::find(c, name).await?.share_id);
    }
    let mut users = Vec::new();
    for name in &p.users {
        users.push(recycle::find(c, name).await?.user_id);
    }
    Ok(PlanSpec {
        name: p.name.clone(),
        kind,
        time: p.time.clone().unwrap_or_else(|| "12:00:00".into()),
        all: p.all,
        shares,
        users,
    })
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_secs()).ok())
        .unwrap_or(0)
}

pub async fn plan_add(c: &UgosClient, p: &PlanParam) -> String {
    let run = async { plan::add(c, &spec(c, p).await?, now()).await };
    json(run.await)
}

pub async fn plan_edit(c: &UgosClient, p: &PlanParam) -> String {
    let run = async {
        let id = p.id.ok_or_else(|| refused("name the plan: id"))?;
        plan::edit(c, id, &spec(c, p).await?, now()).await
    };
    json(run.await)
}

pub async fn plan_remove(c: &UgosClient, p: &PlanParam) -> String {
    let run = async {
        let id = p.id.ok_or_else(|| refused("name the plan: id"))?;
        plan::remove(c, id).await
    };
    done(run.await, "The plan is deleted")
}
