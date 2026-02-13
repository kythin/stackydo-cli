use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestSettings {
    /// Default sort field for TUI list view
    pub default_sort: String,

    /// Default status filter (None = show all)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_filter_status: Option<String>,

    /// Whether to auto-capture git context on task creation
    #[serde(default = "default_true")]
    pub auto_capture_git: bool,

    /// Max tasks to show in headless `list` output
    #[serde(default = "default_quick_list_limit")]
    pub quick_list_limit: usize,
}

fn default_true() -> bool {
    true
}
fn default_quick_list_limit() -> usize {
    50
}

impl Default for ManifestSettings {
    fn default() -> Self {
        Self {
            default_sort: "created".into(),
            default_filter_status: None,
            auto_capture_git: true,
            quick_list_limit: 50,
        }
    }
}

/// Root manifest stored at ~/.stackstodo/manifest.json.
/// Tracks tags, stacks, settings, and feature flags.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub version: String,
    pub tags: HashSet<String>,
    pub stacks: HashSet<String>,

    /// Feature flags — some managed via TUI, others text-edit only
    #[serde(default)]
    pub features: HashMap<String, bool>,

    #[serde(default)]
    pub settings: ManifestSettings,
}

impl Default for Manifest {
    fn default() -> Self {
        Self {
            version: "1.0".into(),
            tags: HashSet::new(),
            stacks: HashSet::new(),
            features: HashMap::new(),
            settings: ManifestSettings::default(),
        }
    }
}
