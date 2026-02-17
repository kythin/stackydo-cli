use crate::cli::args::ImportArgs;
use crate::commands::util::parse_due_date;
use crate::error::{Result, TodoError};
use crate::model::task::{Priority, Task, TaskImportInput, TaskStatus};
use crate::storage::manifest_store::ManifestStore;
use crate::storage::task_store::TaskStore;
use std::io::Read;

pub fn execute(args: &ImportArgs) -> Result<()> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;

    let items: Vec<TaskImportInput> = match args.format.as_str() {
        "json" => serde_json::from_str(&input)?,
        "yaml" => serde_yaml::from_str(&input)?,
        other => {
            return Err(TodoError::Other(format!(
                "Unknown format: '{other}'. Use: json, yaml"
            )));
        }
    };

    if items.is_empty() {
        println!("No tasks to import.");
        return Ok(());
    }

    let store = TaskStore::new();
    let manifest_store = ManifestStore::new();
    let working_dir = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| ".".to_string());

    let mut created_ids: Vec<String> = Vec::new();

    for item in items {
        let id = ulid::Ulid::new().to_string();
        let mut task = Task::new(id.clone(), item.title, working_dir.clone());

        // Priority
        if let Some(ref pri_str) = item.priority {
            task.frontmatter.priority = Some(
                pri_str
                    .parse::<Priority>()
                    .map_err(TodoError::Other)?,
            );
        }

        // Tags
        if let Some(ref tags) = item.tags {
            if let Err(e) = manifest_store.register_tags(tags) {
                eprintln!("Warning: failed to register tags: {e}");
            }
            task.frontmatter.tags = tags.clone();
        }

        // Stack
        if let Some(ref stack) = item.stack {
            if let Err(e) = manifest_store.register_stack(stack) {
                eprintln!("Warning: failed to register stack: {e}");
            }
            task.frontmatter.stack = Some(stack.clone());
        }

        // Body
        if let Some(ref body) = item.body {
            task.body = body.clone();
        }

        // Due
        if let Some(ref due_str) = item.due {
            task.frontmatter.due = Some(parse_due_date(due_str)?);
        }

        // Status
        if let Some(ref status_str) = item.status {
            task.frontmatter.status = status_str
                .parse::<TaskStatus>()
                .map_err(TodoError::Other)?;
        }

        store.save(&task)?;
        created_ids.push(id);
    }

    println!("Imported {} task(s):", created_ids.len());
    for id in &created_ids {
        println!("  {}", &id[..10]);
    }

    Ok(())
}
