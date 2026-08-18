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
export UGOS_HOST=192.168.1.10
export UGOS_USER=admin          # UGOS_USERNAME works too
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
ugos-cli vm snapshot create CachyOS      # UGOS names it after the creation time

# Create a VM
ugos-cli vm create debian --cores 4 --memory 8g --disk 50g \
    --iso /volume1/iso/debian.iso

# Print the request body instead of creating anything (works offline)
ugos-cli vm create debian --cores 4 --memory 8g --disk 50g --dry-run

# Upload an ISO, from a local file or a URL
ugos-cli image upload ~/Downloads/debian.iso
ugos-cli image upload https://example.org/debian.iso --name debian-13

# Host load and all VMs in one call
ugos-cli overview

# How much space KVM uses, per volume and per VM
ugos-cli storage df

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

    let client = UgosClient::connect("192.168.1.10", 9443, creds).await?;
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
        "UGOS_HOST": "192.168.1.10",
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
        "UGOS_HOST_1": "192.168.1.10",
        "UGOS_USER_1": "admin",
        "UGOS_PASSWORD_1": "<password>",
        "UGOS_NAME_1": "nas1",
        "UGOS_HOST_2": "192.168.1.11",
        "UGOS_USER_2": "admin",
        "UGOS_PASSWORD_2": "<password>",
        "UGOS_NAME_2": "nas2"
      }
    }
  }
}
```

## Creating and updating VMs

`vm create` and `vm update` map directly onto the fields of the
`CreateVirtualMachine` and `UpdateVirtualMachine` request bodies and take the
same flags. Device flags are repeatable and take either a short form or a
`key=value,...` list; sizes accept `k`, `m`, `g` and `t` suffixes, and a bare
number means MiB. UGOS itself counts in KiB — the CLI converts, so `--disk 40g`
really is 40 GiB.

```bash
# Two disks on different buses, two NICs, two ISOs, USB passthrough
ugos-cli vm create win11 \
    --os windows --os-version win11 \
    --cores 8 --memory 16g \
    --disk size=60g,bus=ide \
    --disk size=500g,bus=sata \
    --iso /volume1/iso/win11.iso \
    --iso /volume1/iso/virtio-drivers.iso \
    --nic name=vnet-bridge0,type=e1000e \
    --nic name=vnet-nat,type=e1000e,mac=52:54:00:12:34:56 \
    --usb vendor-id=0x8087,product-id=0x0033,bus-id=1,device-id=4 \
    --graphics qxl --keyboard de --autostart
```

| Flag | Short form | Keys |
|------|-----------|------|
| `--disk` | `40g` | `size`, `bus`, `dev`, `path`, `order` |
| `--iso` | `/path.iso` | `path`, `dev`, `order` |
| `--nic` | `vnet-bridge0` | `name`, `type`, `mac` |
| `--usb` | — | `vendor-id`, `product-id`, `bus-id`, `device-id`, `vendor-name`, `product-name` |
| `--share` | — | passed through verbatim |

On `create`, device names (`vda`, `sda`, `hda`, …) are derived from the bus
type and boot order is numbered disks-first, unless `dev=` or `order=` says
otherwise. On `update`, a newly added disk is left unnamed on purpose, because
UGOS insists on naming it (see below). `--usb` and `--share` also accept a raw
JSON object when the generated body needs to look different.

### Updating

`vm update` starts from the VM's current configuration, so only what a flag
names is changed and everything else is sent back verbatim. The VM has to be
shut off — UGOS answers `3002, Fail to edit virtual machine` for a running one
— and its UUID is always the one of the VM named on the command line.

Besides the flags above, which replace a whole device list, `update` edits
lists incrementally. `--set-*` picks the entry to change with a `match=`
selector (disk by `dev`, ISO by `dev` or path, NIC by name or MAC):

```bash
# Grow a disk, keeping its backing file
ugos-cli vm update CachyOS --set-disk match=vda,size=200g

# Attach a second disk and an install ISO, drop a NIC
ugos-cli vm update CachyOS \
    --add-disk 500g \
    --add-iso /volume1/iso/rescue.iso \
    --rm-nic vnet-nat

# Plain resource changes
ugos-cli vm update CachyOS --cores 8 --memory 32g --autostart

# Preview instead of sending
ugos-cli vm update CachyOS --set-nic match=vnet-bridge0,type=e1000 --dry-run
```

| Operation | Flags |
|-----------|-------|
| Replace the whole list | `--disk`, `--iso`, `--nic`, `--usb`, `--share` |
| Append one entry | `--add-disk`, `--add-iso`, `--add-nic` |
| Edit one entry | `--set-disk`, `--set-iso`, `--set-nic` |
| Remove one entry | `--rm-disk`, `--rm-iso`, `--rm-nic` |

Edits run in the order remove, set, add, so a `--rm-disk vdb --add-disk 100g`
pair really does replace that disk rather than cancelling out — though for a
pure resize `--set-disk` is the better tool, since it keeps the existing
backing file instead of starting from an empty one. A selector that matches
nothing is an error rather than a silent no-op.

### Spec files

Anything the flags do not cover can come from a full JSON spec, which the flags
then override. For `create`, `--dry-run` prints the request body instead of
sending it and needs no NAS connection, so specs can be built and inspected
offline; for `update` it still reads the VM's current configuration first.

```bash
# Clone the configuration of an existing VM under a new name
ugos-cli -o json vm show CachyOS > spec.json
ugos-cli vm create CachyOS-2 --spec-file spec.json --memory 32g

