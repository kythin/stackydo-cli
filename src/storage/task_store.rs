use crate::error::{Result, TodoError};
use crate::model::task::{Task, TaskFrontmatter};
use crate::storage::paths::TodoPaths;
use std::fs;
use std::path::PathBuf;

const FRONTMATTER_DELIMITER: &str = "---";

/// Filesystem-backed task store. Each task is a markdown file with YAML frontmatter.
pub struct TaskStore {
    root: PathBuf,
}

impl Default for TaskStore {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskStore {
    pub fn new() -> Self {
        Self {
            root: TodoPaths::root(),
        }
    }

    pub fn with_root(root: PathBuf) -> Self {
        Self { root }
    }

    /// Save a task to disk as `<id>.md`
    pub fn save(&self, task: &Task) -> Result<()> {
        fs::create_dir_all(&self.root)?;
        let path = self.root.join(format!("{}.md", task.frontmatter.id));
        let content = serialize_task(task)?;
        fs::write(&path, content)?;
        Ok(())
    }

    /// Load a single task by ID
    pub fn load(&self, task_id: &str) -> Result<Task> {
        let path = self.root.join(format!("{task_id}.md"));
        if !path.exists() {
            return Err(TodoError::TaskNotFound(task_id.to_string()));
        }
        let content = fs::read_to_string(&path)?;
        parse_task(&content)
    }

    /// Load all tasks from the store
    pub fn load_all(&self) -> Result<Vec<Task>> {
        let mut tasks = Vec::new();
        if !self.root.exists() {
            return Ok(tasks);
        }
        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "md") {
                match fs::read_to_string(&path) {
                    Ok(content) => match parse_task(&content) {
                        Ok(task) => tasks.push(task),
                        Err(e) => {
                            eprintln!("Warning: skipping {}: {e}", path.display());
                        }
                    },
                    Err(e) => {
                        eprintln!("Warning: cannot read {}: {e}", path.display());
                    }
                }
            }
        }
        Ok(tasks)
    }

    /// Delete a task file by ID
    pub fn delete(&self, task_id: &str) -> Result<()> {
        let path = self.root.join(format!("{task_id}.md"));
        if !path.exists() {
            return Err(TodoError::TaskNotFound(task_id.to_string()));
        }
        fs::remove_file(&path)?;
        Ok(())
    }

    /// Search tasks by matching query against title and body (case-insensitive)
    pub fn search(&self, query: &str) -> Result<Vec<Task>> {
        let query_lower = query.to_lowercase();
        let all = self.load_all()?;
        Ok(all
            .into_iter()
            .filter(|t| {
                t.frontmatter.title.to_lowercase().contains(&query_lower)
                    || t.body.to_lowercase().contains(&query_lower)
            })
            .collect())
    }
}

/// Parse a markdown string with YAML frontmatter into a Task.
fn parse_task(content: &str) -> Result<Task> {
    let content = content.trim();
    if !content.starts_with(FRONTMATTER_DELIMITER) {
        return Err(TodoError::FrontmatterParse(
            "File does not start with ---".into(),
        ));
    }

    // Find the closing --- delimiter
    let after_first = &content[3..];
    let end_idx = after_first
        .find("\n---")
        .ok_or_else(|| TodoError::FrontmatterParse("No closing --- found".into()))?;

    let yaml_str = &after_first[..end_idx];
    let body_start = 3 + end_idx + 4; // skip past "\n---"
    let body = if body_start < content.len() {
        content[body_start..].trim_start_matches('\n').to_string()
    } else {
        String::new()
    };

    let frontmatter: TaskFrontmatter =
        serde_yaml::from_str(yaml_str).map_err(TodoError::Yaml)?;

    Ok(Task { frontmatter, body })
}

