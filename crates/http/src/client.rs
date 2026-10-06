use crate::response::HttpResponse;
use kestrel_core::{
    ApiKeyLocation, Auth, Body, Environment, HttpMethod, Request, resolve_variables,
};
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
    #[error("Error de E/S de archivo: {0}")]
    Io(#[from] std::io::Error),
}

impl From<HttpError> for crate::response::RequestError {
    fn from(err: HttpError) -> Self {
        match err {
            HttpError::Network(ref req_err) => crate::response::RequestError::from_reqwest(req_err),
            HttpError::InvalidUrl(ref parse_err) => crate::response::RequestError::with_details(
                crate::response::RequestErrorKind::InvalidUrl,
                "URL inválida",
                parse_err.to_string(),
            ),
            HttpError::Build(ref msg) => crate::response::RequestError::with_details(
                crate::response::RequestErrorKind::Other,
                "Error de configuración",
                msg.clone(),
            ),
            HttpError::Io(ref io_err) => crate::response::RequestError::with_details(
                crate::response::RequestErrorKind::Io,
                "Error de lectura de archivo",
                io_err.to_string(),
            ),
        }
    }
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
            client: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::limited(10))
                .build()
                .unwrap_or_default(),
        }
    }

    /// Prepares and executes a `Request`, resolving variables against the provided environments,
    /// injecting Authentication (Bearer, Basic, API Key), query parameters, and Body (JSON, Text, XML,
    /// Form URL-Encoded, Multipart, Binary file, or GraphQL).
    pub async fn execute(
        &self,
        request: &Request,
        environments: &[&Environment],
    ) -> Result<HttpResponse, HttpError> {
        let start = Instant::now();

        // 1. Resolve URL with variables
        let resolved_url_str = resolve_variables(&request.url, environments);

        // 2. Parse URL and append enabled query parameters
        let mut parsed_url = url::Url::parse(&resolved_url_str)?;

        // Append query parameters from request.params
        for p in &request.params {
            if p.enabled {
                let resolved_key = resolve_variables(&p.key, environments);
                let resolved_val = resolve_variables(&p.value, environments);
                parsed_url
                    .query_pairs_mut()
                    .append_pair(&resolved_key, &resolved_val);
            }
        }

        // Handle ApiKey location in Query
        if let Auth::ApiKey {
            key,
            value,
            location: ApiKeyLocation::Query,
        } = &request.auth
        {
            let resolved_key = resolve_variables(key, environments);
            let resolved_val = resolve_variables(value, environments);
            parsed_url
                .query_pairs_mut()
                .append_pair(&resolved_key, &resolved_val);
        }

        // 3. Map HTTP Method
        let method = match request.method {
            HttpMethod::GET => reqwest::Method::GET,
            HttpMethod::POST => reqwest::Method::POST,
            HttpMethod::PUT => reqwest::Method::PUT,
            HttpMethod::DELETE => reqwest::Method::DELETE,
            HttpMethod::PATCH => reqwest::Method::PATCH,
            HttpMethod::HEAD => reqwest::Method::HEAD,
            HttpMethod::OPTIONS => reqwest::Method::OPTIONS,
        };

        let mut req_builder = self.client.request(method, parsed_url);

        // 4. Set Headers
        for h in &request.headers {
            if h.enabled {
                let resolved_key = resolve_variables(&h.key, environments);
                let resolved_val = resolve_variables(&h.value, environments);
                req_builder = req_builder.header(&resolved_key, &resolved_val);
            }
        }

        // 5. Inject Authentication
        match &request.auth {
            Auth::None => {}
            Auth::Bearer { token } => {
                let resolved_token = resolve_variables(token, environments);
                req_builder = req_builder.bearer_auth(resolved_token);
            }
            Auth::Basic { username, password } => {
                let resolved_user = resolve_variables(username, environments);
                let resolved_pass = resolve_variables(password, environments);
                req_builder = req_builder.basic_auth(resolved_user, Some(resolved_pass));
            }
            Auth::ApiKey {
                key,
                value,
                location: ApiKeyLocation::Header,
            } => {
                let resolved_key = resolve_variables(key, environments);
                let resolved_val = resolve_variables(value, environments);
                req_builder = req_builder.header(&resolved_key, &resolved_val);
            }
            Auth::ApiKey {
                location: ApiKeyLocation::Query,
                ..
            } => {
                // Handled above in URL query pairs
            }
        }

        // 6. Set Body
        req_builder = match &request.body {
            Body::None => req_builder,
            Body::Raw {
                content,
                content_type,
            } => {
                let resolved_content = resolve_variables(content, environments);
                let resolved_type = resolve_variables(content_type, environments);
                req_builder
                    .header(reqwest::header::CONTENT_TYPE, resolved_type)
                    .body(resolved_content)
            }
            Body::Json { content } => {
                let resolved_content = resolve_variables(content, environments);
                req_builder
                    .header(reqwest::header::CONTENT_TYPE, "application/json")
                    .body(resolved_content)
            }
            Body::UrlEncoded { entries } => {
                let mut params = Vec::new();
                for entry in entries {
                    if entry.enabled {
                        let k = resolve_variables(&entry.key, environments);
                        let v = resolve_variables(&entry.value, environments);
                        params.push((k, v));
                    }
                }
                req_builder.form(&params)
            }
            Body::FormData { entries } => {
                let mut form = reqwest::multipart::Form::new();
                for entry in entries {
                    if entry.enabled {
                        let k = resolve_variables(&entry.key, environments);
                        match &entry.value {
                            kestrel_core::FormValue::Text { text } => {
                                let v = resolve_variables(text, environments);
                                form = form.text(k, v);
                            }
                            kestrel_core::FormValue::File { file_path } => {
                                let resolved_path = resolve_variables(file_path, environments);
                                let file_bytes = tokio::fs::read(&resolved_path).await?;
                                let file_name = std::path::Path::new(&resolved_path)
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("file")
                                    .to_string();
                                let part = reqwest::multipart::Part::bytes(file_bytes)
                                    .file_name(file_name);
                                form = form.part(k, part);
                            }
                        }
                    }
                }
                req_builder.multipart(form)
            }
            Body::Binary { file_path } => {
                let resolved_path = resolve_variables(file_path, environments);
                let file_bytes = tokio::fs::read(&resolved_path).await?;
                req_builder
                    .header(reqwest::header::CONTENT_TYPE, "application/octet-stream")
                    .body(file_bytes)
            }
            Body::GraphQL { query, variables } => {
                let resolved_query = resolve_variables(query, environments);
                let parsed_vars = if let Some(vars_str) = variables {
                    let resolved_vars = resolve_variables(vars_str, environments);
                    serde_json::from_str::<serde_json::Value>(&resolved_vars).ok()
                } else {
                    None
                };

                let graphql_payload = serde_json::json!({
                    "query": resolved_query,
                    "variables": parsed_vars,
                });

                req_builder
                    .header(reqwest::header::CONTENT_TYPE, "application/json")
                    .body(graphql_payload.to_string())
            }
        };

        // 7. Send Request & Measure Metrics
        let resp = req_builder.send().await?;
        let url_final = resp.url().to_string();
        let http_version = format!("{:?}", resp.version());
        let status = resp.status().as_u16();
        let status_text = resp.status().canonical_reason().unwrap_or("").to_string();

        let mut headers = Vec::new();
        let mut headers_bytes = 0;
        let mut content_type = None;

        for (name, val) in resp.headers() {
            if let Ok(str_val) = val.to_str() {
                if name.as_str().eq_ignore_ascii_case("content-type") {
                    content_type = Some(str_val.to_string());
                }
                headers_bytes += name.as_str().len() + str_val.len() + 4;
                headers.push((name.as_str().to_string(), str_val.to_string()));
            }
        }

        let body_bytes = resp.bytes().await?;
        let duration = start.elapsed();
        let body_len = body_bytes.len();

        Ok(HttpResponse {
            status,
            status_text,
            headers,
            body: body_bytes,
            content_type,
            timing: crate::response::Timing::from_total(duration),
            size: crate::response::Sizes {
                headers_bytes,
                body_bytes: body_len,
                decompressed_bytes: Some(body_len),
            },
            http_version,
            url_final,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_core::{EnvVariable, Environment, HeaderParam, HttpMethod, QueryParam};

    #[test]
    fn test_client_creation() {
        let client = HttpClient::new();
        assert!(!format!("{:p}", &client).is_empty());
    }

    #[tokio::test]
    async fn test_url_variable_resolution_and_query_params() {
        let mut env = Environment::new("test-env".to_string(), "Test Env");
        env.variables.insert(
            "baseUrl".to_string(),
            EnvVariable {
                value: "http://127.0.0.1:9099".to_string(),
                enabled: true,
                secret: false,
            },
        );

        let req = Request {
            id: "req1".to_string(),
            name: "Test Request".to_string(),
            method: HttpMethod::GET,
            url: "{{baseUrl}}/api/v1/users".to_string(),
            headers: vec![HeaderParam {
                key: "X-Custom".to_string(),
                value: "CustomVal".to_string(),
                enabled: true,
            }],
            params: vec![QueryParam {
                key: "limit".to_string(),
                value: "10".to_string(),
                enabled: true,
            }],
            auth: Auth::None,
            body: Body::None,
            description: None,
            seq: None,
            path: None,
            dirty: false,
            extra: indexmap::IndexMap::new(),
        };

        let resolved_url = resolve_variables(&req.url, &[&env]);
        assert_eq!(resolved_url, "http://127.0.0.1:9099/api/v1/users");

        let mut parsed = url::Url::parse(&resolved_url).unwrap();
        for p in &req.params {
            parsed.query_pairs_mut().append_pair(&p.key, &p.value);
        }
        assert_eq!(
            parsed.as_str(),
            "http://127.0.0.1:9099/api/v1/users?limit=10"
        );
    }

    #[test]
    fn test_graphql_body_json_structure() {
        let query = "query GetUser { user { id name } }";
        let vars = r#"{"limit": 5}"#;
        let parsed_vars = serde_json::from_str::<serde_json::Value>(vars).ok();

        let payload = serde_json::json!({
            "query": query,
            "variables": parsed_vars,
        });

        assert_eq!(payload["query"], query);
        assert_eq!(payload["variables"]["limit"], 5);
    }
}
