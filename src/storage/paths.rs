use std::path::{Path, PathBuf};

/// Centralized path resolution for the todo system.
pub struct TodoPaths;

impl TodoPaths {
    /// Root storage directory.
    /// Uses `STACKSTODO_DIR` env var if set, otherwise defaults to `~/.stackstodo/`.
    pub fn root() -> PathBuf {
        if let Ok(dir) = std::env::var("STACKSTODO_DIR") {
            PathBuf::from(dir)
        } else {
            dirs::home_dir()
                .expect("Cannot determine home directory")
                .join(".stackstodo")
        }
    }

    /// Path to the manifest file
    pub fn manifest() -> PathBuf {
        Self::root().join("manifest.json")
    }

    /// Path to a task markdown file by ULID
    pub fn task_file(task_id: &str) -> PathBuf {
        Self::root().join(format!("{task_id}.md"))
    }

    /// Ensure the root storage directory exists
    pub fn ensure_root() -> std::io::Result<()> {
        std::fs::create_dir_all(Self::root())
    }

    /// Walk up from `start_dir` looking for a `.stackstodo-context` file.
    /// Returns the first one found, or None.
    pub fn find_todo_context(start_dir: &Path) -> Option<PathBuf> {
        let mut current = start_dir.to_path_buf();
        loop {
            let candidate = current.join(".stackstodo-context");
            if candidate.is_file() {
                return Some(candidate);
            }
            if !current.pop() {
                break;
            }
        }
        None
    }

    /// Fallback ~/.stackstodo-context path
    pub fn home_todo_context() -> PathBuf {
        dirs::home_dir()
            .expect("Cannot determine home directory")
            .join(".stackstodo-context")
    }
}
