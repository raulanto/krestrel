//! Multi-file directory collection persistence, slug sanitization, atomic writes, and security checks.

use crate::StorageError;
use crate::converter::*;
use crate::schema::*;
use indexmap::IndexMap;
use kestrel_core::{Collection, CollectionItem, Folder, ItemErrorNode};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const MAX_FILE_SIZE_BYTES: u64 = 10 * 1024 * 1024; // 10 MB limit
const MAX_NESTING_DEPTH: usize = 20;

// ...

/// Sanitizes a string for use as a filename across Windows, macOS, and Linux.
pub fn sanitize_filename(name: &str) -> String {
    let mut safe = String::new();
    for c in name.chars() {
        match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => safe.push('-'),
            c if c.is_control() => safe.push('-'),
            c => safe.push(c),
        }
    }
    let trimmed = safe.trim_matches(|c| c == '.' || c == ' ' || c == '-');
    let mut clean = if trimmed.is_empty() {
        "item".to_string()
    } else {
        trimmed.to_lowercase().replace(' ', "-")
    };

    // Windows reserved filenames check
    let stem = clean.split('.').next().unwrap_or("").to_uppercase();
    let reserved = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    if reserved.contains(&stem.as_str()) {
        clean.push('_');
    }

    if clean.len() > 64 {
        clean.truncate(64);
    }
    clean
}

/// Atomically writes content to a target file (`.tmp` write + `fsync` + `rename`).
pub fn atomic_write_file(path: &Path, content: &str) -> Result<(), StorageError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp_path = path.with_extension("tmp");
    {
        let mut file = File::create(&tmp_path)?;
        file.write_all(content.as_bytes())?;
        file.sync_all()?;
    }
    fs::rename(&tmp_path, path)?;
    Ok(())
}

/// Checks that a target path resolves strictly inside the root directory and does not follow symlinks outside.
pub fn validate_path_inside_root(root: &Path, target: &Path) -> Result<PathBuf, StorageError> {
    let root_canonical = root.canonicalize().map_err(StorageError::Io)?;
    let target_canonical = target.canonicalize().map_err(StorageError::Io)?;
    if !target_canonical.starts_with(&root_canonical) {
        return Err(StorageError::Io(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            format!(
                "Seguridad: La ruta {:?} está fuera de la raíz de la colección {:?}",
                target, root
            ),
        )));
    }
    Ok(target_canonical)
}

/// Cleans content from UTF-8 BOM byte marker if present.
fn strip_bom(content: &str) -> &str {
    content.strip_prefix('\u{feff}').unwrap_or(content)
}

/// Scans and loads a multi-file directory collection or bundled single file collection.
pub fn load_collection_from_dir(root_path: impl AsRef<Path>) -> Result<Collection, StorageError> {
    let root = root_path.as_ref();
    if !root.exists() {
        return Err(StorageError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("La carpeta de la colección no existe: {:?}", root),
        )));
    }

    // Identify root metadata document
    let meta_file = if root.join("opencollection.yaml").exists() {
        root.join("opencollection.yaml")
    } else if root.join("opencollection.yml").exists() {
        root.join("opencollection.yml")
    } else if root.join("collection.yaml").exists() {
        root.join("collection.yaml")
    } else if root.join("kestrel.yaml").exists() {
        root.join("kestrel.yaml")
    } else {
        return Err(StorageError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!(
                "No se encontró archivo de colección (opencollection.yaml) en {:?}",
                root
            ),
        )));
    };

    let meta_content_raw = fs::read_to_string(&meta_file)?;
    let meta_content = strip_bom(&meta_content_raw);
    let doc: OpenCollectionDocument = serde_yaml_ng::from_str(meta_content)?;

    let is_bundled = doc.bundled;
    let mut collection = doc_to_collection(doc);
    collection.path = Some(root.to_path_buf());
    collection.is_bundled = is_bundled;

    if is_bundled {
        return Ok(collection);
    }

    // Directory collection: scan items recursively
    let mut scanned_items = scan_directory_items(root, root, 0)?;
    sort_collection_items(&mut scanned_items);
    collection.items = scanned_items;

    // Load local secret overrides if present in `.kestrel/secrets.json`
    load_and_apply_local_secrets(root, &mut collection)?;

    Ok(collection)
}

