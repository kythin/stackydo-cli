use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Parsed fields from a `.stackydo-context` file.
/// Unknown YAML fields are silently ignored for backward compatibility.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StackydoConfig {
    pub dir: Option<String>,
    pub project: Option<String>,
    pub repo: Option<String>,
    pub stack: Option<String>,
    pub description: Option<String>,
}

/// A discovered and parsed `.stackydo-context` file with its location and raw content.
#[derive(Debug, Clone)]
pub struct ResolvedConfig {
    /// Parsed config fields (all `None` if the file wasn't valid YAML).
    pub config: StackydoConfig,
    /// Absolute path to the `.stackydo-context` file.
    pub file_path: PathBuf,
    /// Raw text content of the file (always preserved).
    pub raw_content: String,
}
