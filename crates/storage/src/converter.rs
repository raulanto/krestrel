use crate::schema::*;
use indexmap::IndexMap;
use kestrel_core::{
    ApiKeyLocation, Auth, Body, Collection, CollectionItem, EnvVariable, Environment, Folder,
    FormEntry, FormValue, HeaderParam, HttpMethod, KeyValuePair, QueryParam, Request,
};
use serde_yaml_ng::Value;
use uuid::Uuid;

pub fn doc_to_collection(doc: OpenCollectionDocument) -> Collection {
    let collection_id = Uuid::new_v4().to_string();

    let environments = if let Some(config) = doc.config {
        config
            .environments
            .into_iter()
            .map(|env_doc| {
                let mut variables = IndexMap::new();
                for var in env_doc.variables {
                    variables.insert(
                        var.name,
                        EnvVariable {
                            value: var.value.unwrap_or_default(),
                            enabled: !var.disabled,
                            secret: var.secret,
                        },
                    );
                }
                Environment {
                    id: Uuid::new_v4().to_string(),
                    name: env_doc.name,
                    variables,
                }
            })
            .collect()
    } else {
        Vec::new()
    };

    let items = doc
        .items
        .into_iter()
        .map(doc_item_to_collection_item)
        .collect();

    let mut extra = IndexMap::new();
    for (k, v) in doc.extra {
        if let Ok(json_v) = serde_json::to_value(v) {
            extra.insert(k, json_v);
        }
    }

    Collection {
        id: collection_id,
        name: doc.info.name,
        description: doc.info.summary.or(doc.info.description),
        items,
        environments,
        extra,
    }
}

pub fn collection_to_doc(collection: &Collection) -> OpenCollectionDocument {
    let mut env_docs = Vec::new();
    for env in &collection.environments {
        let mut vars = Vec::new();
        for (var_name, var_val) in &env.variables {
            // As per AGENTS.md: Never save secrets in plaintext into the collection
            let value = if var_val.secret {
                None
            } else {
                Some(var_val.value.clone())
            };
            vars.push(VariableDoc {
                name: var_name.clone(),
                value,
                disabled: !var_val.enabled,
                secret: var_val.secret,
                extra: IndexMap::new(),
            });
        }
        env_docs.push(EnvironmentDoc {
            name: env.name.clone(),
            color: None,
            description: None,
            variables: vars,
            extra: IndexMap::new(),
        });
    }

    let items = collection
        .items
        .iter()
        .map(collection_item_to_doc_item)
        .collect();

    let mut extra = IndexMap::new();
    for (k, v) in &collection.extra {
        if let Ok(yaml_v) = serde_yaml_ng::to_value(v) {
            extra.insert(k.clone(), yaml_v);
        }
    }

    OpenCollectionDocument {
        opencollection: "1.0.0".to_string(),
        info: CollectionInfoDoc {
            name: collection.name.clone(),
            summary: collection.description.clone(),
            description: None,
            version: Some("1.0.0".to_string()),
            extra: IndexMap::new(),
        },
        bundled: true,
        items,
        config: Some(CollectionConfigDoc {
            environments: env_docs,
            extra: IndexMap::new(),
        }),
        extra,
    }
}

