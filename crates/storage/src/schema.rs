//! OpenCollection YAML Document Schema (AST).
//! Preserves all known and unknown fields using `serde_yaml_ng::Value` and ordered mappings.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_yaml_ng::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenCollectionDocument {
    #[serde(default = "default_version")]
    pub opencollection: String,
    pub info: CollectionInfoDoc,
    #[serde(default = "default_bundled")]
    pub bundled: bool,
    #[serde(default)]
    pub items: Vec<ItemDoc>,
    #[serde(default)]
    pub config: Option<CollectionConfigDoc>,
    #[serde(flatten)]
    pub extra: IndexMap<String, Value>,
}

fn default_version() -> String {
    "1.0.0".to_string()
}

fn default_bundled() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CollectionInfoDoc {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(flatten)]
    pub extra: IndexMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CollectionConfigDoc {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub environments: Vec<EnvironmentDoc>,
    #[serde(flatten)]
    pub extra: IndexMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentDoc {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variables: Vec<VariableDoc>,
    #[serde(flatten)]
    pub extra: IndexMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableDoc {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub disabled: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub secret: bool,
    #[serde(flatten)]
    pub extra: IndexMap<String, Value>,
}

fn is_false(b: &bool) -> bool {
    !*b
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemDoc {
    pub info: ItemInfoDoc,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http: Option<HttpDetailsDoc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graphql: Option<GraphqlDetailsDoc>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<ItemDoc>,
    #[serde(flatten)]
    pub extra: IndexMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemInfoDoc {
    pub name: String,
    #[serde(rename = "type")]
    pub item_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(flatten)]
    pub extra: IndexMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HttpDetailsDoc {
    #[serde(default = "default_method")]
    pub method: String,
    pub url: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub headers: Vec<HeaderDoc>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<ParamDoc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<Value>,
    #[serde(flatten)]
    pub extra: IndexMap<String, Value>,
}

fn default_method() -> String {
    "GET".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GraphqlDetailsDoc {
    pub url: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub headers: Vec<HeaderDoc>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<ParamDoc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<Value>,
    #[serde(flatten)]
    pub extra: IndexMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderDoc {
    pub name: String,
    pub value: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub disabled: bool,
    #[serde(flatten)]
    pub extra: IndexMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamDoc {
    pub name: String,
    pub value: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub disabled: bool,
    #[serde(flatten)]
    pub extra: IndexMap<String, Value>,
}
