use crate::{ImportError, Importer};
use kestrel_core::Collection;

pub struct YaakImporter;

impl Importer for YaakImporter {
    fn import_str(&self, raw: &str) -> Result<Collection, ImportError> {
        let val: serde_json::Value = serde_json::from_str(raw)?;
        let name = val["name"].as_str().unwrap_or("Yaak Workspace").to_string();

        Ok(Collection {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            description: None,
            items: vec![],
            environments: vec![],
            path: None,
            is_bundled: false,
            dirty: false,
            extra: Default::default(),
        })
    }
}
