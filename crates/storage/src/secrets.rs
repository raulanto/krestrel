//! Local secret variables management and `.gitignore` auto-configuration.

use crate::StorageError;
use kestrel_core::Collection;
use std::fs;
use std::path::Path;

/// Saves secret variables to `.kestrel/secrets.json` and updates `.gitignore`.
pub fn save_local_secrets(
    collection_root: &Path,
    collection: &Collection,
) -> Result<(), StorageError> {
    let kestrel_dir = collection_root.join(".kestrel");
    fs::create_dir_all(&kestrel_dir)?;

    let mut secrets_map = serde_json::Map::new();

    for env in &collection.environments {
        let mut env_map = serde_json::Map::new();
        for (name, var) in &env.variables {
            if var.secret {
                env_map.insert(name.clone(), serde_json::Value::String(var.value.clone()));
            }
        }
        if !env_map.is_empty() {
            secrets_map.insert(env.name.clone(), serde_json::Value::Object(env_map));
        }
    }

    let secrets_file = kestrel_dir.join("secrets.json");
    let json_content = serde_json::to_string_pretty(&secrets_map)
        .map_err(|e| StorageError::Io(std::io::Error::other(e)))?;

    fs::write(&secrets_file, json_content)?;

    // Ensure `.kestrel` is in `.gitignore`
    ensure_gitignore_has_kestrel(collection_root)?;

    Ok(())
}

/// Ensures `.gitignore` in collection directory contains `.kestrel/` and `.env.local`.
pub fn ensure_gitignore_has_kestrel(collection_root: &Path) -> Result<(), StorageError> {
    let gitignore_path = collection_root.join(".gitignore");
    let content = if gitignore_path.exists() {
        fs::read_to_string(&gitignore_path).unwrap_or_default()
    } else {
        String::new()
    };

    let mut lines: Vec<&str> = content.lines().collect();
    let mut modified = false;

    if !lines
        .iter()
        .any(|l| l.trim() == ".kestrel" || l.trim() == ".kestrel/")
    {
        lines.push(".kestrel/");
        modified = true;
    }
    if !lines.iter().any(|l| l.trim() == ".env.local") {
        lines.push(".env.local");
        modified = true;
    }

    if modified || !gitignore_path.exists() {
        let mut new_content = lines.join("\n");
        if !new_content.ends_with('\n') {
            new_content.push('\n');
        }
        fs::write(gitignore_path, new_content)?;
    }

    Ok(())
}
