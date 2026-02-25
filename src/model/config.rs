use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Project context metadata nested inside `stackydo.json`.
/// These fields are captured when creating tasks from this directory.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ContextConfig {
    /// Human-readable project name (informational).
    pub project: Option<String>,
    /// Repository URL or identifier (informational).
    pub repo: Option<String>,
    /// Free-text notes captured as context when creating tasks.
    pub description: Option<String>,
}

/// Parsed fields from a `stackydo.json` config file.
/// Unknown JSON fields are silently ignored for forward compatibility.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct StackydoConfig {
    /// Path to the task store, relative to this file. Overrides `STACKYDO_DIR`.
    pub dir: Option<String>,
    /// Glob pattern (supports `*`) filtering which stacks are visible in all
    /// list/stats/stacks/search output and the TUI. When absent, all stacks
    /// are shown. Does not restrict write access; use separate workspaces for
    /// isolation. Example: `"project-myapp_*"`
    pub stack_filter: Option<String>,
    /// Project context metadata captured when creating tasks.
    pub context: Option<ContextConfig>,
}

/// A discovered and parsed `stackydo.json` file with its location and raw content.
#[derive(Debug, Clone)]
pub struct ResolvedConfig {
    /// Parsed config fields.
    pub config: StackydoConfig,
    /// Absolute path to the `stackydo.json` file.
    pub file_path: PathBuf,
    /// Raw text content of the file (always preserved).
    pub raw_content: String,
}
