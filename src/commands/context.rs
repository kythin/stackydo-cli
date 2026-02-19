use crate::context::dir_context;
use crate::error::Result;
use crate::storage::paths::TodoPaths;
use std::path::PathBuf;

/// Execute the headless `context` command: print the context metadata
/// that would be captured if a task were created from the current CWD.
pub fn execute() -> Result<()> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let result = dir_context::capture_full(&cwd);
    let ctx = &result.context;

    println!("Context (would be captured on `stackydo create`):");
    println!();
    println!("  Working dir:     {}", ctx.working_dir);

    if let Some(ref branch) = ctx.git_branch {
        println!("  Git branch:      {branch}");
    } else {
        println!("  Git branch:      (none)");
    }
    if let Some(ref remote) = ctx.git_remote {
        println!("  Git remote:      {remote}");
    }
    if let Some(ref commit) = ctx.git_commit {
        println!("  Git commit:      {commit}");
    }

    if let Some(ref prev) = ctx.session_prev_task_id {
        println!("  Session prev ID: {prev}");
    }

    println!();
    println!("  Config file:     {}", result.config_file_path
        .as_deref()
        .unwrap_or("(none found)"));

    if let Some(ref content) = ctx.todo_context_content {
        println!("  Config content:");
        for line in content.lines() {
            println!("    {line}");
        }
    }

    println!();
    println!("  Task store:      {}", TodoPaths::root().display());
    println!("  Manifest:        {}", TodoPaths::manifest().display());

    Ok(())
}
