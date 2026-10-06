//! User application local state persistence (active environments, open tabs, recent collections, panel splits).
//! Stored in OS user config directory (`dirs::config_dir()/kestrel/state.json`).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppLocalState {
    pub recent_collections: Vec<String>,
    pub active_environments: HashMap<String, String>,
    pub open_tabs: HashMap<String, Vec<String>>,
    pub sidebar_width: Option<f32>,
    pub panel_split_ratio: Option<f32>,
}

impl AppLocalState {
    /// Loads local state from OS user config directory.
    pub fn load() -> Self {
        let Some(config_dir) = dirs::config_dir() else {
            return Self::default();
        };
        let state_file = config_dir.join("kestrel").join("state.json");
        if !state_file.exists() {
            return Self::default();
        }

        let content = match fs::read_to_string(&state_file) {
            Ok(c) => c,
            Err(_) => return Self::default(),
        };

        let mut state: AppLocalState = serde_json::from_str(&content).unwrap_or_default();

        // Prune recent collections paths that no longer exist on disk
        state
            .recent_collections
            .retain(|path_str| Path::new(path_str).exists());

        state
    }

    /// Saves local state to OS user config directory.
    pub fn save(&self) -> Result<(), std::io::Error> {
        let Some(config_dir) = dirs::config_dir() else {
            return Ok(());
        };
        let kestrel_dir = config_dir.join("kestrel");
        fs::create_dir_all(&kestrel_dir)?;

        let state_file = kestrel_dir.join("state.json");
        let content = serde_json::to_string_pretty(self)?;
        fs::write(state_file, content)?;

        Ok(())
    }

    /// Adds a collection path to recent collections list (max 10 items, deduplicated).
    pub fn add_recent(&mut self, path: impl AsRef<Path>) {
        let path_str = path.as_ref().to_string_lossy().to_string();
        self.recent_collections.retain(|p| p != &path_str);
        self.recent_collections.insert(0, path_str);
        if self.recent_collections.len() > 10 {
            self.recent_collections.truncate(10);
        }
    }
}
