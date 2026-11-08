//! Tests for MCP tool registration.

#![allow(clippy::unwrap_used)]

use crate::UgosMcp;
use ugos_client::Credentials;

fn make_server() -> UgosMcp {
    UgosMcp::new(vec![crate::TargetConfig {
        name: "test".into(),
        host: "127.0.0.1".into(),
        port: 9443,
        creds: Credentials {
            username: "test".into(),
            password: "test".into(),
            otp: None,
            otp_secret: None,
        },
    }])
}

/// Every tool the server registers.
const EXPECTED_TOOLS: [&str; 255] = [
    "ugos_user_create",
    "ugos_group_create",
    "ugos_group_edit",
    "ugos_group_delete",
    "ugos_group_permissions",
    "ugos_group_permit",
    "ugos_user_permissions",
    "ugos_user_permit",
    "ugos_user_quota",
    "ugos_user_quota_set",
    "ugos_syslog",
    "ugos_logcenter_modules",
    "ugos_logcenter_archive_show",
    "ugos_logcenter_archive_set",
    "ugos_logcenter_archive_records",
    "ugos_logcenter_archive_files",
    "ugos_logcenter_archive_add",
    "ugos_logcenter_archive_open",
    "ugos_logcenter_archive_remove",
    "ugos_logcenter_notify_show",
    "ugos_logcenter_notify_set",
    "ugos_logcenter_notify_records",
    "ugos_logcenter_filemgr",
    "ugos_logcenter_clear",
    "ugos_logcenter_export",
    "ugos_version_folders",
    "ugos_version_set",
    "ugos_version_status",
    "ugos_version_ls",
    "ugos_version_history",
    "ugos_version_copy",
    "ugos_version_restore_deleted",
    "ugos_version_clear_deleted",
    "ugos_version_calendar",
    "ugos_recycle_status",
    "ugos_recycle_config",
    "ugos_recycle_list",
    "ugos_recycle_enable",
    "ugos_recycle_disable",
    "ugos_recycle_ls",
    "ugos_recycle_restore",
    "ugos_recycle_empty",
    "ugos_recycle_delete",
    "ugos_recycle_plan_list",
    "ugos_recycle_plan_choices",
    "ugos_recycle_plan_add",
    "ugos_recycle_plan_edit",
    "ugos_recycle_plan_remove",
    "ugos_san_setting",
    "ugos_san_setting_set",
    "ugos_san_target_list",
    "ugos_san_target_show",
    "ugos_san_target_predict",
    "ugos_san_target_connections",
    "ugos_san_target_create",
    "ugos_san_target_edit",
    "ugos_san_target_enable",
    "ugos_san_target_disable",
    "ugos_san_target_delete",
    "ugos_san_lun_list",
    "ugos_san_lun_show",
    "ugos_san_lun_create",
    "ugos_san_lun_edit",
    "ugos_san_lun_delete",
    "ugos_san_group_list",
    "ugos_san_group_create",
    "ugos_san_group_edit",
    "ugos_san_group_delete",
    "ugos_update_status",
    "ugos_update_config",
    "ugos_update_install",
    "ugos_update_upload",
    "ugos_update_progress",
    "ugos_update_key",
    "ugos_update_clean",
    "ugos_backup_save",
    "ugos_backup_info",
    "ugos_backup_services",
    "ugos_backup_restore",
    "ugos_backup_progress",
    "ugos_backup_reset_network",
    "ugos_backup_reset_factory",
    "ugos_index_status",
    "ugos_index_ranges",
    "ugos_index_pause",
    "ugos_index_resume",
    "ugos_index_regenerate",
    "ugos_system_about",
    "ugos_system_ports",
    "ugos_system_reboot",
    "ugos_system_poweroff",
    "ugos_fileservice_status",
    "ugos_fileservice_settings",
    "ugos_fileservice_start",
    "ugos_fileservice_set",
    "ugos_fileservice_stop",
    "ugos_smb_discovery",
    "ugos_smb_cache_clear",
    "ugos_hw_buzzer",
    "ugos_hw_buzzer_alarms",
    "ugos_hw_buzzer_set",
    "ugos_hw_cpu",
    "ugos_hw_cpu_set",
    "ugos_hw_fan",
    "ugos_hw_fan_set",
    "ugos_hw_fan_curve",
    "ugos_hw_led",
    "ugos_hw_led_on",
    "ugos_hw_led_off",
    "ugos_hw_led_brightness",
    "ugos_hw_screen",
    "ugos_hw_screen_wallpapers",
    "ugos_hw_screen_set",
    "ugos_hw_zram",
    "ugos_hw_zram_on",
    "ugos_hw_zram_off",
    "ugos_hw_power",
    "ugos_hw_power_set",
    "ugos_hw_schedule_add",
    "ugos_hw_schedule_remove",
    "ugos_hw_ups",
    "ugos_hw_ups_set",
    "ugos_hw_ups_disconnect",
    "ugos_hw_ups_check",
    "ugos_hw_ups_mibs",
    "ugos_hw_ups_firmware",
    "ugos_hw_gpu",
    "ugos_hw_gpu_mode",
    "ugos_time_show",
    "ugos_time_zones",
    "ugos_time_set",
    "ugos_time_servers",
    "ugos_time_server_add",
    "ugos_time_server_remove",
    "ugos_lang_show",
    "ugos_lang_list",
    "ugos_lang_set",
    "ugos_terminal_show",
    "ugos_terminal_set",
    "ugos_system_hostname_set",
    "ugos_system_finder",
    "ugos_system_finder_set",
    "ugos_target_list",
    // KVM
    "ugos_vm_list",
    "ugos_vm_show",
    "ugos_vm_start",
    "ugos_vm_stop",
    "ugos_vm_reboot",
    "ugos_vm_delete",
    "ugos_vm_create",
    "ugos_vm_update",
    "ugos_host_info",
    // Snapshots
    "ugos_snapshot_list",
    "ugos_snapshot_create",
    "ugos_snapshot_delete",
    "ugos_snapshot_revert",
    "ugos_snapshot_describe",
    // Network
    "ugos_network_list",
    "ugos_network_show",
    "ugos_network_delete",
    "ugos_network_create",
    "ugos_network_update",
    // Storage
    "ugos_storage_list",
    "ugos_storage_usage",
    "ugos_storage_add",
    "ugos_storage_delete",
    // Image
    "ugos_image_list",
    "ugos_image_delete",
    "ugos_image_usage",
    // USB
    "ugos_usb_list",
    // VNC
    "ugos_vnc_list",
    "ugos_vnc_generate",
    // OVA
    "ugos_ova_export",
    "ugos_ova_parse",
    // Logs
    "ugos_log_search",
    "ugos_log_operators",
    // Docker
    "ugos_docker_overview",
    "ugos_docker_status",
    "ugos_docker_ps",
    "ugos_docker_show",
    "ugos_docker_create",
    "ugos_docker_start",
    "ugos_docker_stop",
    "ugos_docker_restart",
    "ugos_docker_rm",
    "ugos_docker_logs",
    "ugos_docker_clone",
    "ugos_docker_batch",
    "ugos_docker_images",
    "ugos_docker_search",
    "ugos_docker_pull",
    "ugos_docker_image_export",
    "ugos_docker_image_load_url",
    "ugos_docker_image_load_path",
    "ugos_docker_mirrors",
    "ugos_docker_mirror_add",
    "ugos_docker_mirror_delete",
    "ugos_docker_mirror_switch",
    "ugos_docker_compose",
    "ugos_project_list",
    "ugos_project_show",
    "ugos_project_create",
    "ugos_project_start",
    "ugos_project_stop",
    "ugos_project_restart",
    "ugos_project_remove",
    "ugos_docker_proxy_get",
    "ugos_docker_proxy_set",
    // Shared folders
    "ugos_share_list",
    "ugos_share_show",
    "ugos_share_acl_show",
    "ugos_smb_status",
    "ugos_share_create",
    "ugos_share_update",
    "ugos_share_acl_set",
    "ugos_share_delete",
    // Accounts and groups
    "ugos_user_list",
    "ugos_user_show",
    "ugos_group_list",
    "ugos_user_update",
    "ugos_user_delete",
    // Storage Manager
    "ugos_pool_list",
    "ugos_pool_show",
    "ugos_volume_list",
    "ugos_volume_show",
    "ugos_disk_list",
    "ugos_disk_show",
    "ugos_disk_smart",
    "ugos_disk_history",
    "ugos_disk_plans",
    // Network of the NAS
    "ugos_net_list",
    "ugos_net_show",
    "ugos_net_general",
    "ugos_net_set",
    "ugos_net_dns_set",
    "ugos_net_gateway_order",
    "ugos_net_proxy_set",
    "ugos_net_bond_candidates",
    "ugos_net_bond_create",
    "ugos_net_bond_delete",
    "ugos_net_bridge_candidates",
    "ugos_net_bridge_create",
    "ugos_net_bridge_create_virtual",
    "ugos_net_bridge_delete",
    "ugos_net_wifi_status",
    "ugos_net_wifi_radio",
    "ugos_net_wifi_list",
    "ugos_net_wifi_connect",
    "ugos_net_wifi_forget",
    "ugos_net_wifi_edit",
    "ugos_net_flow_list",
    "ugos_net_flow_services",
    "ugos_net_flow_add",
    "ugos_net_flow_remove",
    "ugos_net_flow_enable",
];

