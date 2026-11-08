//! Scheduled cleanup plans of the recycle bins.

use serde_json::{Value, json};

use super::refused;
use crate::client::UgosClient;
use crate::error::Result;
use crate::types::recycle::{Plan, PlanChoices};

/// The plans.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`](crate::error::UgosError) on failure.
pub async fn list(client: &UgosClient) -> Result<Vec<Plan>> {
    let answer: Value = client.get("filemgr/taskPlanList").await?;
    match answer.get("result") {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(list) => Ok(serde_json::from_value(list.clone())?),
    }
}

/// The bins a plan can still take.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`](crate::error::UgosError) on failure.
pub async fn choices(client: &UgosClient) -> Result<PlanChoices> {
    client
        .get_with_params(
            "filemgr/shares",
            &[("is_dynamic", "false"), ("is_timer", "false")],
        )
        .await
}

/// What kind of plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Delete what is older than this many days, every day at `time`.
    Retention {
        /// 1 to 365.
        days: i64,
    },
    /// Empty the bins on a schedule.
    Timed {
        /// The first run, Unix seconds, in the future.
        first_run: i64,
        /// Every this many days (`unit` 4) or months (`unit` 5), 1 to 365.
        every: i64,
        /// `4` days or `5` months.
        unit: i64,
    },
}

/// A plan to make or to replace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanSpec {
    /// A label, optional.
    pub name: String,
    /// The kind.
    pub kind: Kind,
    /// Time of day of a retention plan, `HH:MM:SS`.
    pub time: String,
    /// Every bin.
    pub all: bool,
    /// Shares by id, when not `all`.
    pub shares: Vec<i64>,
    /// Users by id, when not `all`.
    pub users: Vec<i64>,
}

fn check_time(time: &str) -> Result<()> {
    let parts: Vec<&str> = time.split(':').collect();
    let ok = parts.len() == 3
        && parts
            .iter()
            .zip([23, 59, 59])
            .all(|(p, max)| p.len() == 2 && p.parse::<u32>().is_ok_and(|v| v <= max));
    if ok {
        Ok(())
    } else {
        Err(refused("the time is HH:MM:SS"))
    }
}

fn body(spec: &PlanSpec, now: i64, id: Option<i64>) -> Result<Value> {
    check_time(&spec.time)?;
    if !spec.all && spec.shares.is_empty() && spec.users.is_empty() {
        return Err(refused("a plan covers every bin or names shares or users"));
    }
    let (kind, interval, unit, first) = match spec.kind {
        Kind::Retention { days } => (1, days, 4, now + days * 86_400),
        Kind::Timed {
            first_run,
            every,
            unit,
        } => {
            if first_run <= now {
                return Err(refused("the first run is in the future"));
            }
            if ![4, 5].contains(&unit) {
                return Err(refused("a timed plan runs every N days or months"));
            }
            (2, every, unit, first_run)
        }
    };
    if !(1..=365).contains(&interval) {
        return Err(refused("the days or the interval are 1 to 365"));
    }
    let mut out = json!({
        "task_name": spec.name, "cleaning_type": kind, "interval": interval,
        "interval_unit": unit, "dynamic_task_time": spec.time, "first_run_time": first,
        "is_select_all": spec.all,
        "share_ids": if spec.all { Vec::new() } else { spec.shares.clone() },
        "user_ids": if spec.all { Vec::new() } else { spec.users.clone() },
    });
    if let Some(id) = id {
        out["id"] = json!(id);
    }
    Ok(out)
}

/// Make a plan. Only one plan may cover every bin.
///
/// # Errors
///
/// Refuses what the web UI refuses. While a plan for every bin exists, UGOS
/// takes no other plan (`1359`).
pub async fn add(client: &UgosClient, spec: &PlanSpec, now: i64) -> Result<Plan> {
    let existing = list(client).await?;
    if existing.iter().any(|p| p.is_select_all) {
        return Err(refused(
            "a plan for every bin exists, and then UGOS takes no other plan: edit that one",
        ));
    }
    let before: Vec<i64> = existing.iter().map(|p| p.id).collect();
    let _: Value = client
        .post("filemgr/addPlan", &body(spec, now, None)?)
        .await?;
    list(client)
        .await?
        .into_iter()
        .find(|p| !before.contains(&p.id))
        .ok_or_else(|| refused("UGOS said yes but no plan was made"))
}

/// Replace a plan.
///
/// # Errors
///
/// Refuses what the web UI refuses, and an id that no plan has.
pub async fn edit(client: &UgosClient, id: i64, spec: &PlanSpec, now: i64) -> Result<Plan> {
    if !list(client).await?.iter().any(|p| p.id == id) {
        return Err(refused(format!("there is no plan {id}")));
    }
    let _: Value = client
        .post("filemgr/editScheduledTask", &body(spec, now, Some(id))?)
        .await?;
    list(client)
        .await?
        .into_iter()
        .find(|p| p.id == id)
        .ok_or_else(|| refused("UGOS said yes but the plan is gone"))
}

/// Delete a plan. The bins and their files stay.
///
/// # Errors
///
/// Refuses an id that no plan has.
pub async fn remove(client: &UgosClient, id: i64) -> Result<()> {
    if !list(client).await?.iter().any(|p| p.id == id) {
        return Err(refused(format!("there is no plan {id}")));
    }
    let _: Value = client
        .delete_with_body(&format!("filemgr/rmPlan?id={id}"), &json!({}))
        .await?;
    if list(client).await?.iter().any(|p| p.id == id) {
        return Err(refused("UGOS said yes but the plan is still there"));
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::{Kind, PlanSpec, body, check_time};

    fn spec(kind: Kind) -> PlanSpec {
        PlanSpec {
            name: String::new(),
            kind,
            time: "12:00:00".into(),
            all: true,
            shares: Vec::new(),
            users: Vec::new(),
        }
    }

    #[test]
    fn a_retention_plan_first_runs_after_its_days() {
        let b = body(&spec(Kind::Retention { days: 7 }), 1000, None).unwrap();
        assert_eq!(b["cleaning_type"], 1);
        assert_eq!(b["first_run_time"], 1000 + 7 * 86_400);
        assert_eq!(b["interval_unit"], 4);
        assert!(b.get("id").is_none());
    }

    #[test]
    fn a_timed_plan_needs_a_future_start_and_a_unit() {
        let timed = |first_run, unit| Kind::Timed {
            first_run,
            every: 2,
            unit,
        };
        assert!(body(&spec(timed(900, 4)), 1000, None).is_err());
        assert!(body(&spec(timed(2000, 3)), 1000, None).is_err());
        let b = body(&spec(timed(2000, 5)), 1000, Some(9)).unwrap();
        assert_eq!(
            (b["cleaning_type"].clone(), b["id"].clone()),
            (2.into(), 9.into())
        );
    }

    #[test]
    fn a_plan_names_its_bins_or_all() {
        let mut s = spec(Kind::Retention { days: 7 });
        s.all = false;
        assert!(body(&s, 0, None).is_err());
        s.shares = vec![5];
        assert_eq!(body(&s, 0, None).unwrap()["share_ids"][0], 5);
        assert!(body(&spec(Kind::Retention { days: 366 }), 0, None).is_err());
    }

    #[test]
    fn the_time_of_day_is_checked() {
        assert!(check_time("00:00:00").is_ok());
        assert!(check_time("23:59:59").is_ok());
        assert!(check_time("24:00:00").is_err());
        assert!(check_time("12:00").is_err());
    }
}
