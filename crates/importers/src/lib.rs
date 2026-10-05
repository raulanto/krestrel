//! Importers for Postman and Yaak formats into kestrel-core domain models.

pub mod postman;
pub mod yaak;

use kestrel_core::Collection;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ImportError {
    #[error("Formato JSON inválido: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Estructura de colección no compatible: {0}")]
    InvalidStructure(String),
}

pub trait Importer {
    fn import_str(&self, raw: &str) -> Result<Collection, ImportError>;
}
