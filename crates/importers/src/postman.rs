use crate::{ImportError, Importer};
use kestrel_core::Collection;

pub struct PostmanImporter;

impl Importer for PostmanImporter {
    fn import_str(&self, raw: &str) -> Result<Collection, ImportError> {
        let val: serde_json::Value = serde_json::from_str(raw)?;
        let name = val["info"]["name"]
            .as_str()
            .unwrap_or("Postman Collection")
            .to_string();

        Ok(Collection {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            description: val["info"]["description"].as_str().map(|s| s.to_string()),
            items: vec![],
            environments: vec![],
            extra: Default::default(),
        })
    }
}