/// Serialize a Task to markdown with YAML frontmatter.
fn serialize_task(task: &Task) -> Result<String> {
    let yaml = serde_yaml::to_string(&task.frontmatter).map_err(TodoError::Yaml)?;
    let mut out = String::new();
    out.push_str(FRONTMATTER_DELIMITER);
    out.push('\n');
    out.push_str(&yaml);
    out.push_str(FRONTMATTER_DELIMITER);
    out.push('\n');
    if !task.body.is_empty() {
        out.push('\n');
        out.push_str(&task.body);
        if !task.body.ends_with('\n') {
            out.push('\n');
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::task::{Priority, TaskStatus};

    #[test]
    fn roundtrip_serialize_parse() {
        let task = Task::new("TEST123".into(), "Test task".into(), "/tmp".into());
        let md = serialize_task(&task).expect("serialize should succeed");
        let parsed = parse_task(&md).expect("parse should succeed");
        assert_eq!(parsed.frontmatter.id, "TEST123");
        assert_eq!(parsed.frontmatter.title, "Test task");
        assert_eq!(parsed.frontmatter.status, TaskStatus::Todo);
    }

    #[test]
    fn parse_with_body() {
        let md = r#"---
id: ABC
title: Hello
status: todo
created: 2025-01-01T00:00:00Z
modified: 2025-01-01T00:00:00Z
context:
  working_dir: /tmp
---

This is the body content.

With multiple paragraphs.
"#;
        let task = parse_task(md).expect("parse should succeed");
        assert_eq!(task.frontmatter.id, "ABC");
        assert!(task.body.contains("multiple paragraphs"));
    }

    // ── Corruption / resilience tests (feature 11) ──

    #[test]
    fn parse_missing_opening_delimiter() {
        let md = "id: ABC\ntitle: Hello\n---\n";
        assert!(parse_task(md).is_err());
    }

    #[test]
    fn parse_missing_closing_delimiter() {
        let md = "---\nid: ABC\ntitle: Hello\n";
        assert!(parse_task(md).is_err());
    }

    #[test]
    fn parse_invalid_yaml() {
        let md = "---\n: : : invalid yaml [[\n---\n";
        assert!(parse_task(md).is_err());
    }

    #[test]
    fn parse_empty_file() {
        assert!(parse_task("").is_err());
        assert!(parse_task("   ").is_err());
    }

    #[test]
    fn load_all_skips_corrupt_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = TaskStore::with_root(dir.path().to_path_buf());

        // Write a valid task
        let valid = Task::new("VALID1".into(), "Valid task".into(), "/tmp".into());
        store.save(&valid).expect("save valid");

        // Write a corrupt .md file
        std::fs::write(dir.path().join("CORRUPT.md"), "not valid frontmatter")
            .expect("write corrupt");

        // Write another valid task
        let valid2 = Task::new("VALID2".into(), "Valid task 2".into(), "/tmp".into());
        store.save(&valid2).expect("save valid2");

        let tasks = store.load_all().expect("load_all should succeed");
        assert_eq!(tasks.len(), 2, "should load 2 valid tasks, skipping corrupt");
    }

    // ── Roundtrip with rich fields ──

    #[test]
    fn roundtrip_with_all_fields() {
        use chrono::Utc;

        let mut task = Task::new("RICH1".into(), "Rich task".into(), "/home/user".into());
        task.frontmatter.priority = Some(Priority::High);
        task.frontmatter.tags = vec!["backend".into(), "urgent".into()];
        task.frontmatter.stack = Some("work".into());
        task.frontmatter.due = Some(Utc::now());
        task.body = "Some body content\nwith newlines\n".to_string();

        let md = serialize_task(&task).expect("serialize");
        let parsed = parse_task(&md).expect("parse");

        assert_eq!(parsed.frontmatter.id, "RICH1");
        assert_eq!(parsed.frontmatter.title, "Rich task");
        assert_eq!(parsed.frontmatter.priority, Some(Priority::High));
        assert_eq!(parsed.frontmatter.tags, vec!["backend", "urgent"]);
        assert_eq!(parsed.frontmatter.stack, Some("work".into()));
        assert!(parsed.frontmatter.due.is_some());
        assert!(parsed.body.contains("with newlines"));
    }

    #[test]
    fn roundtrip_empty_body() {
        let task = Task::new("EMPTY1".into(), "No body".into(), "/tmp".into());
        let md = serialize_task(&task).expect("serialize");
        let parsed = parse_task(&md).expect("parse");
        assert!(parsed.body.is_empty());
    }

    #[test]
    fn roundtrip_unicode_title() {
        let task = Task::new("UNI1".into(), "日本語タスク 🚀".into(), "/tmp".into());
        let md = serialize_task(&task).expect("serialize");
        let parsed = parse_task(&md).expect("parse");
        assert_eq!(parsed.frontmatter.title, "日本語タスク 🚀");
    }
}

#[cfg(test)]
mod proptest_tests {
    use super::*;
    use crate::model::task::{ContextInfo, Priority, TaskStatus};
    use chrono::{DateTime, TimeZone, Utc};
    use proptest::prelude::*;

    fn arb_status() -> impl Strategy<Value = TaskStatus> {
        prop_oneof![
            Just(TaskStatus::Todo),
            Just(TaskStatus::InProgress),
            Just(TaskStatus::Done),
            Just(TaskStatus::Blocked),
            Just(TaskStatus::Cancelled),
        ]
    }

    fn arb_priority() -> impl Strategy<Value = Option<Priority>> {
        prop_oneof![
            Just(None),
            Just(Some(Priority::Critical)),
            Just(Some(Priority::High)),
            Just(Some(Priority::Medium)),
            Just(Some(Priority::Low)),
        ]
    }

    fn arb_datetime() -> impl Strategy<Value = DateTime<Utc>> {
        // Dates between 2020 and 2030
        (2020i32..2030, 1u32..13, 1u32..29, 0u32..24, 0u32..60).prop_map(
            |(y, m, d, h, min)| {
                Utc.with_ymd_and_hms(y, m, d, h, min, 0)
                    .single()
                    .unwrap_or_else(|| Utc::now())
            },
        )
    }

    fn arb_tag() -> impl Strategy<Value = String> {
        "[a-z][a-z0-9_-]{0,15}".prop_map(|s| s)
    }

    proptest! {
        #[test]
        fn roundtrip_proptest(
            id in "[A-Z0-9]{10,26}",
            title in ".{1,100}",
            status in arb_status(),
            priority in arb_priority(),
            tags in proptest::collection::vec(arb_tag(), 0..5),
            stack in proptest::option::of("[a-z][a-z0-9-]{0,15}"),
            created in arb_datetime(),
            modified in arb_datetime(),
            body in ".*",
        ) {
            let task = Task {
                frontmatter: TaskFrontmatter {
                    id: id.clone(),
                    title: title.clone(),
                    status: status.clone(),
                    priority: priority.clone(),
                    tags: tags.clone(),
                    stack: stack.clone(),
                    due: None,
                    created,
                    modified,
                    parent_id: None,
                    subtask_ids: Vec::new(),
                    dependencies: Vec::new(),
                    context: ContextInfo {
                        working_dir: "/tmp".into(),
                        ..Default::default()
                    },
                },
                body: body.clone(),
            };

            let md = serialize_task(&task).expect("serialize should succeed");
            let parsed = parse_task(&md).expect("parse should succeed");

            prop_assert_eq!(&parsed.frontmatter.id, &id);
            prop_assert_eq!(&parsed.frontmatter.title, &title);
            prop_assert_eq!(&parsed.frontmatter.status, &status);
            prop_assert_eq!(&parsed.frontmatter.priority, &priority);
            prop_assert_eq!(&parsed.frontmatter.tags, &tags);
            prop_assert_eq!(&parsed.frontmatter.stack, &stack);
        }
    }
}