fn doc_item_to_collection_item(item: ItemDoc) -> CollectionItem {
    let item_type = item.info.item_type.to_lowercase();
    if item_type == "folder" {
        let sub_items = item
            .items
            .into_iter()
            .map(doc_item_to_collection_item)
            .collect();

        let mut extra = IndexMap::new();
        for (k, v) in item.extra {
            if let Ok(json_v) = serde_json::to_value(v) {
                extra.insert(k, json_v);
            }
        }

        CollectionItem::Folder(Folder {
            id: Uuid::new_v4().to_string(),
            name: item.info.name,
            description: item.info.description,
            items: sub_items,
            extra,
        })
    } else {
        // Request (http or graphql)
        let (method, url, headers, params, auth, body) = if let Some(http) = item.http {
            let m = parse_http_method(&http.method);
            let h = http
                .headers
                .into_iter()
                .map(|hdr| HeaderParam {
                    key: hdr.name,
                    value: hdr.value,
                    enabled: !hdr.disabled,
                })
                .collect();
            let p = http
                .params
                .into_iter()
                .map(|prm| QueryParam {
                    key: prm.name,
                    value: prm.value,
                    enabled: !prm.disabled,
                })
                .collect();
            let a = parse_auth(http.auth.as_ref());
            let b = parse_body(http.body.as_ref());
            (m, http.url, h, p, a, b)
        } else if let Some(graphql) = item.graphql {
            let m = HttpMethod::POST;
            let h = graphql
                .headers
                .into_iter()
                .map(|hdr| HeaderParam {
                    key: hdr.name,
                    value: hdr.value,
                    enabled: !hdr.disabled,
                })
                .collect();
            let p = graphql
                .params
                .into_iter()
                .map(|prm| QueryParam {
                    key: prm.name,
                    value: prm.value,
                    enabled: !prm.disabled,
                })
                .collect();
            let a = parse_auth(graphql.auth.as_ref());
            let b = parse_body(graphql.body.as_ref());
            (m, graphql.url, h, p, a, b)
        } else {
            (
                HttpMethod::GET,
                String::new(),
                Vec::new(),
                Vec::new(),
                Auth::None,
                Body::None,
            )
        };

        let mut extra = IndexMap::new();
        for (k, v) in item.extra {
            if let Ok(json_v) = serde_json::to_value(v) {
                extra.insert(k, json_v);
            }
        }

        CollectionItem::Request(Request {
            id: Uuid::new_v4().to_string(),
            name: item.info.name,
            method,
            url,
            headers,
            params,
            auth,
            body,
            description: item.info.description,
            extra,
        })
    }
}

fn collection_item_to_doc_item(item: &CollectionItem) -> ItemDoc {
    match item {
        CollectionItem::Folder(folder) => {
            let sub_items = folder
                .items
                .iter()
                .map(collection_item_to_doc_item)
                .collect();
            let mut extra = IndexMap::new();
            for (k, v) in &folder.extra {
                if let Ok(yaml_v) = serde_yaml_ng::to_value(v) {
                    extra.insert(k.clone(), yaml_v);
                }
            }
            ItemDoc {
                info: ItemInfoDoc {
                    name: folder.name.clone(),
                    item_type: "folder".to_string(),
                    description: folder.description.clone(),
                    extra: IndexMap::new(),
                },
                http: None,
                graphql: None,
                items: sub_items,
                extra,
            }
        }
        CollectionItem::Request(req) => {
            let mut extra = IndexMap::new();
            for (k, v) in &req.extra {
                if let Ok(yaml_v) = serde_yaml_ng::to_value(v) {
                    extra.insert(k.clone(), yaml_v);
                }
            }

            let headers = req
                .headers
                .iter()
                .map(|h| HeaderDoc {
                    name: h.key.clone(),
                    value: h.value.clone(),
                    disabled: !h.enabled,
                    extra: IndexMap::new(),
                })
                .collect();

            let params = req
                .params
                .iter()
                .map(|p| ParamDoc {
                    name: p.key.clone(),
                    value: p.value.clone(),
                    disabled: !p.enabled,
                    extra: IndexMap::new(),
                })
                .collect();

            let auth = serialize_auth(&req.auth);
            let body = serialize_body(&req.body);

            ItemDoc {
                info: ItemInfoDoc {
                    name: req.name.clone(),
                    item_type: "http".to_string(),
                    description: req.description.clone(),
                    extra: IndexMap::new(),
                },
                http: Some(HttpDetailsDoc {
                    method: format!("{:?}", req.method),
                    url: req.url.clone(),
                    headers,
                    params,
                    auth,
                    body,
                    extra: IndexMap::new(),
                }),
                graphql: None,
                items: Vec::new(),
                extra,
            }
        }
    }
}

fn parse_http_method(m: &str) -> HttpMethod {
    match m.to_uppercase().as_str() {
        "POST" => HttpMethod::POST,
        "PUT" => HttpMethod::PUT,
        "DELETE" => HttpMethod::DELETE,
        "PATCH" => HttpMethod::PATCH,
        "HEAD" => HttpMethod::HEAD,
        "OPTIONS" => HttpMethod::OPTIONS,
        _ => HttpMethod::GET,
    }
}

