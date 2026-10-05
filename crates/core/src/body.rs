use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Body {
    #[default]
    None,
    Raw {
        content: String,
        content_type: String,
    },
    Json {
        content: String,
    },
    FormData {
        entries: Vec<FormEntry>,
    },
    UrlEncoded {
        entries: Vec<KeyValuePair>,
    },
    Binary {
        file_path: String,
    },
    GraphQL {
        query: String,
        variables: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormEntry {
    pub key: String,
    pub value: FormValue,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FormValue {
    Text { text: String },
    File { file_path: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyValuePair {
    pub key: String,
    pub value: String,
    pub enabled: bool,
}
