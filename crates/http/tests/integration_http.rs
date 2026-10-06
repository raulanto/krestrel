use kestrel_core::{Auth, Body, Environment, HeaderParam, HttpMethod, QueryParam, Request};
use kestrel_http::HttpClient;
use std::net::SocketAddr;
use tokio::sync::oneshot;

// Local mock server runner using hyper & tokio
async fn start_mock_server() -> (SocketAddr, oneshot::Sender<()>) {
    use hyper::body::Incoming;
    use hyper::service::service_fn;
    use hyper::{Request as HyperRequest, Response as HyperResponse};
    use hyper_util::rt::TokioIo;
    use hyper_util::server::conn::auto;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = oneshot::channel::<()>();

    tokio::spawn(async move {
        let mut rx = rx;
        loop {
            tokio::select! {
                res = listener.accept() => {
                    let (stream, _) = match res {
                        Ok(val) => val,
                        Err(_) => break,
                    };
                    let io = TokioIo::new(stream);
                    tokio::spawn(async move {
                        let service = service_fn(|req: HyperRequest<Incoming>| async move {
                            let path = req.uri().path().to_string();
                            let query = req.uri().query().unwrap_or("").to_string();
                            let headers = req.headers().clone();

                            let body_bytes = http_body_util::BodyExt::collect(req.into_body())
                                .await
                                .unwrap()
                                .to_bytes();

                            let mut res_map = serde_json::json!({
                                "path": path,
                                "query": query,
                                "body": String::from_utf8_lossy(&body_bytes),
                            });

                            if let Some(auth_hdr) = headers.get("authorization") {
                                res_map["auth_header"] = serde_json::json!(auth_hdr.to_str().unwrap_or(""));
                            }
                            if let Some(custom_hdr) = headers.get("x-api-key") {
                                res_map["api_key_header"] = serde_json::json!(custom_hdr.to_str().unwrap_or(""));
                            }

                            let body_str = serde_json::to_string(&res_map).unwrap();

                            Ok::<_, hyper::Error>(
                                HyperResponse::builder()
                                    .status(200)
                                    .header("content-type", "application/json")
                                    .body(http_body_util::Full::new(bytes::Bytes::from(body_str)))
                                    .unwrap()
                            )
                        });

                        let _ = auto::Builder::new(hyper_util::rt::TokioExecutor::new())
                            .serve_connection(io, service)
                            .await;
                    });
                }
                _ = &mut rx => {
                    break;
                }
            }
        }
    });

    (addr, tx)
}

#[tokio::test]
async fn test_mock_server_http_get_with_query_and_auth() {
    let (addr, shutdown_tx) = start_mock_server().await;
    let base_url = format!("http://{}", addr);

    let mut env = Environment::new("test-env", "Test Env");
    env = env.with_variable("host", base_url, true, false);
    env = env.with_variable("token", "secret-bearer-123", true, true);

    let req = Request {
        id: "req1".to_string(),
        name: "Get Users".to_string(),
        method: HttpMethod::GET,
        url: "{{host}}/users".to_string(),
        headers: vec![HeaderParam {
            key: "X-Custom".to_string(),
            value: "Val".to_string(),
            enabled: true,
        }],
        params: vec![QueryParam {
            key: "page".to_string(),
            value: "1".to_string(),
            enabled: true,
        }],
        auth: Auth::Bearer {
            token: "{{token}}".to_string(),
        },
        body: Body::None,
        description: None,
        extra: indexmap::IndexMap::new(),
    };

    let client = HttpClient::new();
    let resp = client.execute(&req, &[&env]).await.unwrap();

    assert_eq!(resp.status, 200);
    assert!(resp.size.body_bytes > 0);

    let json_body: serde_json::Value = serde_json::from_slice(&resp.body).unwrap();
    assert_eq!(json_body["path"], "/users");
    assert_eq!(json_body["query"], "page=1");
    assert_eq!(json_body["auth_header"], "Bearer secret-bearer-123");

    let _ = shutdown_tx.send(());
}

#[tokio::test]
async fn test_mock_server_graphql_post() {
    let (addr, shutdown_tx) = start_mock_server().await;
    let base_url = format!("http://{}", addr);

    let mut env = Environment::new("gql-env", "GQL Env");
    env = env.with_variable("endpoint", format!("{}/graphql", base_url), true, false);

    let req = Request {
        id: "req2".to_string(),
        name: "GraphQL Query".to_string(),
        method: HttpMethod::POST,
        url: "{{endpoint}}".to_string(),
        headers: vec![],
        params: vec![],
        auth: Auth::None,
        body: Body::GraphQL {
            query: "query GetItem($id: ID!) { item(id: $id) { name } }".to_string(),
            variables: Some(r#"{"id": "42"}"#.to_string()),
        },
        description: None,
        extra: indexmap::IndexMap::new(),
    };

    let client = HttpClient::new();
    let resp = client.execute(&req, &[&env]).await.unwrap();

    assert_eq!(resp.status, 200);

    let json_body: serde_json::Value = serde_json::from_slice(&resp.body).unwrap();
    assert_eq!(json_body["path"], "/graphql");

    let parsed_body: serde_json::Value =
        serde_json::from_str(json_body["body"].as_str().unwrap()).unwrap();
    assert_eq!(
        parsed_body["query"],
        "query GetItem($id: ID!) { item(id: $id) { name } }"
    );
    assert_eq!(parsed_body["variables"]["id"], "42");

    let _ = shutdown_tx.send(());
}
