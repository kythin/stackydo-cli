use rmcp::{ServiceExt, transport::io::stdio};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Ensure storage directory exists
    stackstodo::storage::paths::TodoPaths::ensure_root()?;

    let server = stackstodo::mcp::StackstodoMcp::new();
    let service = server.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
