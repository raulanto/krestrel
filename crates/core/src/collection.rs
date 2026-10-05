use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::environment::Environment;
use crate::request::Request;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Collection {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub items: Vec<CollectionItem>,
    #[serde(default)]
    pub environments: Vec<Environment>,
    #[serde(default, flatten)]
    pub extra: IndexMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CollectionItem {
    Folder(Folder),
    Request(Request),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Folder {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub items: Vec<CollectionItem>,
    #[serde(default, flatten)]
    pub extra: IndexMap<String, serde_json::Value>,
}
