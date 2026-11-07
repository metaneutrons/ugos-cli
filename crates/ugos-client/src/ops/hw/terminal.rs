//! SSH and Telnet.
//!
//! UGOS reads the two services as `SshConfig` and `TelnetConfig` and takes
//! them back as `sshConfig` and `telnetConfig`, both together every time.
//! Switching either on opens a way in, so it needs `force`, as the web UI asks
//! for a confirmation.

use serde_json::{Value, json};

use crate::client::UgosClient;
use crate::error::{Result, UgosError};
use crate::types::hw::{Terminal, TerminalGet};

fn refused(message: impl Into<String>) -> UgosError {
    UgosError::Refused(message.into())
}

/// Ports each service may not use.
const SSH_RESERVED: [i64; 6] = [80, 20, 21, 23, 443, 3493];
const TELNET_RESERVED: [i64; 6] = [80, 20, 21, 22, 443, 3493];

/// The two services as they are.
///
/// # Errors
///
/// Returns the appropriate [`UgosError`] on failure.
pub async fn terminal(client: &UgosClient) -> Result<Terminal> {
    let mut got: TerminalGet = client.get("terminal/get").await?;
    // The server says 2 where the web UI works with 3.
    if got.ssh.encrypt_level == 2 {
        got.ssh.encrypt_level = 3;
    }
    Ok(Terminal {
        ssh: got.ssh,
        telnet: got.telnet,
    })
}

/// What to change. `None` leaves a setting alone.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TerminalChange {
    /// Switch SSH on or off.
    pub ssh: Option<bool>,
    /// SSH port.
    pub ssh_port: Option<i64>,
    /// SFTP over SSH.
    pub sftp: Option<bool>,
    /// SSH encryption: `1` low, `3` high.
    pub ssh_encryption: Option<i64>,
    /// How long SSH stays on, `1` to `7`.
    pub ssh_duration: Option<i64>,
    /// Switch Telnet on or off.
    pub telnet: Option<bool>,
    /// Telnet port.
    pub telnet_port: Option<i64>,
    /// Only from the local network, for both.
    pub lan_only: Option<bool>,
}

fn port(value: i64, reserved: &[i64], what: &str) -> Result<i64> {
    if !(1..=65535).contains(&value) {
        return Err(refused(format!("the {what} port is 1 to 65535")));
    }
    if reserved.contains(&value) {
        return Err(refused(format!("{value} is taken for another service")));
    }
    Ok(value)
}

/// Apply a change to a copy.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] for a port the web UI would refuse or equal
/// ports on two running services.
pub fn apply(current: &Terminal, change: &TerminalChange) -> Result<Terminal> {
    let mut next = current.clone();
    let on = |flag: bool| if flag { 1 } else { 2 };
    if let Some(flag) = change.ssh {
        next.ssh.enable = on(flag);
    }
    if let Some(flag) = change.telnet {
        next.telnet.enable = on(flag);
    }
    if let Some(p) = change.ssh_port {
        next.ssh.port = port(p, &SSH_RESERVED, "SSH")?;
    }
    if let Some(p) = change.telnet_port {
        next.telnet.port = port(p, &TELNET_RESERVED, "Telnet")?;
    }
    if let Some(flag) = change.sftp {
        next.ssh.use_sftp = on(flag);
    }
    if let Some(level) = change.ssh_encryption {
        if ![1, 3].contains(&level) {
            return Err(refused("the SSH encryption level is 1 (low) or 3 (high)"));
        }
        next.ssh.encrypt_level = level;
    }
    if let Some(hours) = change.ssh_duration {
        if !(1..=7).contains(&hours) {
            return Err(refused(
                "the SSH duration is 1 (10 minutes) to 7 (until switched off)",
            ));
        }
        next.ssh.ex_time_duration = hours;
    }
    if let Some(flag) = change.lan_only {
        next.ssh.permit_lan_access_only = on(flag);
        next.telnet.permit_lan_access_only = on(flag);
    }
    if next.ssh.enable == 1 && next.telnet.enable == 1 && next.ssh.port == next.telnet.port {
        return Err(refused("SSH and Telnet cannot share a port"));
    }
    Ok(next)
}

fn body(next: &Terminal) -> Value {
    json!({
        "sshConfig": {
            "enable": next.ssh.enable, "port": next.ssh.port,
            "permitLanAccessOnly": next.ssh.permit_lan_access_only,
            "useSftp": next.ssh.use_sftp,
            "encryptLevel": if next.ssh.encrypt_level == 0 { 1 } else { next.ssh.encrypt_level },
            "exTimeDuration": if next.ssh.ex_time_duration == 0 { 1 } else { next.ssh.ex_time_duration },
        },
        "telnetConfig": {
            "enable": next.telnet.enable, "port": next.telnet.port,
            "permitLanAccessOnly": next.telnet.permit_lan_access_only,
        },
    })
}

const fn same(now: &Terminal, wanted: &Terminal) -> bool {
    now.ssh.enable == wanted.ssh.enable
        && now.ssh.port == wanted.ssh.port
        && now.telnet.enable == wanted.telnet.enable
        && now.telnet.port == wanted.telnet.port
}

/// Change SSH or Telnet and read them back.
///
/// # Errors
///
/// Returns [`UgosError::NeedsForce`] when a service is being switched on and
/// `force` is not set, and [`UgosError::Refused`] for nothing to change, input
/// the web UI would refuse, or a change UGOS did not keep.
pub async fn set_terminal(
    client: &UgosClient,
    change: &TerminalChange,
    force: bool,
) -> Result<Terminal> {
    if *change == TerminalChange::default() {
        return Err(refused("nothing to change"));
    }
    let current = terminal(client).await?;
    let next = apply(&current, change)?;
    let opens = (next.ssh.enable == 1 && current.ssh.enable != 1)
        || (next.telnet.enable == 1 && current.telnet.enable != 1);
    if opens && !force {
        return Err(UgosError::NeedsForce(
            "switching SSH or Telnet on opens a way into the NAS; Telnet sends passwords in the clear".into(),
        ));
    }
    let _: Value = client.post("terminal/set", &body(&next)).await?;
    let now = terminal(client).await?;
    if same(&now, &next) {
        Ok(now)
    } else {
        Err(refused(
            "UGOS accepted the request but the terminal settings did not change",
        ))
    }
}
