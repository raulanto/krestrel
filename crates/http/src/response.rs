use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sizes {
    pub headers_bytes: usize,
    pub body_bytes: usize,
    pub decompressed_bytes: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Timing {
    pub total: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseData {
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<(String, String)>,
    #[serde(with = "serde_bytes_impl")]
    pub body: Bytes,
    pub content_type: Option<String>,
    pub timing: Timing,
    pub size: Sizes,
    pub http_version: String,
    pub url_final: String,
}

mod serde_bytes_impl {
    use bytes::Bytes;
    use serde::{Deserializer, Serializer};

    pub fn serialize<S>(bytes: &Bytes, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(bytes)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Bytes, D::Error>
    where
        D: Deserializer<'de>,
    {
        let vec: Vec<u8> = serde::Deserialize::deserialize(deserializer)?;
        Ok(Bytes::from(vec))
    }
}

#[derive(Debug, Clone)]
pub enum ResponseState {
    Idle,
    Loading,
    Done(std::sync::Arc<ResponseData>),
    Failed(String),
    Cancelled,
}

// Retro-compatibility alias while updating UI crate
pub type HttpResponse = ResponseData;

impl ResponseData {
    pub fn body_as_str(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(&self.body)
    }
}
