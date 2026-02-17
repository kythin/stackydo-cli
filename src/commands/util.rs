use crate::error::{Result, TodoError};
use crate::model::task::{Task, TaskFrontmatter, TaskStatus};
use chrono::{DateTime, NaiveDateTime, Utc};
use serde::Serialize;

/// Parse a flexible due date string into UTC DateTime.
/// Accepts: "2025-03-15", "2025-03-15 17:00", "2025-03-15T17:00:00+05:00", etc.
pub fn parse_due_date(s: &str) -> Result<DateTime<Utc>> {
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
        let naive_dt = naive_date.and_hms_opt(0, 0, 0).ok_or_else(|| {
            TodoError::InvalidDateTime(format!("Cannot convert date '{s}' to datetime"))
        })?;
        return Ok(DateTime::<Utc>::from_naive_utc_and_offset(naive_dt, Utc));
    }

    Err(TodoError::InvalidDateTime(format!(
        "Cannot parse '{s}'. Use formats like: 2025-03-15, 2025-03-15 17:00, or RFC 3339"
    )))
}

/// Format a task as a single-line row for list/search output.
pub fn format_task_row(fm: &TaskFrontmatter) -> String {
    let pri = fm
        .priority
        .as_ref()
        .map(|p| format!("[{p}]"))
        .unwrap_or_default();
    let due = fm
        .due
        .map(|d| format!(" due:{}", d.format("%Y-%m-%d")))
        .unwrap_or_default();
    let tags = if fm.tags.is_empty() {
        String::new()
    } else {
        format!(" #{}", fm.tags.join(" #"))
    };
    let stack = fm
        .stack
        .as_ref()
        .map(|s| format!(" @{s}"))
        .unwrap_or_default();

    format!(
        "{status:<12} {id:.10}  {pri:<10} {title}{due}{tags}{stack}",
        status = fm.status,
        id = fm.id,
        pri = pri,
        title = fm.title,
    )
}

/// Check if a task matches the given filter criteria.
pub fn matches_filters(
    task: &Task,
    status: Option<&TaskStatus>,
    tag: Option<&str>,
    stack: Option<&str>,
) -> bool {
    if let Some(s) = status {
        if &task.frontmatter.status != s {
            return false;
        }
    }
    if let Some(t) = tag {
        let t_lower = t.to_lowercase();
        if !task
            .frontmatter
            .tags
            .iter()
            .any(|tt| tt.to_lowercase() == t_lower)
        {
            return false;
        }
    }
    if let Some(st) = stack {
        let st_lower = st.to_lowercase();
        match &task.frontmatter.stack {
            Some(s) if s.to_lowercase() == st_lower => {}
            _ => return false,
        }
    }
    true
}

/// Print a single value as JSON to stdout.
pub fn print_json<T: Serialize>(value: &T) -> Result<()> {
    let json = serde_json::to_string_pretty(value)?;
    println!("{json}");
    Ok(())
}

/// Print a slice of values as a JSON array to stdout.
pub fn print_json_array<T: Serialize>(values: &[T]) -> Result<()> {
    let json = serde_json::to_string_pretty(values)?;
    println!("{json}");
    Ok(())
}
