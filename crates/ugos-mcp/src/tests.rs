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
        },
    }])
}

#[test]
fn all_tools_registered() {
    let server = make_server();
    let tools = server.tool_router.list_all();
    let names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();

    let expected = [
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
    ];

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
