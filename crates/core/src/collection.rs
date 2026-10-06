use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::environment::Environment;
use crate::request::Request;

use std::path::PathBuf;

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
    #[serde(default)]
    pub path: Option<PathBuf>,
    #[serde(default)]
    pub is_bundled: bool,
    #[serde(default)]
    pub dirty: bool,
    #[serde(default, flatten)]
    pub extra: IndexMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CollectionItem {
    Folder(Folder),
    Request(Request),
    ErrorNode(ItemErrorNode),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemErrorNode {
    pub id: String,
    pub name: String,
    pub file_path: PathBuf,
    pub error_message: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Folder {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub seq: Option<usize>,
    #[serde(default)]
    pub items: Vec<CollectionItem>,
    #[serde(default)]
    pub path: Option<PathBuf>,
    #[serde(default)]
    pub dirty: bool,
    #[serde(default, flatten)]
    pub extra: IndexMap<String, serde_json::Value>,
}

impl Collection {
    pub fn find_request(&self, id: &str) -> Option<&Request> {
        find_request_in_items(&self.items, id)
    }

    pub fn find_request_mut(&mut self, id: &str) -> Option<&mut Request> {
        find_request_in_items_mut(&mut self.items, id)
    }

    pub fn update_request(&mut self, request: Request) -> bool {
        if let Some(target) = self.find_request_mut(&request.id) {
            *target = request;
            true
        } else {
            false
        }
    }
}

fn find_request_in_items_mut<'a>(
    items: &'a mut [CollectionItem],
    id: &str,
) -> Option<&'a mut Request> {
    for item in items {
        match item {
            CollectionItem::Request(req) => {
                if req.id == id {
                    return Some(req);
                }
            }
            CollectionItem::Folder(folder) => {
                if let Some(req) = find_request_in_items_mut(&mut folder.items, id) {
                    return Some(req);
                }
            }
            CollectionItem::ErrorNode(_) => {}
        }
    }
    None
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
            CollectionItem::ErrorNode(_) => {}
        }
    }
    None
}
