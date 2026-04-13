//! Tests for MCP tool registration.

#[cfg(test)]
mod tests {
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
            "ugos_snapshot_rename",
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
}
