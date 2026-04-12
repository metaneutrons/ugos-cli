# Architecture

## Crate Layout

```
ugos-cli/
├── crates/
│   ├── ugos-client/     # API client library (SSOT for types, auth, API calls)
│   ├── ugos-cli/        # CLI binary (clap, table/json output)
│   └── ugos-mcp/        # MCP stdio server (JSON tool responses)
├── .kiro/
│   ├── docs/            # API research & reference
│   └── steering/        # Project steering (this file)
└── Cargo.toml           # Workspace root
```

## Dependency Graph

```
ugos-client          ← owns all business logic, auth, types, API calls
    ↑
    ├── ugos-cli     ← thin wrapper: clap CLI → client calls → table/json
    └── ugos-mcp     ← thin wrapper: MCP stdio → client calls → JSON tools
```

## ugos-client

The single source of truth for:
- **Types**: all request/response structs (serde, shared by CLI and MCP)
- **Auth**: RSA key exchange, password encryption, login, token management
- **API modules**: one module per UGOS app (kvm, docker, filemgr, etc.)
- **Error handling**: typed errors via thiserror

### Module structure

```
ugos-client/src/
├── lib.rs
├── auth.rs              # RSA + login flow
├── client.rs            # UgosClient: reqwest + token + cookie jar
├── error.rs             # Error types
├── types/
│   ├── mod.rs
│   ├── common.rs        # ApiResponse<T>, shared types
│   └── kvm.rs           # KVM-specific types (VmSummary, VmDetail, etc.)
└── api/
    ├── mod.rs
    └── kvm.rs           # KVM API methods on UgosClient
```

### API method pattern

Each API module extends `UgosClient` via a trait:

```rust
pub trait KvmApi {
    async fn vm_list(&self) -> Result<Vec<VmSummary>>;
    async fn vm_show(&self, name: &str) -> Result<VmDetail>;
    async fn vm_start(&self, name: &str) -> Result<()>;
    async fn vm_stop(&self, name: &str, force: bool) -> Result<()>;
    async fn vm_reboot(&self, name: &str, force: bool) -> Result<()>;
    async fn vm_delete(&self, name: &str) -> Result<()>;
    async fn host_info(&self) -> Result<HostInfo>;
    async fn snapshot_list(&self, vm: &str) -> Result<Vec<Snapshot>>;
    async fn snapshot_create(&self, vm: &str, name: &str) -> Result<()>;
    async fn snapshot_delete(&self, vm: &str, name: &str) -> Result<()>;
    async fn snapshot_revert(&self, vm: &str, name: &str) -> Result<()>;
    async fn snapshot_rename(&self, vm: &str, old: &str, new: &str) -> Result<()>;
    async fn network_list(&self) -> Result<Vec<NetworkSummary>>;
    async fn network_show(&self, name: &str) -> Result<NetworkDetail>;
    async fn storage_list(&self) -> Result<Vec<StorageInfo>>;
    async fn image_list(&self) -> Result<Vec<ImageInfo>>;
    async fn usb_list(&self, vm: &str) -> Result<Vec<UsbDevice>>;
}

impl KvmApi for UgosClient { ... }
```

### Name resolution

All public API methods accept display names or UUIDs. The client resolves
display names to UUIDs internally by calling `ShowLocalVirtualList` (cached
for the duration of the operation).

## ugos-cli

Thin CLI frontend:

```
ugos-cli/src/
├── main.rs              # Entry point, tracing setup
├── cli.rs               # clap derive structs
├── commands/
│   ├── mod.rs
│   └── vm.rs            # VM subcommand handlers
└── output.rs            # Table/JSON formatting
```

### CLI structure

