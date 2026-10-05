use crate::collection::Collection;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CollectionRepositoryError {
    #[error("Error de almacenamiento: {0}")]
    Storage(String),
    #[error("Ruta no válida: {0}")]
    InvalidPath(String),
}

/// Port defining collection persistence operations.
/// Implementations reside in infrastructure crates such as `kestrel-storage`.
pub trait CollectionRepository {
    fn load_from_path(&self, path: &Path) -> Result<Collection, CollectionRepositoryError>;
    fn save_to_path(
        &self,
        path: &Path,
        collection: &Collection,
    ) -> Result<(), CollectionRepositoryError>;
}
