use crate::model::context::TodoContextFile;
use crate::storage::paths::TodoPaths;
use std::fs;
use std::path::Path;

/// Discover and read the nearest `.todo-context` file.
///
/// Search order:
/// 1. Walk up from `start_dir` looking for `.todo-context`
/// 2. Fall back to `~/.todo-context`
/// 3. Return None if neither exists
pub fn discover(start_dir: &Path) -> Option<TodoContextFile> {
    // Try ancestry walk first
    if let Some(path) = TodoPaths::find_todo_context(start_dir) {
        if let Ok(content) = fs::read_to_string(&path) {
            return Some(TodoContextFile {
                path: path.to_string_lossy().into(),
                content,
            });
        }
    }

    // Fallback to home directory
    let home_ctx = TodoPaths::home_todo_context();
    if home_ctx.is_file() {
        if let Ok(content) = fs::read_to_string(&home_ctx) {
            return Some(TodoContextFile {
                path: home_ctx.to_string_lossy().into(),
                content,
            });
        }
    }

    None
}