```
ugos [global-opts] <resource> <action> [args]

Global options:
  --host <host>          UGOS NAS address (env: UGOS_HOST)
  --user <user>          Username (env: UGOS_USER)
  --password <password>  Password (env: UGOS_PASSWORD)
  --output <format>      Output format: table (default), json
  --no-cache             Skip session token cache

Resources (MVP):
  vm                     Virtual machine management
  network                KVM network management
  storage                KVM storage management
  image                  KVM image management

VM actions:
  list                   List all VMs
  show <name>            Show VM details
  start <name>           Power on
  stop <name>            Graceful shutdown
  stop --force <name>    Forced shutdown
  reboot <name>          Reboot
  reboot --force <name>  Forced restart
  delete <name>          Delete VM

  snapshot list <vm>
  snapshot create <vm> <snapshot-name>
  snapshot delete <vm> <snapshot-name>
  snapshot revert <vm> <snapshot-name>
  snapshot rename <vm> <old-name> <new-name>
```

## ugos-mcp

MCP stdio server. One tool per operation:

```
ugos_vm_list, ugos_vm_show, ugos_vm_start, ugos_vm_stop,
ugos_vm_reboot, ugos_vm_delete, ugos_snapshot_list,
ugos_snapshot_create, ugos_snapshot_delete, ugos_snapshot_revert,
ugos_network_list, ugos_network_show, ugos_storage_list,
ugos_image_list, ugos_host_info
```

Config via env vars only (UGOS_HOST, UGOS_USER, UGOS_PASSWORD).

## Token Persistence

### CLI
- Session cached at `~/.config/ugos-cli/session.json`
- Contains: host, user, token, cookie, created_at
- No password on disk
- Re-auth on missing/expired/rejected (code 1024)
- `--no-cache` flag to skip

### MCP
- Token held in memory (long-running process)
- Lazy auth on first tool call
- Re-auth on 1024

## Auth Implementation Notes (Rust)

The auth module (`auth.rs`) must implement this exact flow:

```rust
// 1. Fetch RSA public key
//    POST /verify/check {"username": "..."}
//    → parse x-rsa-token response header
//    → base64-decode → PEM string → rsa::RsaPublicKey

// 2. Encrypt password
//    rsa::RsaPublicKey::encrypt(&mut rng, Pkcs1v15Encrypt, password.as_bytes())
//    → base64-encode the ciphertext
//    NOTE: must use Pkcs1v15Encrypt, NOT Oaep

// 3. Login
//    POST /verify/login {"username", "password": "<b64-ciphertext>", "keepalive": true, "otp": false}
//    → extract data.token from JSON response body
//    → extract Set-Cookie headers (reqwest cookie_store handles this)

// 4. All subsequent requests:
//    → reqwest cookie jar sends cookies automatically
//    → append ?token=<token> to every URL (or &token= if query exists)
```

### reqwest setup

```rust
// Client must be built with:
// - cookie_store(true) for automatic cookie handling
// - danger_accept_invalid_certs(true) for self-signed NAS certs
// - no default headers for token — it goes in the URL, not a header
let client = reqwest::Client::builder()
    .cookie_store(true)
    .danger_accept_invalid_certs(true)
    .build()?;
```

### Token refresh

On any API response with `code: 1024`, the client must:
1. Re-run the full auth flow (steps 1-3)
2. Retry the failed request once with the new token
3. If it fails again, propagate the error

### Session file (`~/.config/ugos-cli/session.json`)

```json
{
  "host": "192.168.2.5",
  "port": 9443,
  "user": "fabian",
  "token": "E430830B3EDA4A8C9E3F8F3A462050F0",
  "cookies": [
    {"name": "token", "value": "m1u985so...", "path": "/", "http_only": true},
    {"name": "token_uid", "value": "1000", "path": "/", "http_only": true}
  ],
  "created_at": "2026-04-12T15:10:00Z"
}
```

The CLI loads this on startup. If the host/user match, it injects the cookies
into the reqwest cookie jar and uses the saved token. On 1024, it re-auths
and overwrites the file.

## Error Handling Strategy

- `ugos-client`: returns `Result<T, ugos_client::Error>` (thiserror)
- `ugos-cli`: converts to anyhow, prints human-readable messages
- `ugos-mcp`: converts to MCP error responses
- API code 1024 triggers automatic re-auth + retry (once)