#[test]
fn all_tools_registered() {
    let server = make_server();
    let tools = server.tool_router.list_all();
    let names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();

    let expected = EXPECTED_TOOLS;

    for name in &expected {
        assert!(
            names.contains(name),
            "missing MCP tool: {name}\nregistered: {names:?}"
        );
    }

    assert_eq!(
        names.len(),
        expected.len(),
        "tool count mismatch: got {}, expected {}\nextra: {:?}",
        names.len(),
        expected.len(),
        names
            .iter()
            .filter(|n| !expected.contains(n))
            .collect::<Vec<_>>()
    );
}

/// The tools that delete name `force`, so that a refusal can be overridden,
/// and say in the description that the user is to be asked first.
#[test]
fn the_delete_tools_take_force_and_tell_the_model_to_ask() {
    let server = make_server();
    let tools = server.tool_router.list_all();
    for name in [
        "ugos_share_delete",
        "ugos_user_delete",
        "ugos_net_bond_delete",
        "ugos_net_bridge_delete",
    ] {
        let tool = tools.iter().find(|t| t.name == name).unwrap();
        let has_force = tool
            .input_schema
            .get("properties")
            .and_then(|value| value.as_object())
            .is_some_and(|map| map.contains_key("force"));
        assert!(has_force, "{name} has no force parameter");
    }
    for name in ["ugos_share_delete", "ugos_user_delete"] {
        let tool = tools.iter().find(|t| t.name == name).unwrap();
        let description = tool.description.as_ref().unwrap();
        assert!(
            description.contains("ask the user"),
            "{name}: {description}"
        );
    }
}

