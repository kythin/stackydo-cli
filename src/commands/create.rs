use crate::cli::args::CreateArgs;
use crate::context::dir_context;
use crate::error::{Result, TodoError};
use crate::model::task::{Priority, Task};
use crate::storage::manifest_store::ManifestStore;
use crate::storage::task_store::TaskStore;
use chrono::{DateTime, NaiveDateTime, Utc};
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

    // Determine title
    let title = args
        .title
        .clone()
        .or_else(|| {
            // Use first line of body if no explicit title
            body.lines().next().map(|l| {
                let t = l.trim();
                if t.len() > 80 {
                    format!("{}...", &t[..77])
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
        task.frontmatter.priority = Some(
            p.parse::<Priority>()
                .map_err(|e| TodoError::Other(e))?,
        );
    }

    if let Some(ref tags_csv) = args.tags {
        let tags: Vec<String> = tags_csv
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        // Register tags in manifest
        let _ = manifest_store.register_tags(&tags);
        task.frontmatter.tags = tags;
    }

    if let Some(ref due_str) = args.due {
        task.frontmatter.due = Some(parse_due_date(due_str)?);
    }

    // Save
    store.save(&task)?;

    // Print for shell integration
    // Users can do: export TODO_LAST_ID=$(todo create --title "..." -- body)
    println!("{id}");

    Ok(id)
}

/// Parse a flexible due date string into UTC DateTime.
/// Accepts: "2025-03-15", "2025-03-15 17:00", "2025-03-15T17:00:00+05:00", etc.
fn parse_due_date(s: &str) -> Result<DateTime<Utc>> {
    // Try RFC 3339 first (includes timezone)
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Ok(dt.with_timezone(&Utc));
    }

    // Try common formats
    let formats = [
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%d %H:%M",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%dT%H:%M",
        "%Y-%m-%d",
    ];

    for fmt in &formats {
        if let Ok(naive) = NaiveDateTime::parse_from_str(s, fmt) {
            return Ok(DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc));
        }
    }

    // Try date-only
    if let Ok(naive_date) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        let naive_dt = naive_date.and_hms_opt(0, 0, 0).unwrap();
        return Ok(DateTime::<Utc>::from_naive_utc_and_offset(naive_dt, Utc));
    }

    Err(TodoError::InvalidDateTime(format!(
        "Cannot parse '{s}'. Use formats like: 2025-03-15, 2025-03-15 17:00, or RFC 3339"
    )))
}