fn parse_auth(val: Option<&Value>) -> Auth {
    let Some(Value::Mapping(map)) = val else {
        return Auth::None;
    };
    if let Some(Value::String(auth_type)) = map.get(Value::String("type".to_string())) {
        match auth_type.to_lowercase().as_str() {
            "bearer" => {
                let token = map
                    .get(Value::String("token".to_string()))
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                return Auth::Bearer { token };
            }
            "basic" => {
                let username = map
                    .get(Value::String("username".to_string()))
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                let password = map
                    .get(Value::String("password".to_string()))
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                return Auth::Basic { username, password };
            }
            "apikey" => {
                let key = map
                    .get(Value::String("key".to_string()))
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                let value = map
                    .get(Value::String("value".to_string()))
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                let placement = map
                    .get(Value::String("placement".to_string()))
                    .and_then(Value::as_str)
                    .unwrap_or("header");
                let location = if placement == "query" {
                    ApiKeyLocation::Query
                } else {
                    ApiKeyLocation::Header
                };
                return Auth::ApiKey {
                    key,
                    value,
                    location,
                };
            }
            _ => {}
        }
    }
    Auth::None
}

fn serialize_auth(auth: &Auth) -> Option<Value> {
    match auth {
        Auth::None => None,
        Auth::Bearer { token } => {
            let mut map = serde_yaml_ng::Mapping::new();
            map.insert(
                Value::String("type".to_string()),
                Value::String("bearer".to_string()),
            );
            map.insert(
                Value::String("token".to_string()),
                Value::String(token.clone()),
            );
            Some(Value::Mapping(map))
        }
        Auth::Basic { username, password } => {
            let mut map = serde_yaml_ng::Mapping::new();
            map.insert(
                Value::String("type".to_string()),
                Value::String("basic".to_string()),
            );
            map.insert(
                Value::String("username".to_string()),
                Value::String(username.clone()),
            );
            map.insert(
                Value::String("password".to_string()),
                Value::String(password.clone()),
            );
            Some(Value::Mapping(map))
        }
        Auth::ApiKey {
            key,
            value,
            location,
        } => {
            let mut map = serde_yaml_ng::Mapping::new();
            map.insert(
                Value::String("type".to_string()),
                Value::String("apikey".to_string()),
            );
            map.insert(Value::String("key".to_string()), Value::String(key.clone()));
            map.insert(
                Value::String("value".to_string()),
                Value::String(value.clone()),
            );
            map.insert(
                Value::String("placement".to_string()),
                Value::String(match location {
                    ApiKeyLocation::Header => "header".to_string(),
                    ApiKeyLocation::Query => "query".to_string(),
                }),
            );
            Some(Value::Mapping(map))
        }
    }
}

fn parse_body(val: Option<&Value>) -> Body {
    let Some(Value::Mapping(map)) = val else {
        return Body::None;
    };
    if let Some(Value::String(body_type)) = map.get(Value::String("type".to_string())) {
        match body_type.to_lowercase().as_str() {
            "json" => {
                let data = map
                    .get(Value::String("data".to_string()))
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                return Body::Json { content: data };
            }
            "text" | "xml" | "sparql" => {
                let data = map
                    .get(Value::String("data".to_string()))
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                return Body::Raw {
                    content: data,
                    content_type: body_type.clone(),
                };
            }
            "formurlencoded" => {
                let mut entries = Vec::new();
                if let Some(Value::Sequence(seq)) = map.get(Value::String("data".to_string())) {
                    for item in seq {
                        if let Value::Mapping(field_map) = item {
                            let key = field_map
                                .get(Value::String("name".to_string()))
                                .and_then(Value::as_str)
                                .unwrap_or_default()
                                .to_string();
                            let val = field_map
                                .get(Value::String("value".to_string()))
                                .and_then(Value::as_str)
                                .unwrap_or_default()
                                .to_string();
                            let disabled = field_map
                                .get(Value::String("disabled".to_string()))
                                .and_then(Value::as_bool)
                                .unwrap_or(false);
                            entries.push(KeyValuePair {
                                key,
                                value: val,
                                enabled: !disabled,
                            });
                        }
                    }
                }
                return Body::UrlEncoded { entries };
            }
            "multipart" => {
                let mut entries = Vec::new();
                if let Some(Value::Sequence(seq)) = map.get(Value::String("data".to_string())) {
                    for item in seq {
                        if let Value::Mapping(part_map) = item {
                            let key = part_map
                                .get(Value::String("name".to_string()))
                                .and_then(Value::as_str)
                                .unwrap_or_default()
                                .to_string();
                            let val_str = part_map
                                .get(Value::String("value".to_string()))
                                .and_then(Value::as_str)
                                .unwrap_or_default()
                                .to_string();
                            let disabled = part_map
                                .get(Value::String("disabled".to_string()))
                                .and_then(Value::as_bool)
                                .unwrap_or(false);
                            let ptype = part_map
                                .get(Value::String("type".to_string()))
                                .and_then(Value::as_str)
                                .unwrap_or("text");
                            let form_val = if ptype == "file" {
                                FormValue::File { file_path: val_str }
                            } else {
                                FormValue::Text { text: val_str }
                            };
                            entries.push(FormEntry {
                                key,
                                value: form_val,
                                enabled: !disabled,
                            });
                        }
                    }
                }
                return Body::FormData { entries };
            }
            _ => {}
        }
    }
    Body::None
}