# Import an OVA (parse, then create from the parsed spec)
ugos-cli -o json ova parse /volume1/ova/appliance.ova > spec.json
ugos-cli vm create appliance --spec-file spec.json

# Inspect what would be sent
ugos-cli vm create test --spec-file - --dry-run < spec.json
```

A repeatable flag replaces the corresponding list from the spec file rather
than appending to it. The UUID in a spec file is never used: `create` always
generates a fresh one, and `update` always keeps the one of the VM it was
pointed at.

### What the live tests showed

`create` and `update` were verified end to end against a real NAS (UGOS app
build 656, 2026-08-18): a VM is created and starts; CPU, memory and disk size
are changed and it starts again; a second disk is added and removed; it is
renamed, force-stopped and deleted. Five findings from those runs are baked
into the client:

- **Sizes are KiB, not bytes** — for disks just as for memory. Sending bytes
  asks for 1024 times the intended size; UGOS accepts the create but the
  domain then fails to start.
- **`keyboardLanguage` must be a real QEMU keymap.** The web UI sends
  `en-us`; a plain `en` is accepted by `CreateVirtualMachine` and then makes
  every start fail with `3037, Failed to start`.
- **`storageUUID` is mandatory** on `CreateVirtualMachine` — without it the
  answer is `3000, Fail to create virtual machine`. The client resolves it
  from `storage_name`, so `--storage volume1` is enough, but a `--dry-run`
  body shows it empty because resolving needs the NAS.
- **UGOS assigns the VM UUID itself** and ignores `virtualMachineName` in the
  body, so `vm create` reports the UUID it finds in the listing afterwards.
- **A newly added disk must carry neither `dev` nor `path`.** UGOS assigns
  both and answers `3002, Fail to edit virtual machine` when the body names
  them itself, so `vm create` names its disks and `vm update` leaves new ones
  unnamed. Growing an existing disk with `--set-disk` keeps its backing file.

One difference to the web UI remains: it picks **defaults per OS type** that
this CLI does not — `ide` plus an `e1000e` NIC for Windows and `other`,
`virtio` for Linux. `vm create` always defaults to `virtio`, so a Windows
guest without virtio drivers needs
`--disk size=60g,bus=ide --nic name=vnet-bridge0,type=e1000e`.

Where UGOS answers with a bare code, the client asks the validators the web UI
uses and turns the answer into something readable: a taken VM or network name,
a VM asking for more memory than the host has, or a network still attached to
VMs. Note that `CreateVirtualMachine` does not check memory itself — such a VM
is created and only fails to start.

> **Still inferred.** USB passthrough and shared-directory bodies have never
> been sent to a NAS, and a second NIC was accepted but not exercised on a
> running guest. Use `--dry-run` to inspect a body before sending it.

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
| **VM** | list, show, start, stop, force-stop, reboot, force-reboot, delete, create, update |
| **Snapshot** | list, create, delete, revert, describe |
| **Network** | list, show, create, update, delete |
| **Storage** | list (with VM count), usage, add, delete, df (usage per VM) |
| **Image** | list, upload (file or URL), register, delete, usage |
| **USB** | list |
| **PCI** | list passthrough devices |
| **VNC** | list links, generate noVNC link |
| **OVA** | export, parse |
| **Log** | search audit log, list operators |
| **Host** | info (CPU cores, memory), overview (load plus every VM) |
| **Docker container** | list, show, create, start, stop, restart, kill, remove, update, clone, batch-operate, logs |
| **Docker image** | list, search, download, delete, export, load (URL/path) |
| **Docker registry** | list/add/delete/switch mirror, HTTP proxy get/set |
| **Docker overview** | engine status, resource usage |
| **Docker compose (project)** | list, show, create, start, stop, restart, remove |
| **Auth** | RSA key exchange, PKCS1v1.5 encryption, session tokens, auto re-auth |

`Docker container create/update` were reverse-engineered against the real
`CreateContainer` request body (live-captured 2026-08-06, see
[docs/api-docker.md](docs/api-docker.md)) — this caught two bugs that had never actually
been exercised against a live NAS: `port_mapping` was built with wrong field
names (`hostPort`/`protocol` instead of the real `nasPort`/`portType`), and
`subnet_settings`/`gpu_ids` were entirely missing from `ContainerDetail`.
Both are fixed now; the CLI's `docker container create --port` flag has been
tested end-to-end against a real NAS (nginx container, port mapping,
env vars).

`Docker compose` project management (create/list/show/stop/remove) was
reverse-engineered against the live `CreateProject`/`GetProjectListV3`/
`StopProject`/`DownProject` endpoints (live-captured 2026-08-07, see
[docs/api-docker.md](docs/api-docker.md)) and tested end-to-end on a live NAS (a two-service
nginx+redis project, created, verified running, stopped, and removed). The
`start`/`restart` endpoints (`StartProject`/`RestartProject`) are implemented
by analogy with `StopProject` and the container-level start/restart pattern
but have **not** been live-verified — confirm before relying on them for
anything critical.

### Not Yet Implemented

| Resource | Notes |
|----------|-------|
| Image rename | `RenameImage` answers `successful` and renames nothing; field names unknown |
| OVA import (one step) | `ova parse` reads an OVA into a VM spec; creating the VM from it is still a manual second step |
| Image upload | |
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

| Model | UGOS Version |
|-------|-------------|
| DXP480T Plus | 1.14.1.0107 |
| DXP4800 Plus | 1.14.x |

## Requirements

- Rust 1.85+ (edition 2024)
- UGOS NAS with KVM app installed
- Network access to the NAS (HTTPS port 9443)

## License

This project is licensed under the [GNU General Public License v3.0](LICENSE).