fn scan_directory_items(
    root: &Path,
    current_dir: &Path,
    depth: usize,
) -> Result<Vec<CollectionItem>, StorageError> {
    if depth > MAX_NESTING_DEPTH {
        return Ok(Vec::new());
    }

    let entries = match fs::read_dir(current_dir) {
        Ok(e) => e,
        Err(_) => return Ok(Vec::new()),
    };

    let mut items = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // Ignore hidden, git, node_modules, tmp files, root metadata
        if file_name.starts_with('.')
            || file_name == "node_modules"
            || file_name.ends_with(".tmp")
            || file_name == "opencollection.yaml"
            || file_name == "opencollection.yml"
            || file_name == "collection.yaml"
            || file_name == "kestrel.yaml"
            || file_name == "folder.yaml"
            || file_name == "folder.yml"
        {
            continue;
        }

        let metadata = match fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };

        if metadata.file_type().is_symlink() {
            // Security check for symlinks
            if validate_path_inside_root(root, &path).is_err() {
                continue;
            }
        }

        if path.is_dir() {
            let mut sub_items = scan_directory_items(root, &path, depth + 1)?;
            sort_collection_items(&mut sub_items);

            // Check for optional folder.yaml
            let f_yaml = path.join("folder.yaml");
            let folder_meta = if f_yaml.exists() {
                f_yaml
            } else {
                path.join("folder.yml")
            };

            let (folder_name, folder_desc, folder_seq) = if folder_meta.exists() {
                if let Ok(content) = fs::read_to_string(&folder_meta) {
                    if let Ok(f_doc) = serde_yaml_ng::from_str::<ItemDoc>(strip_bom(&content)) {
                        (f_doc.info.name, f_doc.info.description, f_doc.seq)
                    } else {
                        (file_name.to_string(), None, None)
                    }
                } else {
                    (file_name.to_string(), None, None)
                }
            } else {
                (file_name.to_string(), None, None)
            };

            items.push(CollectionItem::Folder(Folder {
                id: Uuid::new_v4().to_string(),
                name: folder_name,
                description: folder_desc,
                seq: folder_seq,
                items: sub_items,
                path: Some(path.clone()),
                dirty: false,
                extra: IndexMap::new(),
            }));
        } else if path.is_file() {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext != "yaml" && ext != "yml" {
                continue;
            }

            if metadata.len() > MAX_FILE_SIZE_BYTES {
                items.push(CollectionItem::ErrorNode(ItemErrorNode {
                    id: Uuid::new_v4().to_string(),
                    name: file_name.to_string(),
                    file_path: path.clone(),
                    error_message: "El archivo excede el tamaño máximo permitido (10MB)"
                        .to_string(),
                    line: None,
                    column: None,
                }));
                continue;
            }

            let content_raw = match fs::read_to_string(&path) {
                Ok(c) => c,
                Err(err) => {
                    items.push(CollectionItem::ErrorNode(ItemErrorNode {
                        id: Uuid::new_v4().to_string(),
                        name: file_name.to_string(),
                        file_path: path.clone(),
                        error_message: format!("Error al leer archivo: {}", err),
                        line: None,
                        column: None,
                    }));
                    continue;
                }
            };

            let content = strip_bom(&content_raw);
            if content.trim().is_empty() {
                continue;
            }

            match serde_yaml_ng::from_str::<ItemDoc>(content) {
                Ok(item_doc) => {
                    let mut col_item = doc_to_item(item_doc);
                    match &mut col_item {
                        CollectionItem::Request(req) => {
                            req.path = Some(path.clone());
                        }
                        CollectionItem::Folder(f) => {
                            f.path = Some(path.clone());
                        }
                        CollectionItem::ErrorNode(_) => {}
                    }
                    items.push(col_item);
                }
                Err(yaml_err) => {
                    let (line, col) = yaml_err
                        .location()
                        .map(|l| (l.line(), l.column()))
                        .unwrap_or((0, 0));
                    items.push(CollectionItem::ErrorNode(ItemErrorNode {
                        id: Uuid::new_v4().to_string(),
                        name: file_name.to_string(),
                        file_path: path.clone(),
                        error_message: format!("Sintaxis YAML inválida: {}", yaml_err),
                        line: Some(line),
                        column: Some(col),
                    }));
                }
            }
        }
    }

    Ok(items)
}

