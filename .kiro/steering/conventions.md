# Conventions

## Code Style

- **Edition 2024** idioms: use `gen` blocks, `let-else`, `?` operator everywhere
- **No `.unwrap()` or `.expect()`** in library code (warned by clippy)
- **No `println!`/`eprintln!`** — use `tracing` for logging, structured output for CLI
- **All public items documented** with `///` doc comments
- **Module-level docs** with `//!` in every file

## Naming

- Types: `VmSummary`, `VmDetail`, `NetworkSummary` (no `Kvm` prefix — module provides context)
- API traits: `KvmApi`, `DockerApi` (one per UGOS app)
- CLI commands: `ugos <resource> <action>` (noun-verb)
- MCP tools: `ugos_<resource>_<action>` (underscore-separated)
- Serde: `#[serde(rename_all = "camelCase")]` to match UGOS JSON

## Error Handling

- Library: `thiserror` with `#[error("...")]` on all variants
- Binaries: `anyhow` with `.context("doing X")?`
- Never swallow errors silently

## API Client Pattern

```rust
// Endpoints returning {data: {result: [T]}}
async fn vm_list(&self) -> Result<Vec<VmSummary>> {
    let resp: ApiResponse<ResultWrapper<Vec<VmSummary>>> = self
        .get("kvm/manager/ShowLocalVirtualList")
        .await?;
    Ok(resp.data.result)
}

// Endpoints returning {data: T} directly (no result wrapper)
async fn vm_show(&self, name: &str) -> Result<VmDetail> {
    let resp: ApiResponse<VmDetail> = self
        .get_with_params("kvm/manager/ShowLocalVirtualMachine", &[("name", name)])
        .await?;
    Ok(resp.data)
}

// Endpoints returning {data: {result: "successful"}}
async fn vm_start(&self, name: &str, display_name: &str) -> Result<()> {
    let resp: ApiResponse<ResultWrapper<String>> = self
        .get_with_params("kvm/manager/PowerOn", &[("name", name), ("virtualMachineDisplayName", display_name)])
        .await?;
    // resp.data.result == "successful"
    Ok(())
}
```

## Testing

- Unit tests for type deserialization (embed JSON fixtures)
- Integration tests against real NAS (behind feature flag `integration`)
- `cargo test` must pass without network access

## Dependencies

- Minimize dependency count
- Prefer `rustls` over `openssl` (no system deps)
- No `async-trait` — use native async trait (Rust 1.85+)
- No `derive_builder` — manual builders where needed