fn serialize_body(body: &Body) -> Option<Value> {
    match body {
        Body::None => None,
        Body::Json { content } => {
            let mut map = serde_yaml_ng::Mapping::new();
            map.insert(
                Value::String("type".to_string()),
                Value::String("json".to_string()),
            );
            map.insert(
                Value::String("data".to_string()),
                Value::String(content.clone()),
            );
            Some(Value::Mapping(map))
        }
        Body::Raw {
            content,
            content_type,
        } => {
            let mut map = serde_yaml_ng::Mapping::new();
            map.insert(
                Value::String("type".to_string()),
                Value::String(content_type.clone()),
            );
            map.insert(
                Value::String("data".to_string()),
                Value::String(content.clone()),
            );
            Some(Value::Mapping(map))
        }
        Body::UrlEncoded { entries } => {
            let mut map = serde_yaml_ng::Mapping::new();
            map.insert(
                Value::String("type".to_string()),
                Value::String("formurlencoded".to_string()),
            );
            let mut seq = Vec::new();
            for e in entries {
                let mut field = serde_yaml_ng::Mapping::new();
                field.insert(
                    Value::String("name".to_string()),
                    Value::String(e.key.clone()),
                );
                field.insert(
                    Value::String("value".to_string()),
                    Value::String(e.value.clone()),
                );
                if !e.enabled {
                    field.insert(Value::String("disabled".to_string()), Value::Bool(true));
                }
                seq.push(Value::Mapping(field));
            }
            map.insert(Value::String("data".to_string()), Value::Sequence(seq));
            Some(Value::Mapping(map))
        }
        Body::FormData { entries } => {
            let mut map = serde_yaml_ng::Mapping::new();
            map.insert(
                Value::String("type".to_string()),
                Value::String("multipart".to_string()),
            );
            let mut seq = Vec::new();
            for e in entries {
                let mut field = serde_yaml_ng::Mapping::new();
                field.insert(
                    Value::String("name".to_string()),
                    Value::String(e.key.clone()),
                );
                match &e.value {
                    FormValue::Text { text } => {
                        field.insert(
                            Value::String("type".to_string()),
                            Value::String("text".to_string()),
                        );
                        field.insert(
                            Value::String("value".to_string()),
                            Value::String(text.clone()),
                        );
                    }
                    FormValue::File { file_path } => {
                        field.insert(
                            Value::String("type".to_string()),
                            Value::String("file".to_string()),
                        );
                        field.insert(
                            Value::String("value".to_string()),
                            Value::String(file_path.clone()),
                        );
                    }
                }
                if !e.enabled {
                    field.insert(Value::String("disabled".to_string()), Value::Bool(true));
                }
                seq.push(Value::Mapping(field));
            }
            map.insert(Value::String("data".to_string()), Value::Sequence(seq));
            Some(Value::Mapping(map))
        }
        Body::Binary { file_path } => {
            let mut map = serde_yaml_ng::Mapping::new();
            map.insert(
                Value::String("type".to_string()),
                Value::String("file".to_string()),
            );
            map.insert(
                Value::String("src".to_string()),
                Value::String(file_path.clone()),
            );
            Some(Value::Mapping(map))
        }
        Body::GraphQL { query, variables } => {
            let mut map = serde_yaml_ng::Mapping::new();
            map.insert(
                Value::String("type".to_string()),
                Value::String("graphql".to_string()),
            );
            map.insert(
                Value::String("query".to_string()),
                Value::String(query.clone()),
            );
            if let Some(vars) = variables {
                map.insert(
                    Value::String("variables".to_string()),
                    Value::String(vars.clone()),
                );
            }
            Some(Value::Mapping(map))
        }
    }
}
