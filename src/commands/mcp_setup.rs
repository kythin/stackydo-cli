use crate::cli::args::McpSetupArgs;
use crate::error::Result;

pub fn execute(args: &McpSetupArgs) -> Result<()> {
    let scope = args.scope.as_deref().unwrap_or("project");
    let name = args.name.as_deref().unwrap_or("stackydo");

    println!("Running: claude mcp add --scope {scope} {name} -- stackydo-mcp");

    let status = std::process::Command::new("claude")
        .args(["mcp", "add", "--scope", scope, name, "--", "stackydo-mcp"])
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("Registered stackydo-mcp as '{name}' (scope: {scope}).");
            println!("Restart Claude Code for changes to take effect.");
            Ok(())
        }
        Ok(s) => {
            eprintln!("claude mcp add exited with {s}");
            std::process::exit(s.code().unwrap_or(1));
        }
        Err(e) => {
            eprintln!("Failed to run `claude`: {e}");
            eprintln!("Is Claude Code installed and on your PATH?");
            eprintln!();
            eprintln!("Run manually:");
            eprintln!("  claude mcp add --scope {scope} {name} -- stackydo-mcp");
            std::process::exit(1);
        }
    }
}
