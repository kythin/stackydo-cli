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
            if path.extension().map_or(false, |ext| ext == "md") {
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
    use crate::model::task::{ContextInfo, TaskStatus};

    #[test]
    fn roundtrip_serialize_parse() {
        let task = Task::new("TEST123".into(), "Test task".into(), "/tmp".into());
        let md = serialize_task(&task).unwrap();
        let parsed = parse_task(&md).unwrap();
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
        let task = parse_task(md).unwrap();
        assert_eq!(task.frontmatter.id, "ABC");
        assert!(task.body.contains("multiple paragraphs"));
    }
}
