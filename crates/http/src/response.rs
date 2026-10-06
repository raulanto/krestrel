use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sizes {
    pub headers_bytes: usize,
    pub body_bytes: usize,
    pub decompressed_bytes: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Timing {
    pub total: Duration,
    pub dns: Option<Duration>,
    pub connect: Option<Duration>,
    pub ttfb: Option<Duration>,
    pub download: Option<Duration>,
}

impl Timing {
    pub fn from_total(total: Duration) -> Self {
        Self {
            total,
            dns: None,
            connect: None,
            ttfb: None,
            download: None,
        }
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RequestErrorKind {
    Dns,
    Tls,
    Timeout,
    ConnectionRefused,
    InvalidUrl,
    Cancelled,
    Io,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestError {
    pub kind: RequestErrorKind,
    pub message: String,
    pub details: Option<String>,
}

impl std::fmt::Display for RequestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for RequestError {}

impl RequestError {
    pub fn new(kind: RequestErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(
        kind: RequestErrorKind,
        message: impl Into<String>,
        details: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            message: message.into(),
            details: Some(details.into()),
        }
    }

    pub fn dns(msg: impl Into<String>) -> Self {
        Self::new(RequestErrorKind::Dns, msg)
    }

    pub fn timeout(msg: impl Into<String>) -> Self {
        Self::new(RequestErrorKind::Timeout, msg)
    }

    pub fn tls(msg: impl Into<String>) -> Self {
        Self::new(RequestErrorKind::Tls, msg)
    }

    pub fn connection_refused(msg: impl Into<String>) -> Self {
        Self::new(RequestErrorKind::ConnectionRefused, msg)
    }

    pub fn cancelled() -> Self {
        Self::new(RequestErrorKind::Cancelled, "Solicitud cancelada")
    }

    pub fn from_reqwest(err: &reqwest::Error) -> Self {
        let msg = err.to_string();
        if err.is_timeout() {
            Self::with_details(RequestErrorKind::Timeout, "Tiempo de espera agotado", msg)
        } else if err.is_connect() {
            if msg.to_lowercase().contains("dns") || msg.to_lowercase().contains("resolve") {
                Self::with_details(
                    RequestErrorKind::Dns,
                    "Error al resolver el nombre de host (DNS)",
                    msg,
                )
            } else if msg.to_lowercase().contains("cert") || msg.to_lowercase().contains("tls") {
                Self::with_details(RequestErrorKind::Tls, "Error de verificación TLS/SSL", msg)
            } else {
                Self::with_details(
                    RequestErrorKind::ConnectionRefused,
                    "Conexión rechazada o no disponible",
                    msg,
                )
            }
        } else if err.is_builder() || err.is_request() {
            Self::with_details(
                RequestErrorKind::InvalidUrl,
                "Error en la URL o configuración de la solicitud",
                msg,
            )
        } else {
            Self::with_details(
                RequestErrorKind::Other,
                "Error de red al enviar la solicitud",
                msg,
            )
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResponseState {
    Idle,
    Loading { started: Instant },
    Done(Arc<ResponseData>),
    Failed(RequestError),
    Cancelled,
}

pub type HttpResponse = ResponseData;

impl ResponseData {
    pub fn is_empty(&self) -> bool {
        self.body.is_empty() || self.status == 204
    }

    pub fn body_as_str(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(&self.body)
    }
}
