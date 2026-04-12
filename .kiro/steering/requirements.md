# Requirements

## Problem Statement

UGREEN NAS (UGOS) provides a web UI for KVM virtual machine management but no
CLI or API documentation. Direct use of `virsh`/`qemu` on the host causes UGOS
to lose track of VMs, leading to unintended deletions. A programmatic interface
through the official (undocumented) API is needed.

## Goals

1. **CLI tool** for managing UGOS NAS resources from the terminal
2. **MCP server** for AI-assisted NAS management
3. **Reusable client library** for Rust applications

## MVP Scope (v0.1)

### Must Have
- [ ] Authentication (RSA key exchange + session tokens)
- [ ] Token caching (`~/.config/ugos-cli/session.json`)
- [ ] Auto re-auth on token expiry (code 1024)
- [ ] VM lifecycle: list, show, start, stop, force-stop, reboot, force-reboot, delete
- [ ] Snapshot management: list, create, delete, revert, rename
- [ ] Name resolution: accept display names or UUIDs
- [ ] Output formats: table (default), JSON
- [ ] Environment variable configuration (UGOS_HOST, UGOS_USER, UGOS_PASSWORD)

### Should Have
- [ ] Network listing and detail
- [ ] Storage listing
- [ ] Image listing
- [ ] Host info (ShowNativeInfo)
- [ ] MCP server with all MVP tools

### Won't Have (v0.1)
- VM creation / update (complex schema, needs UI validation logic)
- OVA import/export
- Image upload
- VNC link management
- Docker management
- File management
- Non-KVM UGOS modules

## Non-Functional Requirements

- **Rust edition 2024**, minimum Rust 1.85
- **No unsafe code** (forbid)
- **Strict lints**: clippy pedantic + nursery, warn on missing docs
- **No dead code**: all public items documented, no unused imports
- **Error handling**: thiserror in library, anyhow in binaries
- **TLS**: rustls (no OpenSSL dependency), accept self-signed certs
- **Cross-platform**: macOS (primary), Linux (secondary)
- **Single binary**: no runtime dependencies

## Target Devices

| NAS | Model | IP | UGOS Version |
|-----|-------|----|-------------|
| picard | DXP480T Plus | 192.168.2.5 | 1.14.1.0107 |
| kirk | DXP4800 Plus | 192.168.2.4 | 1.14.x |
