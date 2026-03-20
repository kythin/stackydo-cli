use crate::cli::args::CreateArgs;
use crate::commands::util::parse_due_date;
use crate::context::dir_context;
use crate::error::{Result, TodoError};
use crate::model::task::{Dependency, DependencyType, Priority, Task};
use crate::storage::manifest_store::ManifestStore;
use crate::storage::task_store::TaskStore;
use std::path::PathBuf;

/// Execute headless task creation.
/// Returns the new task's ULID on success.
pub fn execute(args: &CreateArgs) -> Result<String> {
    let store = TaskStore::new();
    let manifest_store = ManifestStore::new();

    // Generate ULID
    let id = ulid::Ulid::new().to_string();

    // Determine context path
    let context_path = match &args.context_path {
        Some(p) => PathBuf::from(p),
        None => std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    };

    // Capture context
    let mut ctx = dir_context::capture(&context_path);

    // Apply context overrides from CLI flags
    if args.context_path.is_some() {
        ctx.path = args.context_path.clone();
    }
    if let Some(ref line_spec) = args.context_path_line {
        let parts: Vec<&str> = line_spec.splitn(2, ':').collect();
        ctx.line = parts.first().and_then(|s| s.parse().ok());
        ctx.column = parts.get(1).and_then(|s| s.parse().ok());
    }
    if args.context_path_lookfor.is_some() {
        ctx.lookfor = args.context_path_lookfor.clone();
    }

    // Build body from trailing args
    let body = if args.body.is_empty() {
        String::new()
    } else {
        args.body.join(" ")
    };

    // Determine title — treat whitespace-only explicit title as if it wasn't provided
    let explicit_title = args
        .title
        .as_deref()
        .filter(|t| !t.trim().is_empty())
        .map(|t| t.to_string());
    let title = explicit_title
        .or_else(|| {
            // Use first line of body if no explicit title
            body.lines().next().map(|l| {
                let t = l.trim();
                if t.chars().count() > 80 {
                    format!("{}...", t.chars().take(77).collect::<String>())
                } else {
                    t.to_string()
                }
            })
        })
        .unwrap_or_else(|| "Untitled".into());

    // Create the task
    let cwd = std::env::current_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| ".".into());
    let mut task = Task::new(id.clone(), title, cwd);
    task.frontmatter.context = ctx;
    task.body = body;

    // Parse optional fields
    if let Some(ref p) = args.priority {
        // "none" or empty means no priority (same behaviour as update --priority none)
        if !p.eq_ignore_ascii_case("none") && !p.is_empty() {
            task.frontmatter.priority = Some(p.parse::<Priority>().map_err(TodoError::Other)?);
        }
    }

    if let Some(ref tags_csv) = args.tags {
        let tags: Vec<String> = tags_csv
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        // Register tags in manifest
        if let Err(e) = manifest_store.register_tags(&tags) {
            eprintln!("Warning: failed to register tags in manifest: {e}");
        }
        task.frontmatter.tags = tags;
    }

    if let Some(ref stack) = args.stack {
        let stack = stack.trim().to_string();
        if !stack.is_empty() {
            if let Err(e) = manifest_store.register_stack(&stack) {
                eprintln!("Warning: failed to register stack in manifest: {e}");
            }
            task.frontmatter.stack = Some(stack);
        }
    }

    if let Some(ref due_str) = args.due {
        task.frontmatter.due = Some(parse_due_date(due_str)?);
    }

    // Wire dependencies
    for raw_id in &args.blocked_by {
        let dep_task = crate::commands::show::resolve_task_pub(&store, raw_id)?;
        task.frontmatter.dependencies.push(Dependency {
            task_id: dep_task.frontmatter.id,
            dep_type: DependencyType::BlockedBy,
        });
    }
    for raw_id in &args.blocks {
        let dep_task = crate::commands::show::resolve_task_pub(&store, raw_id)?;
        task.frontmatter.dependencies.push(Dependency {
            task_id: dep_task.frontmatter.id,
            dep_type: DependencyType::Blocks,
        });
    }
    for raw_id in &args.related_to {
        let dep_task = crate::commands::show::resolve_task_pub(&store, raw_id)?;
        task.frontmatter.dependencies.push(Dependency {
            task_id: dep_task.frontmatter.id,
            dep_type: DependencyType::RelatedTo,
        });
    }

    // Wire parent/subtask
    if let Some(ref parent_raw) = args.parent {
        let mut parent_task = crate::commands::show::resolve_task_pub(&store, parent_raw)?;
        let parent_id = parent_task.frontmatter.id.clone();
        task.frontmatter.parent_id = Some(parent_id.clone());

        // Add this task as subtask of parent
        if !parent_task.frontmatter.subtask_ids.contains(&id) {
            parent_task.frontmatter.subtask_ids.push(id.clone());
            parent_task.frontmatter.modified = chrono::Utc::now();
            store.save(&parent_task)?;
        }
    }

    // Save first, then allocate short ID only on success
    store.save(&task)?;

    // Assign short ID after successful save to avoid burning IDs on failure
    match manifest_store.allocate_short_id() {
        Ok(sid) => {
            task.frontmatter.short_id = Some(sid);
            if let Err(e) = store.save(&task) {
                eprintln!("Warning: failed to save short ID: {e}");
            }
        }
        Err(e) => eprintln!("Warning: failed to allocate short ID: {e}"),
    }

    // Print ULID to stdout for shell integration (parseable)
    // Users can do: export STACKYDO_LAST_ID=$(stackydo create --title "..." -- body)
    // Short ID is visible via `show` and `list` output.
    println!("{id}");

    Ok(id)
}
