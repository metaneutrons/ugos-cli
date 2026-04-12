//! UGOS MCP server — stdio entry point.

use anyhow::{Context, Result, bail};
use rmcp::ServiceExt;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    let targets = ugos_mcp::parse_targets_from_env();
    if targets.is_empty() {
        bail!("no targets configured — set UGOS_HOST/UGOS_USER/UGOS_PASSWORD env vars");
    }

    tracing::info!("configured {} target(s)", targets.len());
    for t in &targets {
        tracing::info!("  {} → {}:{}", t.name, t.host, t.port);
    }

    let server = ugos_mcp::UgosMcp::new(targets);

    let transport = rmcp::transport::stdio();
    let service = server
        .serve(transport)
        .await
        .context("MCP server failed to start")?;
    let _quit = service.waiting().await.context("MCP server error")?;

    Ok(())
}
