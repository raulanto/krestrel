//! OpenCollection YAML serialization and storage persistence.

use kestrel_core::Collection;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Error de entrada/salida: {0}")]
    Io(#[from] std::io::Error),
    #[error("Error al procesar YAML: {0}")]
    Yaml(#[from] serde_yaml::Error),
}

pub fn read_collection_from_yaml(content: &str) -> Result<Collection, StorageError> {
    let collection: Collection = serde_yaml::from_str(content)?;
    Ok(collection)
}

pub fn write_collection_to_yaml(collection: &Collection) -> Result<String, StorageError> {
    let yaml = serde_yaml::to_string(collection)?;
    Ok(yaml)
}

pub fn load_collection_file(path: impl AsRef<Path>) -> Result<Collection, StorageError> {
    let content = std::fs::read_to_string(path)?;
    read_collection_from_yaml(&content)
}

pub fn save_collection_file(
    path: impl AsRef<Path>,
    collection: &Collection,
) -> Result<(), StorageError> {
    let yaml = write_collection_to_yaml(collection)?;
    std::fs::write(path, yaml)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_core::{Collection, HttpMethod, Request};

    #[test]
    fn test_roundtrip_collection() {
        let collection = Collection {
            id: "col-1".to_string(),
            name: "Test Collection".to_string(),
            description: Some("Una colección de prueba".to_string()),
            items: vec![kestrel_core::CollectionItem::Request(Request {
                id: "req-1".to_string(),
                name: "Get Users".to_string(),
                method: HttpMethod::GET,
                url: "https://api.example.com/users".to_string(),
                headers: vec![],
                params: vec![],
                auth: Default::default(),
                body: Default::default(),
                description: None,
                extra: Default::default(),
            })],
            environments: vec![],
            extra: Default::default(),
        };

        let yaml = write_collection_to_yaml(&collection).expect("Failed to write yaml");
        let deserialized = read_collection_from_yaml(&yaml).expect("Failed to read yaml");
        assert_eq!(collection, deserialized);
    }
}
