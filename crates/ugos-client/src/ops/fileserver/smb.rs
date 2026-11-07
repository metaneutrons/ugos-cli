//! The SMB service.

use serde_json::{Value, json};

use super::fields::{Field, Kind};
use super::{Config, PortUse, flag, number, refused};
use crate::client::UgosClient;
use crate::error::{Result, UgosError};

const MODE: &[(&str, i64)] = &[("client", 0), ("off", 1), ("force", 2)];

/// The settings, as the three tabs of the web UI's advanced dialog have them.
pub const FIELDS: &[Field] = &[
    Field::of(
        "workgroup",
        "work_group",
        Kind::Workgroup,
        "workgroup, 1 to 15 characters",
    ),
    Field::of(
        "wins",
        "wins_server",
        Kind::Addresses,
        "WINS servers, IP addresses, or empty for none",
    ),
    Field::of(
        "protocols",
        "smb_protocols",
        Kind::Protocols,
        "smb1, smb2, or smb1,smb2 (SMB1 is high risk)",
    ),
    Field::of(
        "encryption",
        "encrypt_mode",
        Kind::Code(MODE),
        "transport encryption: client, off or force",
    ),
    Field::of(
        "signing",
        "server_sign_mode",
        Kind::Code(MODE),
        "server signing: client, off or force",
    ),
    Field::flag(
        "hide-unreadable",
        "hide_share_dir_for_no_permission",
        "hide shares a user may not open",
    ),
    Field::flag("log", "enable_log", "keep a log"),
    Field::flag(
        "durable-handles",
        "enable_smb_durable_handles",
        "SMB durable handles",
    ),
    Field::flag(
        "multichannel",
        "enable_smb3_multi_channel",
        "SMB3 multichannel (not with forced encryption)",
    ),
    Field::flag("oplocks", "enable_op_locks", "opportunistic locks"),
    Field::flag(
        "leases",
        "enable_smb2_leases",
        "SMB2 leases (needs oplocks)",
    ),
    Field::flag(
        "macos",
        "enable_vfs_for_mac_special_char",
        "macOS special characters",
    ),
    Field::flag(
        "local-master",
        "enable_local_master_browser",
        "local master browser",
    ),
    Field::flag("dir-sort", "enable_dir_sort_vfs", "sort directories"),
    Field::flag("veto-on", "enable_veto_files", "hide files by name"),
    Field::of(
        "veto",
        "veto_files",
        Kind::Veto,
        "names to hide, in slashes: /name/other/",
    ),
    Field::flag(
        "veto-delete",
        "enable_delete_veto_files",
        "delete hidden files with their folder",
    ),
    Field::flag(
        "symlinks",
        "enable_follow_symlinks",
        "follow symbolic links",
    ),
    Field::flag(
        "wide-links",
        "enable_wide_links",
        "follow links outside the share (needs symlinks)",
    ),
    Field::flag(
        "one-connection-per-ip",
        "disable_many_conn_same_ip",
        "refuse many connections from one address",
    ),
    Field::flag("debug-log", "enable_debug_log", "debug log"),
    Field::flag(
        "unix-permissions",
        "disable_strict_allocate",
        "apply default UNIX permissions to new files",
    ),
    Field::flag("ntlmv1", "enable_ntlmv1", "allow the weak NTLMv1 login"),
    Field::flag("async-read", "enable_async_read", "asynchronous reads"),
    Field::flag("strict-sync", "enable_strict_sync", "strict sync"),
    Field::flag(
        "wildcard-cache",
        "enable_wildcard_search_cache",
        "cache wildcard searches",
    ),
];

fn has_protocol(config: &Config, code: i64) -> bool {
    config
        .get("smb_protocols")
        .and_then(Value::as_array)
        .is_some_and(|list| list.iter().any(|v| v.as_i64() == Some(code)))
}

/// The combinations of settings the web UI's dialog refuses.
fn check_combinations(next: &Config) -> Result<()> {
    if next
        .get("smb_protocols")
        .and_then(Value::as_array)
        .is_none_or(Vec::is_empty)
    {
        return Err(refused("at least one protocol is needed"));
    }
    if number(next, "encrypt_mode") == 2 && flag(next, "enable_smb3_multi_channel") {
        return Err(refused(
            "forced encryption and multichannel exclude each other: turn one off",
        ));
    }
    if flag(next, "enable_smb2_leases") && !flag(next, "enable_op_locks") {
        return Err(refused("leases need oplocks"));
    }
    if flag(next, "enable_wide_links") && !flag(next, "enable_follow_symlinks") {
        return Err(refused("wide links need symbolic links"));
    }
    let veto = next
        .get("veto_files")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if flag(next, "enable_veto_files") && veto.is_empty() {
        return Err(refused("hiding files by name needs a veto list"));
    }
    if flag(next, "enable_delete_veto_files") && !flag(next, "enable_veto_files") {
        return Err(refused("deleting hidden files needs hiding files by name"));
    }
    if !flag(next, "enable_follow_symlinks") && flag(next, "enable_vfs_for_mac_special_char") {
        return Err(refused(
            "macOS special characters need symbolic links, which are off",
        ));
    }
    Ok(())
}

/// What the web UI asks a confirmation for.
fn check_risks(current: &Config, next: &Config) -> Result<()> {
    if has_protocol(next, 1) && !has_protocol(current, 1) {
        return Err(UgosError::NeedsForce(
            "SMB1 is a high-risk protocol, old and open to attacks".into(),
        ));
    }
    if flag(next, "enable_ntlmv1") && !flag(current, "enable_ntlmv1") {
        return Err(UgosError::NeedsForce(
            "NTLMv1 is a weak login that can be broken".into(),
        ));
    }
    Ok(())
}

/// Check the settings together, as the web UI's dialog does, and say what
/// needs `force`.
pub(super) fn finish(current: &Config, next: &mut Config, force: bool) -> Result<Vec<PortUse>> {
    let name = next
        .get("work_group")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if name.is_empty() {
        let _ = next.insert("work_group".into(), Value::String("WORKGROUP".into()));
    }
    check_combinations(next)?;
    if !force {
        check_risks(current, next)?;
    }
    Ok(Vec::new())
}

/// Clear the SMB cache. The network service restarts after it, so this needs
/// `force`.
///
/// # Errors
///
/// Returns [`UgosError::NeedsForce`] without `force`, and the appropriate
/// [`UgosError`] on failure.
pub async fn clear_cache(client: &UgosClient, force: bool) -> Result<()> {
    if !force {
        return Err(UgosError::NeedsForce(
            "clearing the SMB cache restarts the network service".into(),
        ));
    }
    let _: Value = client
        .delete_with_body("file/smb/cache", &json!({}))
        .await?;
    Ok(())
}
