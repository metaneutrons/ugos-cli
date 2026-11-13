//! Docker networks, read-only: which exist and which containers sit in them.

use crate::client::UgosClient;
use crate::error::{Result, UgosError};
use crate::types::common::ResultWrapper;
use crate::types::docker::{Network, NetworkMember};

fn refused(message: impl Into<String>) -> UgosError {
    UgosError::Refused(message.into())
}

/// The networks.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn list(client: &UgosClient) -> Result<Vec<Network>> {
    let answer: ResultWrapper<Option<Vec<Network>>> =
        client.get("docker/network/ShowLocalList").await?;
    Ok(answer.result.unwrap_or_default())
}

/// A network and the containers attached to it.
#[derive(Debug, Clone, serde::Serialize)]
pub struct NetworkDetail {
    /// The network.
    pub network: Network,
    /// The containers attached to it.
    pub members: Vec<NetworkMember>,
}

/// Pick the network a name or an ID (or its first characters) means.
///
/// # Errors
///
/// Refuses when nothing matches, or when a prefix matches several.
pub fn find<'a>(networks: &'a [Network], what: &str) -> Result<&'a Network> {
    if let Some(n) = networks.iter().find(|n| n.network_name == what) {
        return Ok(n);
    }
    let by_id: Vec<&Network> = networks
        .iter()
        .filter(|n| !what.is_empty() && n.network_id.starts_with(what))
        .collect();
    match by_id.as_slice() {
        [one] => Ok(one),
        [] => Err(refused(format!("there is no network '{what}'"))),
        _ => Err(refused(format!("'{what}' matches several networks"))),
    }
}

/// A network by name or ID, with its member containers.
///
/// # Errors
///
/// Refuses an unknown or ambiguous network and returns the appropriate
/// [`UgosError`] on failure.
pub async fn show(client: &UgosClient, what: &str) -> Result<NetworkDetail> {
    let networks = list(client).await?;
    let network = find(&networks, what)?.clone();
    let answer: ResultWrapper<Option<Vec<NetworkMember>>> = client
        .get_with_params(
            "docker/network/ShowLocalContainerAndNetwork",
            &[("networkId", network.network_id.as_str())],
        )
        .await?;
    let members = answer
        .result
        .unwrap_or_default()
        .into_iter()
        .filter(|m| m.flag)
        .collect();
    Ok(NetworkDetail { network, members })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::find;
    use crate::types::docker::Network;

    fn net(id: &str, name: &str) -> Network {
        serde_json::from_value(serde_json::json!({"networkId": id, "networkName": name})).unwrap()
    }

    #[test]
    fn a_network_is_found_by_name_or_id_prefix() {
        let all = [net("abc123", "bridge"), net("abd999", "host")];
        assert_eq!(find(&all, "host").unwrap().network_id, "abd999");
        assert_eq!(find(&all, "abc").unwrap().network_name, "bridge");
    }

    #[test]
    fn an_unknown_or_ambiguous_network_is_refused() {
        let all = [net("abc123", "bridge"), net("abd999", "host")];
        assert!(find(&all, "nope").is_err());
        assert!(find(&all, "ab").is_err());
        assert!(find(&all, "").is_err());
    }

    #[test]
    fn a_null_member_list_reads_as_empty() {
        let n: Network = serde_json::from_value(
            serde_json::json!({"networkId": "x", "networkName": "y", "containerNames": null}),
        )
        .unwrap();
        assert_eq!(n.container_names, Vec::<String>::new());
    }
}
