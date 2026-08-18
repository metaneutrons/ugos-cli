//! UGOS core API: machine info and live monitoring.
//!
//! These endpoints sit outside the KVM and Docker apps, under `sysinfo/` and
//! `taskmgr/`, and answer in `snake_case` rather than the camelCase the app
//! APIs use.

use crate::client::UgosClient;
use crate::error::Result;
use crate::types::system::{MachineInfo, ProcessList, ServiceList, SystemStats};

/// Machine information and monitoring.
#[allow(clippy::module_name_repetitions)]
pub trait SystemApi {
    /// Machine identity and installed hardware.
    fn machine_info(&self) -> impl Future<Output = Result<MachineInfo>> + Send;
    /// Current CPU, memory, disk, network, fan and GPU readings.
    fn system_stats(&self) -> impl Future<Output = Result<SystemStats>> + Send;
    /// Running processes with their resource use.
    fn processes(&self) -> impl Future<Output = Result<ProcessList>> + Send;
    /// Installed services with their resource use.
    fn services(&self) -> impl Future<Output = Result<ServiceList>> + Send;
}

impl SystemApi for UgosClient {
    async fn machine_info(&self) -> Result<MachineInfo> {
        self.get("sysinfo/machine/common").await
    }

    async fn system_stats(&self) -> Result<SystemStats> {
        self.get("taskmgr/stat/overview").await
    }

    async fn processes(&self) -> Result<ProcessList> {
        self.get("taskmgr/processes").await
    }

    async fn services(&self) -> Result<ServiceList> {
        self.get("taskmgr/services").await
    }
}
