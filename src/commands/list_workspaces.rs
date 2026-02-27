use crate::cli::args::ListWorkspacesArgs;
use crate::error::Result;
use crate::storage::workspace::discover_workspaces;

pub fn execute(args: &ListWorkspacesArgs) -> Result<()> {
    let workspaces = discover_workspaces();

    if workspaces.is_empty() {
        if args.json {
            println!("[]");
        } else {
            println!("No stackydo workspaces found.");
        }
        return Ok(());
    }

    if args.json {
        let json = serde_json::to_string_pretty(&workspaces)?;
        println!("{json}");
        return Ok(());
    }

    // Human-readable output
    for (i, ws) in workspaces.iter().enumerate() {
        if i > 0 {
            println!();
        }

        // Header line
        if ws.is_default {
            println!("{}/ (global default)", ws.store_dir.display());
        } else if let Some(ref name) = ws.project_name {
            if let Some(ref cfg) = ws.config_path {
                println!("{} ({name})", cfg.display());
            } else {
                println!("{}/ ({name})", ws.store_dir.display());
            }
        } else if let Some(ref cfg) = ws.config_path {
            println!("{}", cfg.display());
        } else {
            println!("{}/", ws.store_dir.display());
        }

        // Details line
        let stacks_str = if ws.stacks.is_empty() {
            String::new()
        } else {
            format!("  stacks: {}", ws.stacks.join(", "))
        };

        if ws.is_default || ws.config_path.is_none() {
            println!(
                "  tasks: {}{}",
                ws.task_count, stacks_str
            );
        } else {
            println!(
                "  store: {}/  tasks: {}{}",
                ws.store_dir.display(),
                ws.task_count,
                stacks_str
            );
        }
    }

    Ok(())
}
