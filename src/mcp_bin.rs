use rmcp::{ServiceExt, transport::io::stdio};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Ensure storage directory exists
    stackydo::storage::paths::TodoPaths::ensure_root()?;

    let server = stackydo::mcp::StackydoMcp::new();
    let service = server.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
