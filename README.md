# ugos-cli

[![CI](https://github.com/metaneutrons/ugos-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/metaneutrons/ugos-cli/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/ugos-client.svg)](https://crates.io/crates/ugos-client)
[![docs.rs](https://docs.rs/ugos-client/badge.svg)](https://docs.rs/ugos-client)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)

CLI, MCP server, and Rust client library for managing **UGREEN NAS** (UGOS) devices.

UGOS provides a web UI for KVM virtual machine management but no CLI or API documentation. Direct use of `virsh`/`qemu` on the host causes UGOS to lose track of VMs. This project provides a programmatic interface through the official (undocumented) API.

> **⚠️ Work in Progress** — This project currently implements a subset of the UGOS API focused on KVM virtual machine management. See the [implementation status](#implementation-status) below.

## Crates

| Crate | Description |
|-------|-------------|
| [`ugos-client`](crates/ugos-client) | API client library — auth, types, API calls |
| [`ugos-cli`](crates/ugos-cli) | Command-line interface |
| [`ugos-mcp`](crates/ugos-mcp) | MCP server for AI-assisted NAS management |

## Quick Start

### CLI

```bash
# Set credentials (or use --host, --user, --password flags)
export UGOS_HOST=192.168.2.5
export UGOS_USER=admin
export UGOS_PASSWORD=<password>

# List VMs
ugos-cli vm list

# Show VM details
ugos-cli vm show CachyOS

# Power management
ugos-cli vm start CachyOS
ugos-cli vm stop CachyOS
ugos-cli vm stop --force CachyOS

# Snapshots
ugos-cli vm snapshot list CachyOS
ugos-cli vm snapshot create CachyOS my-snapshot

# Other resources
ugos-cli network list
ugos-cli storage list
ugos-cli image list
ugos-cli info

# JSON output
ugos-cli -o json vm list
```

### Library

```rust
use ugos_client::{UgosClient, Credentials};
use ugos_client::api::kvm::KvmApi;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let creds = Credentials {
        username: "admin".into(),
        password: "<password>".into(),
    };

    let client = UgosClient::connect("192.168.2.5", 9443, creds).await?;
    let vms = client.vm_list().await?;

    for vm in &vms {
        println!("{}: {}", vm.vir_display_name, vm.status);
    }

    Ok(())
}
```

### MCP Server

Configure in your MCP client (e.g. Kiro, Claude Desktop):

```json
{
  "mcpServers": {
    "ugos": {
      "command": "ugos-mcp",
      "env": {
        "UGOS_HOST": "192.168.2.5",
        "UGOS_USER": "admin",
        "UGOS_PASSWORD": "<password>"
      }
    }
  }
}
```

Multiple NAS targets are supported:

```json
{
  "mcpServers": {
    "ugos": {
      "command": "ugos-mcp",
      "env": {
        "UGOS_HOST_1": "192.168.2.5",
        "UGOS_USER_1": "admin",
        "UGOS_PASSWORD_1": "<password>",
        "UGOS_NAME_1": "picard",
        "UGOS_HOST_2": "192.168.2.4",
        "UGOS_USER_2": "admin",
        "UGOS_PASSWORD_2": "<password>",
        "UGOS_NAME_2": "kirk"
      }
    }
  }
}
```

## Installation

### From GitHub Releases

Download pre-built binaries from the [releases page](https://github.com/metaneutrons/ugos-cli/releases).

Available for:
- Linux (x86_64, aarch64)
- macOS (x86_64, aarch64)
- Windows (x86_64, aarch64)

### From Source

```bash
cargo install --git https://github.com/metaneutrons/ugos-cli ugos-cli
cargo install --git https://github.com/metaneutrons/ugos-cli ugos-mcp
```

### Library

```toml
[dependencies]
ugos-client = "0.1"
```

## Implementation Status

### Implemented ✅

| Resource | Operations |
|----------|-----------|
| **VM** | list, show, start, stop, force-stop, reboot, force-reboot, delete |
| **Snapshot** | list, create, delete, revert, rename |
| **Network** | list, show |
| **Storage** | list |
| **Image** | list |
| **USB** | list |
| **Host** | info (CPU cores, memory) |
| **Auth** | RSA key exchange, PKCS1v1.5 encryption, session tokens, auto re-auth |

### Not Yet Implemented

| Resource | Notes |
|----------|-------|
| VM create/update | Complex schema, needs UI validation logic |
| OVA import/export | |
| Image upload | |
| VNC link management | |
| Docker management | Separate UGOS app |
| File management | Separate UGOS app |
| Non-KVM modules | Photo, video, music, backup, etc. |

## Authentication

UGOS uses a multi-step auth flow:

1. **RSA key exchange** — `POST /verify/check` returns an RSA public key
2. **Password encryption** — PKCS1v1.5 padding (not OAEP)
3. **Login** — `POST /verify/login` returns a session token + cookies
4. **Authenticated requests** — cookies + `?token=` query parameter

The client handles this automatically, including transparent re-authentication when tokens expire (UGOS error code 1024).

## Tested Devices

| NAS | Model | UGOS Version |
|-----|-------|-------------|
| picard | DXP480T Plus | 1.14.1.0107 |
| kirk | DXP4800 Plus | 1.14.x |

## Requirements

- Rust 1.85+ (edition 2024)
- UGOS NAS with KVM app installed
- Network access to the NAS (HTTPS port 9443)

## License

This project is licensed under the [GNU General Public License v3.0](LICENSE).
