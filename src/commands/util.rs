use crate::error::{Result, TodoError};
use crate::model::task::{Priority, Stage, Task, TaskFrontmatter, WorkflowConfig};
use crate::storage::manifest_store::ManifestStore;
use chrono::{DateTime, Datelike, NaiveDateTime, Utc};
use serde::Serialize;

/// Case-insensitive glob match. Supports `*` as a wildcard (zero or more chars).
pub fn glob_matches(pattern: &str, text: &str) -> bool {
    glob_match_impl(&pattern.to_lowercase(), &text.to_lowercase())
}

fn glob_match_impl(p: &str, t: &str) -> bool {
    if p.is_empty() {
        return t.is_empty();
    }
    if p.starts_with('*') {
        // Consume consecutive stars (they collapse to one).
        let rest_p = p.trim_start_matches('*');
        if rest_p.is_empty() {
            return true; // trailing * matches anything
        }
        // Try matching rest_p at every position in t.
        for i in 0..=t.len() {
            if t.is_char_boundary(i) && glob_match_impl(rest_p, &t[i..]) {
                return true;
            }
        }
        false
    } else {
        // Consume the literal prefix up to the next *.
        let star_pos = p.find('*').unwrap_or(p.len());
        let literal = &p[..star_pos];
        t.starts_with(literal) && glob_match_impl(&p[star_pos..], &t[literal.len()..])
    }
}

/// Returns true if `task_stack` matches the glob `pattern`.
/// A task with no stack never matches any pattern.
pub fn stack_filter_matches(pattern: &str, task_stack: Option<&str>) -> bool {
    task_stack.is_some_and(|s| glob_matches(pattern, s))
}

/// Return the `stack_filter` from the active `stackydo.json`, if any.
pub fn active_stack_filter() -> Option<String> {
    crate::storage::paths::TodoPaths::resolved_config().and_then(|c| c.config.stack_filter.clone())
}

/// Load the workflow config from the manifest, falling back to defaults.
pub fn active_workflow() -> WorkflowConfig {
    ManifestStore::new()
        .load()
        .map(|m| m.workflow)
        .unwrap_or_default()
}

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

/// Return the short display ID for a task: short_id if available, else first 10 chars of ULID.
pub fn display_id(fm: &TaskFrontmatter) -> &str {
    fm.short_id
        .as_deref()
        .unwrap_or(&fm.id[..fm.id.len().min(10)])
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

    let comments = if fm.comments.is_empty() {
        String::new()
    } else {
        format!(" [{}c]", fm.comments.len())
    };

    let did = display_id(fm);

    format!(
        "{status:<12} {did:<10}  {pri:<10} {title}{due}{tags}{stack}{comments}",
        status = fm.status,
        pri = pri,
        title = fm.title,
    )
}

