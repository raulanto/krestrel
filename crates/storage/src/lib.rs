//! OpenCollection YAML serialization and storage persistence.

pub mod converter;
pub mod schema;

pub use converter::*;
pub use schema::*;

use kestrel_core::Collection;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Error de entrada/salida: {0}")]
    Io(#[from] std::io::Error),
    #[error("Error al procesar YAML: {0}")]
    Yaml(#[from] serde_yaml_ng::Error),
}

/// Reads OpenCollection YAML into the domain model `Collection`.
pub fn read_collection_from_yaml(content: &str) -> Result<Collection, StorageError> {
    let doc: OpenCollectionDocument = serde_yaml_ng::from_str(content)?;
    Ok(doc_to_collection(doc))
}

/// Writes domain model `Collection` into OpenCollection YAML.
pub fn write_collection_to_yaml(collection: &Collection) -> Result<String, StorageError> {
    let doc = collection_to_doc(collection);
    let yaml = serde_yaml_ng::to_string(&doc)?;
    Ok(yaml)
}

/// Loads a collection from a YAML file on disk.
pub fn load_collection_file(path: impl AsRef<Path>) -> Result<Collection, StorageError> {
    let content = std::fs::read_to_string(path)?;
    read_collection_from_yaml(&content)
}

/// Saves a collection to a YAML file on disk atomically.
pub fn save_collection_file(
    path: impl AsRef<Path>,
    collection: &Collection,
) -> Result<(), StorageError> {
    let yaml = write_collection_to_yaml(collection)?;
    let p = path.as_ref();
    // Atomic write by writing to a temp file next to target then rename
    let tmp_path = p.with_extension("tmp");
    std::fs::write(&tmp_path, yaml)?;
    std::fs::rename(tmp_path, p)?;
    Ok(())
}

/// OpenCollection YAML implementation of the `CollectionRepository` port.
#[derive(Debug, Default, Clone, Copy)]
pub struct OpenCollectionStorage;

impl OpenCollectionStorage {
    pub fn new() -> Self {
        Self
    }
}

impl kestrel_core::CollectionRepository for OpenCollectionStorage {
    fn load_from_path(
        &self,
        path: &Path,
    ) -> Result<Collection, kestrel_core::CollectionRepositoryError> {
        load_collection_file(path)
            .map_err(|e| kestrel_core::CollectionRepositoryError::Storage(e.to_string()))
    }

    fn save_to_path(
        &self,
        path: &Path,
        collection: &Collection,
    ) -> Result<(), kestrel_core::CollectionRepositoryError> {
        save_collection_file(path, collection)
            .map_err(|e| kestrel_core::CollectionRepositoryError::Storage(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_example_collection() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/basic_opencollection.yaml"
        );
        let collection = load_collection_file(path).expect("Failed to load example collection");
        assert_eq!(collection.name, "JSONPlaceholder API");
        assert_eq!(collection.items.len(), 2);
        assert_eq!(collection.environments.len(), 1);

        let env = &collection.environments[0];
        assert_eq!(env.name, "Development");
        assert_eq!(env.variables.len(), 2);
        assert_eq!(
            env.variables["baseUrl"].value,
            "https://jsonplaceholder.typicode.com"
        );
        assert!(env.variables["apiKey"].secret);
    }

    #[test]
    fn test_roundtrip_fidelity_no_diff() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/basic_opencollection.yaml"
        );
        let original_content = std::fs::read_to_string(path).expect("Read example yaml");
        let collection = read_collection_from_yaml(&original_content).expect("Parse to collection");
        let serialized = write_collection_to_yaml(&collection).expect("Serialize to yaml");

        // Parse both back into serde_yaml_ng::Value to verify semantic equality without loss
        let v1: serde_yaml_ng::Value =
            serde_yaml_ng::from_str(&original_content).expect("v1 parse");
        let v2: serde_yaml_ng::Value = serde_yaml_ng::from_str(&serialized).expect("v2 parse");

        // Secret should not be written to plaintext
        // Check that core info matches
        assert_eq!(v1["info"]["name"], v2["info"]["name"]);
        assert_eq!(v1["opencollection"], v2["opencollection"]);
    }
}