fn doc_to_item(doc: ItemDoc) -> CollectionItem {
    let mut doc_converter = vec![doc];
    let mut res = doc_converter
        .drain(..)
        .map(doc_to_single_item)
        .collect::<Vec<_>>();
    if let Some(item) = res.pop() {
        item
    } else {
        CollectionItem::ErrorNode(ItemErrorNode {
            id: Uuid::new_v4().to_string(),
            name: "Item vacío".to_string(),
            file_path: PathBuf::new(),
            error_message: "Item sin contenido".to_string(),
            line: None,
            column: None,
        })
    }
}

fn doc_to_single_item(item: ItemDoc) -> CollectionItem {
    let dummy_doc = OpenCollectionDocument {
        opencollection: "1.0.0".to_string(),
        info: CollectionInfoDoc::default(),
        bundled: true,
        items: vec![item],
        config: None,
        extra: IndexMap::new(),
    };
    let mut col = doc_to_collection(dummy_doc);
    if !col.items.is_empty() {
        col.items.remove(0)
    } else {
        CollectionItem::ErrorNode(ItemErrorNode {
            id: Uuid::new_v4().to_string(),
            name: "Item".to_string(),
            file_path: PathBuf::new(),
            error_message: "Item doc sin elementos".to_string(),
            line: None,
            column: None,
        })
    }
}

/// Sorts collection items by `seq` field if present, with stable alphabetical fallback.
pub fn sort_collection_items(items: &mut [CollectionItem]) {
    items.sort_by(|a, b| {
        let seq_a = item_seq(a);
        let seq_b = item_seq(b);
        match (seq_a, seq_b) {
            (Some(sa), Some(sb)) => sa.cmp(&sb),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => item_name(a)
                .to_lowercase()
                .cmp(&item_name(b).to_lowercase()),
        }
    });

    for item in items.iter_mut() {
        if let CollectionItem::Folder(folder) = item {
            sort_collection_items(&mut folder.items);
        }
    }
}

fn item_seq(item: &CollectionItem) -> Option<usize> {
    match item {
        CollectionItem::Folder(f) => f.seq,
        CollectionItem::Request(r) => r.seq,
        CollectionItem::ErrorNode(_) => None,
    }
}

fn item_name(item: &CollectionItem) -> &str {
    match item {
        CollectionItem::Folder(f) => &f.name,
        CollectionItem::Request(r) => &r.name,
        CollectionItem::ErrorNode(e) => &e.name,
    }
}

/// Saves a collection to disk (either single bundled file or directory collection).
pub fn save_collection_to_dir(
    root_path: impl AsRef<Path>,
    collection: &Collection,
    dirty_only: bool,
) -> Result<(), StorageError> {
    let root = root_path.as_ref();
    fs::create_dir_all(root)?;

    if collection.is_bundled {
        let meta_file = root.join("opencollection.yaml");
        let doc = collection_to_doc(collection);
        let yaml = serde_yaml_ng::to_string(&doc)?;
        atomic_write_file(&meta_file, &yaml)?;
        return Ok(());
    }

    // Save root opencollection.yaml
    let meta_file = root.join("opencollection.yaml");
    let mut root_doc = collection_to_doc(collection);
    root_doc.items = Vec::new(); // Items live in separate files in directory collection
    root_doc.bundled = false;
    let yaml = serde_yaml_ng::to_string(&root_doc)?;
    atomic_write_file(&meta_file, &yaml)?;

    // Save items recursively
    save_items_recursive(root, &collection.items, dirty_only)?;

    Ok(())
}

