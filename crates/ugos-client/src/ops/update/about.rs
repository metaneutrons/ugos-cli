//! The about pages: the machine, the warranty and the ports in use.

use serde_json::Value;

use crate::client::UgosClient;
use crate::error::Result;
use crate::types::update::{ServiceList, ServicePorts};

/// Model, serial, version, run time and hardware, as UGOS gives them.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`](crate::error::UgosError) on failure.
pub async fn machine(client: &UgosClient) -> Result<Value> {
    client.get("sysinfo/machine/common").await
}

/// The warranty period, as the start and end in Unix milliseconds.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`](crate::error::UgosError) on failure.
pub async fn repair(client: &UgosClient) -> Result<(i64, i64)> {
    let v: Value = client.get("sysinfo/machine/repair/status").await?;
    let get = |key: &str| v.get(key).and_then(Value::as_i64).unwrap_or(0);
    Ok((get("repairStartTime"), get("repairEndTime")))
}

/// The services and applications and the ports they listen on.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`](crate::error::UgosError) on failure.
pub async fn ports(client: &UgosClient) -> Result<Vec<ServicePorts>> {
    let list: ServiceList = client.get("sysinfo/collect/getInfo").await?;
    Ok(list.result)
}
