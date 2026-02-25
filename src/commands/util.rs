use crate::error::{Result, TodoError};
use crate::model::task::{Task, TaskFrontmatter, TaskStatus};
use chrono::{DateTime, NaiveDateTime, Utc};
use serde::Serialize;

/// Case-insensitive glob match. Supports `*` as a wildcard (zero or more chars).
pub fn glob_matches(pattern: &str, text: &str) -> bool {
    glob_match_impl(
        &pattern.to_lowercase(),
        &text.to_lowercase(),
    )
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
    task_stack.map_or(false, |s| glob_matches(pattern, s))
}

/// Return the `stack_filter` from the active `stackydo.json`, if any.
pub fn active_stack_filter() -> Option<String> {
    crate::storage::paths::TodoPaths::resolved_config()
        .and_then(|c| c.config.stack_filter.clone())
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
