//! Image updates in the Docker app: what is outdated, and the settings that
//! decide when UGOS looks and whether it reminds.

use serde::Serialize;

use crate::api::docker::DockerApi;
use crate::client::UgosClient;
use crate::error::{Result, UgosError};
use crate::ops::dockerview;
use crate::types::common::ResultWrapper;
use crate::types::docker::UpdateConfig;

const CONFIG_GET: &str = "docker/user/GetContainerImageUpdateConfig";
const CONFIG_SET: &str = "docker/user/SetContainerImageUpdateConfig";

fn refused(message: impl Into<String>) -> UgosError {
    UgosError::Refused(message.into())
}

/// What has a newer image.
#[derive(Debug, Clone, Serialize)]
pub struct UpdateStatus {
    /// Containers with a newer image, by name.
    pub containers: Vec<String>,
    /// Compose projects with a newer image, by name.
    pub projects: Vec<String>,
    /// The count UGOS gives for containers.
    pub container_count: i64,
    /// The count UGOS gives for projects.
    pub project_count: i64,
    /// The settings.
    pub config: UpdateConfig,
}

/// The current settings.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn config(client: &UgosClient) -> Result<UpdateConfig> {
    client.get(CONFIG_GET).await
}

/// What is outdated, and the settings.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn status(client: &UgosClient) -> Result<UpdateStatus> {
    let container_count: ResultWrapper<i64> = client
        .get("docker/container/GetUpdateContainerCount")
        .await?;
    let project_count: ResultWrapper<i64> =
        client.get("docker/compose/GetUpdateProjectCount").await?;
    let containers = dockerview::containers(client, 1, 500)
        .await?
        .into_iter()
        .filter(|v| v.stats.as_ref().is_some_and(|s| s.img_has_update))
        .map(|v| v.container.name)
        .collect();
    let projects = client
        .project_list()
        .await?
        .into_iter()
        .filter(|p| p.img_has_update)
        .map(|p| p.name)
        .collect();
    Ok(UpdateStatus {
        containers,
        projects,
        container_count: container_count.result,
        project_count: project_count.result,
        config: config(client).await?,
    })
}

/// What to change in the settings. A field left `None` stays as it is.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConfigChange {
    /// Remind when an update exists.
    pub reminder: Option<bool>,
    /// Look for updates on a schedule.
    pub plan: Option<bool>,
    /// Time of day of the schedule, `HH:MM`.
    pub time: Option<String>,
}

fn check_time(time: &str) -> Result<()> {
    let ok = time.split_once(':').is_some_and(|(h, m)| {
        h.len() == 2
            && m.len() == 2
            && h.parse::<u32>().is_ok_and(|h| h < 24)
            && m.parse::<u32>().is_ok_and(|m| m < 60)
    });
    if ok {
        Ok(())
    } else {
        Err(refused("the time is HH:MM, for example 06:00"))
    }
}

/// The settings with a change applied. The schedule's days and type are kept
/// as they are, they are not understood well enough to be changed.
///
/// # Errors
///
/// Refuses a time that is not `HH:MM`.
pub fn apply(now: &UpdateConfig, change: &ConfigChange) -> Result<UpdateConfig> {
    let mut out = now.clone();
    if let Some(time) = &change.time {
        check_time(time)?;
        out.plan_settings.date.clone_from(time);
    }
    if let Some(plan) = change.plan {
        out.is_plan = plan;
    }
    if let Some(reminder) = change.reminder {
        out.update_reminder = reminder;
    }
    Ok(out)
}

/// Change the settings and read them back.
///
/// # Errors
///
/// Refuses an empty change or a bad time, and fails when UGOS answers yes but
/// keeps the old settings.
pub async fn set_config(client: &UgosClient, change: &ConfigChange) -> Result<UpdateConfig> {
    if *change == ConfigChange::default() {
        return Err(refused("nothing to change"));
    }
    let wanted = apply(&config(client).await?, change)?;
    let _: serde_json::Value = client.post(CONFIG_SET, &wanted).await?;
    let now = config(client).await?;
    if now != wanted {
        return Err(refused("UGOS said yes but the settings did not change"));
    }
    Ok(now)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::{ConfigChange, apply, check_time};
    use crate::types::docker::UpdateConfig;

    fn now() -> UpdateConfig {
        serde_json::from_value(serde_json::json!({
            "isPlan": false,
            "planSettings": {"date": "06:00", "day": [1], "planType": 2},
            "updateReminder": true
        }))
        .unwrap()
    }

    #[test]
    fn a_change_touches_only_what_it_names() {
        let out = apply(
            &now(),
            &ConfigChange {
                reminder: Some(false),
                ..ConfigChange::default()
            },
        )
        .unwrap();
        assert!(!out.update_reminder);
        assert_eq!(out.plan_settings, now().plan_settings);
        assert!(!out.is_plan);
    }

    #[test]
    fn the_time_replaces_the_schedule_time_and_keeps_days() {
        let out = apply(
            &now(),
            &ConfigChange {
                time: Some("07:30".into()),
                plan: Some(true),
                ..ConfigChange::default()
            },
        )
        .unwrap();
        assert_eq!(out.plan_settings.date, "07:30");
        assert_eq!(
            (out.plan_settings.day, out.plan_settings.plan_type),
            (vec![1], 2)
        );
        assert!(out.is_plan);
    }

    #[test]
    fn a_bad_time_is_refused() {
        assert!(check_time("06:00").is_ok());
        assert!(check_time("23:59").is_ok());
        assert!(check_time("24:00").is_err());
        assert!(check_time("6:00").is_err());
        assert!(check_time("0600").is_err());
    }
}