#[test]
fn all_tools_have_descriptions() {
    let server = make_server();
    let tools = server.tool_router.list_all();
    for tool in &tools {
        assert!(
            tool.description.is_some(),
            "tool {} has no description",
            tool.name
        );
        let desc = tool.description.as_ref().unwrap();
        assert!(!desc.is_empty(), "tool {} has empty description", tool.name);
    }
}

#[test]
fn tool_names_follow_convention() {
    let server = make_server();
    let tools = server.tool_router.list_all();
    for tool in &tools {
        assert!(
            tool.name.starts_with("ugos_"),
            "tool {} doesn't start with ugos_",
            tool.name
        );
        assert!(
            tool.name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c == '_'),
            "tool {} has non-lowercase chars",
            tool.name
        );
    }
}

#[test]
fn target_list_works_without_auth() {
    let server = make_server();
    let result = server.ugos_target_list();
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0]["name"], "test");
    assert_eq!(parsed[0]["host"], "127.0.0.1");
}

#[test]
fn the_server_names_itself_not_the_library() {
    use rmcp::ServerHandler;

    let info = make_server().get_info();
    // Without with_server_info rmcp calls Implementation::from_build_env() and
    // thereby reports itself. A client would then see "rmcp" instead of the
    // server it is talking to.
    assert_eq!(info.server_info.name, "ugos-mcp");
    assert_eq!(info.server_info.version, env!("CARGO_PKG_VERSION"));
}

/// The tool reference the agent skill of `ugos-cli` carries, from the router.
fn skill_mcp_reference() -> String {
    use std::fmt::Write as _;
    let server = make_server();
    let mut tools = server.tool_router.list_all();
    tools.sort_by(|a, b| a.name.cmp(&b.name));
    let mut out = String::from(
        "# ugos-mcp tools\n\nGenerated from the build by a test in ugos-mcp. \
         Started with `ugos-mcp`.\n\n",
    );
    for tool in tools {
        let _ = writeln!(out, "## {}\n", tool.name);
        if let Some(description) = &tool.description {
            let _ = writeln!(out, "{description}\n");
        }
        let required: Vec<&str> = tool
            .input_schema
            .get("required")
            .and_then(|value| value.as_array())
            .map(|names| names.iter().filter_map(|name| name.as_str()).collect())
            .unwrap_or_default();
        if let Some(properties) = tool
            .input_schema
            .get("properties")
            .and_then(|value| value.as_object())
        {
            for (name, schema) in properties {
                let needed = if required.contains(&name.as_str()) {
                    " (required)"
                } else {
                    ""
                };
                let what = schema
                    .get("description")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default();
                let _ = writeln!(out, "- `{name}`{needed}: {what}");
            }
            out.push('\n');
        }
    }
    out
}

/// `crates/ugos-cli/skill/references/mcp.md` is checked in, because ugos-cli
/// cannot depend on ugos-mcp to generate it. This test is what keeps the two
/// from drifting. After changing a tool, regenerate with
/// `UGOS_SKILL_BLESS=1 cargo test -p ugos-mcp skill_mcp_reference`.
#[test]
fn skill_mcp_reference_is_current() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../ugos-cli/skill/references/mcp.md");
    let expected = skill_mcp_reference();
    if std::env::var_os("UGOS_SKILL_BLESS").is_some() {
        std::fs::write(&path, &expected).unwrap();
        return;
    }
    let found = std::fs::read_to_string(&path).unwrap();
    assert!(
        found == expected,
        "the skill's MCP reference is out of date; run \
         `UGOS_SKILL_BLESS=1 cargo test -p ugos-mcp skill_mcp_reference` and commit the result"
    );
}
