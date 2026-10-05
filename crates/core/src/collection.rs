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

impl Collection {
    pub fn find_request(&self, id: &str) -> Option<&Request> {
        find_request_in_items(&self.items, id)
    }
}

fn find_request_in_items<'a>(items: &'a [CollectionItem], id: &str) -> Option<&'a Request> {
    for item in items {
        match item {
            CollectionItem::Request(req) => {
                if req.id == id {
                    return Some(req);
                }
            }
            CollectionItem::Folder(folder) => {
                if let Some(req) = find_request_in_items(&folder.items, id) {
                    return Some(req);
                }
            }
        }
    }
    None
}
