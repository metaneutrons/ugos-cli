//! What the Docker app shows besides the container list: live figures, the
//! logs of a container, and the app's own log.

use std::collections::HashMap;

use serde_json::json;

use crate::api::docker::DockerApi;
use crate::client::UgosClient;
use crate::error::{Result, UgosError};
use crate::types::common::ResultWrapper;
use crate::types::docker::{Container, ContainerLog, ContainerStats, OpLog, OpLogPage};

fn refused(message: impl Into<String>) -> UgosError {
    UgosError::Refused(message.into())
}

/// A container with its live figures.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ContainerView {
    /// The entry of the list.
    pub container: Container,
    /// The figures, when UGOS has them for it.
    pub stats: Option<ContainerStats>,
}

/// The containers of one page, each with its live figures.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn containers(
    client: &UgosClient,
    page: u32,
    page_size: u32,
) -> Result<Vec<ContainerView>> {
    let list = client
        .container_list(page, page_size)
        .await?
        .result
        .unwrap_or_default();
    let body = json!({"pageNum": page, "pageSize": page_size});
    let answer: ResultWrapper<Option<Vec<ContainerStats>>> = client
        .post("docker/container/ShowContainerDetailListV2", &body)
        .await?;
    let mut stats: HashMap<String, ContainerStats> = answer
        .result
        .unwrap_or_default()
        .into_iter()
        .map(|s| (s.container_id.clone(), s))
        .collect();
    Ok(list
        .into_iter()
        .map(|container| {
            let stats = stats.remove(&container.container_id);
            ContainerView { container, stats }
        })
        .collect())
}

/// The full ID of the container a name, an ID or the first characters of an
/// ID mean.
///
/// # Errors
///
/// Refuses when nothing matches, or when the characters match several
/// containers.
pub async fn resolve(client: &UgosClient, what: &str) -> Result<String> {
    let all = client
        .container_list(1, 1000)
        .await?
        .result
        .unwrap_or_default();
    pick(&all, what)
}

fn pick(all: &[Container], what: &str) -> Result<String> {
    if let Some(c) = all.iter().find(|c| c.name == what) {
        return Ok(c.container_id.clone());
    }
    let by_id: Vec<&Container> = all
        .iter()
        .filter(|c| !what.is_empty() && c.container_id.starts_with(what))
        .collect();
    match by_id.as_slice() {
        [one] => Ok(one.container_id.clone()),
        [] => Err(refused(format!("there is no container '{what}'"))),
        _ => Err(refused(format!("'{what}' matches several containers"))),
    }
}

/// The last `lines` lines of a text, with the line breaks kept.
#[must_use]
pub fn tail(text: &str, lines: usize) -> String {
    let all: Vec<&str> = text.split_inclusive('\n').collect();
    all[all.len().saturating_sub(lines)..].concat()
}

/// The log of a container as UGOS keeps it, cut to the last `lines` lines.
///
/// `ShowContainerLogs` takes no count: it answers with the whole log up to a
/// limit, so the cut is made here. `cut` is true when UGOS stopped at its
/// limit, which means older lines exist that this call cannot reach.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn container_log(client: &UgosClient, id: &str, lines: usize) -> Result<(String, bool)> {
    let body = json!({"containerId": id});
    let answer: ResultWrapper<ContainerLog> = client
        .post("docker/container/ShowContainerLogs", &body)
        .await?;
    let log = answer.result;
    Ok((tail(&log.message, lines), log.upper_limit))
}

/// What to look for in the Docker app's log.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OpLogQuery {
    /// Only this user (`system` is the NAS itself).
    pub operator: Option<String>,
    /// Only failures.
    pub failures_only: bool,
    /// Only entries whose text contains this.
    pub contains: Option<String>,
    /// Oldest first.
    pub oldest_first: bool,
}

fn oplog_body(q: &OpLogQuery, page: u32, size: u32) -> serde_json::Value {
    json!({
        "pageNum": page,
        "pageSize": size,
        "operator": q.operator.clone().unwrap_or_default(),
        "level": if q.failures_only { 2 } else { 0 },
        "content": q.contains.clone().unwrap_or_default(),
        "createTimeSort": if q.oldest_first { "asc" } else { "desc" },
    })
}

/// One page of the Docker app's log.
///
/// # Errors
///
/// Refuses a page of 0 and returns the appropriate [`UgosError`] on failure.
pub async fn oplog(
    client: &UgosClient,
    q: &OpLogQuery,
    page: u32,
    size: u32,
) -> Result<(Vec<OpLog>, u64)> {
    if page == 0 || size == 0 {
        return Err(refused("page and page size start at 1"));
    }
    let answer: OpLogPage = client
        .post("docker/log/PageSearchLogs", &oplog_body(q, page, size))
        .await?;
    Ok((answer.logs.unwrap_or_default(), answer.total))
}

/// The users that appear in the Docker app's log.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn oplog_users(client: &UgosClient) -> Result<Vec<String>> {
    let answer: ResultWrapper<Vec<String>> = client.get("docker/log/GetAllOperator").await?;
    Ok(answer.result)
}

/// The whole log of the Docker app as the tab-separated text UGOS exports.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn oplog_export(client: &UgosClient, failures_only: bool) -> Result<String> {
    let level = if failures_only { "2" } else { "0" };
    let params = [
        ("pageNum", "1"),
        ("pageSize", "20"),
        ("level", level),
        ("exportType", "txt"),
    ];
    let bytes = client
        .get_bytes("docker/log/ExportLogs", &params)
        .await?
        .bytes()
        .await?;
    String::from_utf8(bytes.to_vec()).map_err(|_| refused("the exported log is not text"))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::{OpLogQuery, oplog_body, pick, tail};
    use crate::types::docker::Container;

    fn container(id: &str, name: &str) -> Container {
        serde_json::from_value(serde_json::json!({"containerId": id, "name": name})).unwrap()
    }

    #[test]
    fn a_container_is_found_by_name_or_id_prefix() {
        let all = [container("abc123", "web"), container("abd999", "db")];
        assert_eq!(pick(&all, "db").unwrap(), "abd999");
        assert_eq!(pick(&all, "abc").unwrap(), "abc123");
        assert!(pick(&all, "ab").is_err());
        assert!(pick(&all, "zzz").is_err());
        assert!(pick(&all, "").is_err());
    }

    #[test]
    fn tail_keeps_the_last_lines_and_their_breaks() {
        assert_eq!(tail("a\nb\nc\n", 2), "b\nc\n");
        assert_eq!(tail("a\nb", 5), "a\nb");
        assert_eq!(tail("a\nb\n", 0), "");
        assert_eq!(tail("", 3), "");
    }

    #[test]
    fn the_query_maps_to_the_fields_ugos_filters_on() {
        let q = OpLogQuery {
            operator: Some("fabian".into()),
            failures_only: true,
            contains: Some("deploy".into()),
            oldest_first: true,
        };
        let b = oplog_body(&q, 2, 10);
        assert_eq!(b["operator"], "fabian");
        assert_eq!(b["level"], 2);
        assert_eq!(b["content"], "deploy");
        assert_eq!(b["createTimeSort"], "asc");
        assert_eq!(b["pageNum"], 2);
        assert_eq!(oplog_body(&OpLogQuery::default(), 1, 5)["level"], 0);
    }
}
