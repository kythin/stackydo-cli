use clap::Parser;
use rmcp::{transport::io::stdio, ServiceExt};

#[derive(Parser)]
#[command(name = "stackydo-mcp")]
#[command(
    version,
    about = "MCP server for stackydo — stdio transport for use with Claude and other MCP clients"
)]
struct Args {}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse args so --help / --version work; no runtime flags yet.
    let _args = Args::parse();

    // Resolve task store root from env / stackydo.json / default
    stackydo::storage::paths::TodoPaths::init();

    // Ensure storage directory exists
    stackydo::storage::paths::TodoPaths::ensure_root()?;

    let server = stackydo::mcp::StackydoMcp::new();
    let service = server.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