/// Check if a task matches the given filter criteria.
pub fn matches_filters(
    task: &Task,
    status: Option<&str>,
    tag: Option<&str>,
    stack: Option<&str>,
) -> bool {
    if let Some(s) = status {
        if task.frontmatter.status != s {
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

/// Parameters for filtering tasks (shared between CLI and MCP).
pub struct FilterParams<'a> {
    pub status: Option<&'a str>,
    pub stage: Option<&'a str>,
    pub tag: Option<&'a str>,
    pub priority: Option<&'a str>,
    pub stack: Option<&'a str>,
    pub overdue: bool,
    pub due_before: Option<&'a str>,
    pub due_after: Option<&'a str>,
    pub due_this_week: bool,
}

/// Apply all filters in-place. Returns Err if a filter value is invalid.
pub fn apply_filters(tasks: &mut Vec<Task>, f: &FilterParams) -> Result<()> {
    let workflow = active_workflow();

    // Status
    if let Some(status_str) = f.status {
        let canonical = workflow
            .validate_status(status_str)
            .map_err(TodoError::Other)?;
        tasks.retain(|t| t.frontmatter.status == canonical);
    }

    // Stage
    if let Some(stage_str) = f.stage {
        let stage = stage_str.parse::<Stage>().map_err(TodoError::Other)?;
        tasks.retain(|t| workflow.stage_for(&t.frontmatter.status) == stage);
    }

    // Tag
    if let Some(tag) = f.tag {
        let tag_lower = tag.to_lowercase();
        tasks.retain(|t| {
            t.frontmatter
                .tags
                .iter()
                .any(|tt| tt.to_lowercase() == tag_lower)
        });
    }

    // Priority
    if let Some(pri_str) = f.priority {
        let pri = pri_str.parse::<Priority>().map_err(TodoError::Other)?;
        tasks.retain(|t| t.frontmatter.priority.as_ref() == Some(&pri));
    }

    // Stack
    if let Some(stack) = f.stack {
        let stack_lower = stack.to_lowercase();
        tasks.retain(|t| {
            t.frontmatter
                .stack
                .as_ref()
                .map(|s| s.to_lowercase() == stack_lower)
                .unwrap_or(false)
        });
    }

    // Overdue
    if f.overdue {
        let now = Utc::now();
        tasks.retain(|t| {
            if let Some(due) = t.frontmatter.due {
                due < now && !workflow.is_terminal(&t.frontmatter.status)
            } else {
                false
            }
        });
    }

    // Due before
    if let Some(date_str) = f.due_before {
        let cutoff = parse_due_date(date_str)?;
        tasks.retain(|t| t.frontmatter.due.map(|d| d < cutoff).unwrap_or(false));
    }

    // Due after
    if let Some(date_str) = f.due_after {
        let cutoff = parse_due_date(date_str)?;
        tasks.retain(|t| t.frontmatter.due.map(|d| d > cutoff).unwrap_or(false));
    }

    // Due this week
    if f.due_this_week {
        let today = Utc::now().date_naive();
        let weekday = today.weekday().num_days_from_monday();
        let monday = today - chrono::Duration::days(weekday as i64);
        let sunday = monday + chrono::Duration::days(6);
        tasks.retain(|t| {
            if let Some(due) = t.frontmatter.due {
                let due_date = due.date_naive();
                due_date >= monday && due_date <= sunday
            } else {
                false
            }
        });
    }

    Ok(())
}

/// Sort tasks in-place by the given field. Optionally reverse.
/// Returns an error if the sort field is not recognized.
pub fn apply_sort(tasks: &mut [Task], sort: &str, reverse: bool) -> Result<()> {
    match sort {
        "due" => tasks.sort_by(|a, b| a.frontmatter.due.cmp(&b.frontmatter.due)),
        "modified" => tasks.sort_by(|a, b| b.frontmatter.modified.cmp(&a.frontmatter.modified)),
        "priority" => tasks.sort_by(|a, b| a.frontmatter.priority.cmp(&b.frontmatter.priority)),
        "created" => tasks.sort_by(|a, b| b.frontmatter.created.cmp(&a.frontmatter.created)),
        other => {
            return Err(TodoError::Other(format!(
                "Invalid sort: {other}. Use: created, due, modified, priority"
            )))
        }
    }
    if reverse {
        tasks.reverse();
    }
    Ok(())
}

/// Pagination result info.
pub struct PaginationInfo {
    /// Total items before pagination.
    pub total: usize,
    /// 0-indexed start position in original list.
    pub start: usize,
    /// Number of items in the page.
    pub page_len: usize,
}

/// Compute effective limit from raw limit value (None → 50, 0 → no limit).
pub fn effective_limit(limit: Option<usize>) -> usize {
    match limit {
        Some(0) => usize::MAX,
        Some(n) => n,
        None => 50,
    }
}

/// Apply offset and limit to a task list in-place. Returns pagination info.
pub fn apply_pagination(tasks: &mut Vec<Task>, offset: usize, limit: usize) -> PaginationInfo {
    let total = tasks.len();
    let start = offset.min(total);
    if start > 0 {
        tasks.drain(..start);
    }
    let effective_limit = if limit == usize::MAX {
        tasks.len()
    } else {
        limit
    };
    tasks.truncate(effective_limit);
    PaginationInfo {
        total,
        start,
        page_len: tasks.len(),
    }
}

/// Print a human-readable pagination footer.
pub fn print_pagination_footer(info: &PaginationInfo, label: &str) {
    if info.page_len == info.total {
        // All results shown
        println!(
            "\n({} {}{} total)",
            info.total,
            label,
            if info.total == 1 { "" } else { "s" }
        );
    } else {
        // Paginated
        println!(
            "\n(showing {} {}{} of {} total)",
            info.page_len,
            label,
            if info.page_len == 1 { "" } else { "s" },
            info.total,
        );
    }
}

#[cfg(test)]
mod glob_tests {
    use super::glob_matches;

    #[test]
    fn exact_match() {
        assert!(glob_matches("work", "work"));
        assert!(!glob_matches("work", "Work2"));
    }

    #[test]
    fn case_insensitive() {
        assert!(glob_matches("WORK", "work"));
        assert!(glob_matches("work", "WORK"));
    }

    #[test]
    fn trailing_star() {
        assert!(glob_matches("project-myapp_*", "project-myapp_frontend"));
        assert!(glob_matches("project-myapp_*", "project-myapp_"));
        assert!(!glob_matches("project-myapp_*", "project-other_frontend"));
    }

    #[test]
    fn leading_star() {
        assert!(glob_matches("*-dev", "project-dev"));
        assert!(glob_matches("*-dev", "sprint-12-dev"));
        assert!(!glob_matches("*-dev", "project-staging"));
    }

    #[test]
    fn surrounding_stars() {
        assert!(glob_matches("*frontend*", "project-frontend-v2"));
        assert!(glob_matches("*frontend*", "frontend"));
        assert!(!glob_matches("*frontend*", "backend"));
    }

    #[test]
    fn bare_star_matches_all() {
        assert!(glob_matches("*", "anything"));
        assert!(glob_matches("*", ""));
    }

    #[test]
    fn no_stack_never_matches() {
        assert!(!super::stack_filter_matches("work", None));
        assert!(!super::stack_filter_matches("*", None));
    }
}
