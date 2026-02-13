use crate::error::Result;
use crate::model::manifest::Manifest;
use crate::storage::paths::TodoPaths;
use std::fs;
use std::path::PathBuf;

/// JSON-backed manifest store at ~/.todos/manifest.json
pub struct ManifestStore {
    path: PathBuf,
}

impl ManifestStore {
    pub fn new() -> Self {
        Self {
            path: TodoPaths::manifest(),
        }
    }

    pub fn with_path(path: PathBuf) -> Self {
        Self { path }
    }

    /// Load the manifest from disk, or return defaults if it doesn't exist yet.
    pub fn load(&self) -> Result<Manifest> {
        if !self.path.exists() {
            return Ok(Manifest::default());
        }
        let content = fs::read_to_string(&self.path)?;
        let manifest: Manifest = serde_json::from_str(&content)?;
        Ok(manifest)
    }

    /// Save the manifest to disk (pretty-printed JSON).
    pub fn save(&self, manifest: &Manifest) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(manifest)?;
        fs::write(&self.path, json)?;
        Ok(())
    }

    /// Register a tag in the manifest (idempotent).
    pub fn register_tags(&self, tags: &[String]) -> Result<()> {
        let mut manifest = self.load()?;
        for tag in tags {
            manifest.tags.insert(tag.clone());
        }
        self.save(&manifest)
    }

    /// Register a category in the manifest (idempotent).
    pub fn register_categories(&self, categories: &[String]) -> Result<()> {
        let mut manifest = self.load()?;
        for cat in categories {
            manifest.categories.insert(cat.clone());
        }
        self.save(&manifest)
    }
}