fn save_items_recursive(
    current_dir: &Path,
    items: &[CollectionItem],
    dirty_only: bool,
) -> Result<(), StorageError> {
    fs::create_dir_all(current_dir)?;

    for item in items {
        match item {
            CollectionItem::Request(req) => {
                if dirty_only && !req.dirty {
                    continue;
                }
                let file_name = if let Some(p) = &req.path {
                    p.file_name()
                        .and_then(|n| n.to_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| format!("{}.yaml", sanitize_filename(&req.name)))
                } else {
                    format!("{}.yaml", sanitize_filename(&req.name))
                };
                let target_path = current_dir.join(file_name);
                let item_doc = collection_item_to_doc_item(item);
                let yaml = serde_yaml_ng::to_string(&item_doc)?;
                atomic_write_file(&target_path, &yaml)?;
            }
            CollectionItem::Folder(folder) => {
                let folder_dir = current_dir.join(sanitize_filename(&folder.name));
                fs::create_dir_all(&folder_dir)?;

                // Save folder metadata file folder.yaml if description or seq present
                if folder.description.is_some() || folder.seq.is_some() || folder.dirty {
                    let folder_meta_path = folder_dir.join("folder.yaml");
                    let item_doc = collection_item_to_doc_item(item);
                    let yaml = serde_yaml_ng::to_string(&item_doc)?;
                    atomic_write_file(&folder_meta_path, &yaml)?;
                }

                save_items_recursive(&folder_dir, &folder.items, dirty_only)?;
            }
            CollectionItem::ErrorNode(_) => {}
        }
    }

    Ok(())
}

/// Local secret variables file `.kestrel/secrets.json`
fn load_and_apply_local_secrets(
    root: &Path,
    collection: &mut Collection,
) -> Result<(), StorageError> {
    let secret_file = root.join(".kestrel").join("secrets.json");
    if !secret_file.exists() {
        return Ok(());
    }

    let content = match fs::read_to_string(&secret_file) {
        Ok(c) => c,
        Err(_) => return Ok(()),
    };

    let secrets: serde_json::Value = match serde_json::from_str(&content) {
        Ok(val) => val,
        Err(_) => return Ok(()),
    };

    if let Some(envs_obj) = secrets.as_object() {
        for env in &mut collection.environments {
            if let Some(var_map) = envs_obj.get(&env.name).and_then(|v| v.as_object()) {
                for (var_name, var_val) in var_map {
                    if let (Some(secret_str), Some(env_var)) =
                        (var_val.as_str(), env.variables.get_mut(var_name))
                    {
                        env_var.value = secret_str.to_string();
                        env_var.secret = true;
                    }
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("Get Users / v1"), "get-users---v1");
        assert_eq!(sanitize_filename("  CON  "), "con_");
        assert_eq!(sanitize_filename("Hello <World>?"), "hello--world");

        assert_eq!(sanitize_filename("Normal Name"), "normal-name");
    }

    #[test]
    fn test_load_directory_collection_fixture() {
        let fixture_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/directory_collection"
        );
        let collection = load_collection_from_dir(fixture_path).expect("Load directory collection");
        assert_eq!(collection.name, "Directory API Spec");
        assert!(!collection.is_bundled);
        assert_eq!(collection.items.len(), 1); // Users folder

        if let CollectionItem::Folder(folder) = &collection.items[0] {
            assert_eq!(folder.name, "Users");
            assert_eq!(folder.items.len(), 1); // Get Users List request
            if let CollectionItem::Request(req) = &folder.items[0] {
                assert_eq!(req.name, "Get Users List");
                assert_eq!(req.url, "{{baseUrl}}/users");
            } else {
                panic!("Expected request inside folder");
            }
        } else {
            panic!("Expected folder item");
        }
    }
}
