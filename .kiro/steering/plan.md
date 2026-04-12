# Implementation Plan

## Phase 1: Core Client (ugos-client)

### 1.1 Error types
- `error.rs`: `UgosError` enum with variants for API, auth, network, parse errors
- Map UGOS error codes (1003, 1005, 1024, 9404, 9405) to typed variants

### 1.2 Common types
- `types/common.rs`: `ApiResponse<T>` wrapper matching `{code, msg, data, time}`
- Deserialize with serde, handle both `{data: {result: [...]}}` and `{data: {...}}`

### 1.3 Auth module
- `auth.rs`: RSA public key fetch, PKCS1v1.5 encryption, login flow
- Returns `Session { token, cookies, created_at }`

### 1.4 Client core
- `client.rs`: `UgosClient` struct wrapping `reqwest::Client`
- Constructor takes host, port, optional session
- All requests append `?token=<token>` and send cookies
- Auto re-auth on 1024 (retry once)

### 1.5 KVM types
- `types/kvm.rs`: `VmSummary`, `VmDetail`, `NetworkSummary`, `NetworkDetail`,
  `StorageInfo`, `ImageInfo`, `UsbDevice`, `Snapshot`, etc.

### 1.6 KVM API
- `api/kvm.rs`: `KvmApi` trait with all operations
- Name-to-UUID resolution helper

## Phase 2: CLI (ugos-cli)

### 2.1 CLI structure
- `cli.rs`: clap derive with `Cli > Resource > Action` nesting
- Global options: host, user, password, output format, no-cache

### 2.2 Session cache
- Load/save `~/.config/ugos-cli/session.json`
- Skip with `--no-cache`

### 2.3 VM commands
- `commands/vm.rs`: list, show, start, stop, reboot, delete, snapshot subcommands
- Each command: build client → call API → format output

### 2.4 Output formatting
- `output.rs`: trait `Render` with `table()` and `json()` methods
- Table via `tabled` crate, JSON via `serde_json`

## Phase 3: MCP Server (ugos-mcp)

### 3.1 MCP protocol
- Stdio transport (stdin/stdout JSON-RPC)
- Tool registration with schemas derived from the same types

### 3.2 Tool handlers
- One handler per CLI command, reusing ugos-client

## Build Order

Start with Phase 1 (1.1 → 1.6), then Phase 2 (2.1 → 2.4), then Phase 3.
Each step should compile and pass clippy before moving on.
