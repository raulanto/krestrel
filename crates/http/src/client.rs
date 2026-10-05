use crate::response::HttpResponse;
use kestrel_core::Request;
use std::time::Instant;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HttpError {
    #[error("Error de red: {0}")]
    Network(#[from] reqwest::Error),
    #[error("URL inválida: {0}")]
    InvalidUrl(#[from] url::ParseError),
    #[error("Error al preparar la solicitud: {0}")]
    Build(String),
}

pub struct HttpClient {
    client: reqwest::Client,
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder().build().unwrap_or_default(),
        }
    }

    pub async fn execute(&self, request: &Request) -> Result<HttpResponse, HttpError> {
        let start = Instant::now();
        let method = match request.method {
            kestrel_core::HttpMethod::GET => reqwest::Method::GET,
            kestrel_core::HttpMethod::POST => reqwest::Method::POST,
            kestrel_core::HttpMethod::PUT => reqwest::Method::PUT,
            kestrel_core::HttpMethod::DELETE => reqwest::Method::DELETE,
            kestrel_core::HttpMethod::PATCH => reqwest::Method::PATCH,
            kestrel_core::HttpMethod::HEAD => reqwest::Method::HEAD,
            kestrel_core::HttpMethod::OPTIONS => reqwest::Method::OPTIONS,
        };

        let mut req_builder = self.client.request(method, &request.url);

        for h in &request.headers {
            if h.enabled {
                req_builder = req_builder.header(&h.key, &h.value);
            }
        }

        let resp = req_builder.send().await?;
        let duration = start.elapsed();
        let status = resp.status().as_u16();
        let status_text = resp.status().canonical_reason().unwrap_or("").to_string();

        let mut headers = indexmap::IndexMap::new();
        for (name, val) in resp.headers() {
            if let Ok(str_val) = val.to_str() {
                headers.insert(name.as_str().to_string(), str_val.to_string());
            }
        }

        let body_bytes = resp.bytes().await?.to_vec();
        let size_bytes = body_bytes.len();

        Ok(HttpResponse {
            status,
            status_text,
            headers,
            body: body_bytes,
            duration,
            size_bytes,
        })
    }
}
