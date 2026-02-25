use crate::model::config::{ResolvedConfig, StackydoConfig};
use crate::model::context::TodoContextFile;
use crate::storage::paths::TodoPaths;
use std::fs;
use std::path::Path;

/// Discover and read the nearest `stackydo.json` file.
///
/// Search order:
/// 1. Walk up from `start_dir` looking for `stackydo.json`
/// 2. Fall back to `~/.stackydo.json`
/// 3. Return None if neither exists
pub fn discover(start_dir: &Path) -> Option<TodoContextFile> {
    // Try ancestry walk first
    if let Some(path) = TodoPaths::find_config(start_dir) {
        if let Ok(content) = fs::read_to_string(&path) {
            return Some(TodoContextFile {
                path: path.to_string_lossy().into(),
                content,
            });
        }
    }

    // Fallback to home directory
    let home_cfg = TodoPaths::home_config();
    if home_cfg.is_file() {
        if let Ok(content) = fs::read_to_string(&home_cfg) {
            return Some(TodoContextFile {
                path: home_cfg.to_string_lossy().into(),
                content,
            });
        }
    }

    None
}

/// Discover the nearest `stackydo.json` file and parse it.
///
/// Uses the same search order as `discover()`. If the file exists but isn't
/// valid JSON, returns a `ResolvedConfig` with all fields `None` but the raw
/// content preserved.
pub fn discover_config(start_dir: &Path) -> Option<ResolvedConfig> {
    let ctx_file = discover(start_dir)?;
    let file_path = Path::new(&ctx_file.path).to_path_buf();

    let config = serde_json::from_str::<StackydoConfig>(&ctx_file.content)
        .unwrap_or_default();

    Some(ResolvedConfig {
        config,
        file_path,
        raw_content: ctx_file.content,
    })
}
