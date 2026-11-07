//! The NFS service.

use serde_json::Value;

use super::fields::{Field, Kind};
use super::{Config, PortUse, flag, number, refused};
use crate::error::{Result, UgosError};

/// The settings. The web UI offers only `NFSv3` and no control for packet sizes
/// and domain, so those stay as they are.
pub const FIELDS: &[Field] = &[
    Field::of(
        "protocol",
        "maximumNFSProtocol",
        Kind::Word(&["NFSv3"]),
        "highest NFS version (only NFSv3 is offered)",
    ),
    Field::flag(
        "custom-ports",
        "customerPort",
        "use fixed ports for statd and nlockmgr",
    ),
    Field::of(
        "statd-port",
        "statdPort",
        Kind::Int(1, 65535),
        "statd port, with custom-ports",
    ),
    Field::of(
        "nlockmgr-port",
        "nlockMgrPort",
        Kind::Int(1, 65535),
        "nlockmgr port, with custom-ports",
    ),
];

pub(super) fn finish(
    current: &Config,
    next: &mut Config,
    enable: bool,
    force: bool,
) -> Result<Vec<PortUse>> {
    if enable && !force && !flag(current, "enableNfsServer") {
        return Err(UgosError::NeedsForce(
            "NFS has no login: who may mount a share rests on the address rules of each share"
                .into(),
        ));
    }
    // The web UI sends these two with every start and never reads them.
    let _ = next.insert("logs".into(), Value::Bool(true));
    let _ = next.insert("enableAdvance".into(), Value::Bool(true));
    for key in ["statdPort", "nlockMgrPort"] {
        let _ = next.insert(key.into(), Value::from(number(next, key)));
    }
    if !flag(next, "customerPort") {
        return Ok(Vec::new());
    }
    let (statd, lock) = (number(next, "statdPort"), number(next, "nlockMgrPort"));
    if statd == 0 || lock == 0 {
        return Err(refused("custom ports need statd-port and nlockmgr-port"));
    }
    if statd == lock {
        return Err(refused("statd-port and nlockmgr-port must differ"));
    }
    Ok(vec![
        PortUse {
            module: "nfs",
            key: "statdPort",
            name: Some("statd"),
            port: statd,
        },
        PortUse {
            module: "nfs",
            key: "nlockMgrPort",
            name: Some("nlockmgr"),
            port: lock,
        },
    ])
}
