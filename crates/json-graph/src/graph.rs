use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Graph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub title: String,
    pub fields: Vec<NodeField>,
    pub is_expandable: bool,
    pub is_collapsed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeField {
    pub key: String,
    pub value_repr: String,
    pub value_type: ValueType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValueType {
    String,
    Number,
    Boolean,
    Null,
    Array,
    Object,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphEdge {
    pub from_node_id: String,
    pub to_node_id: String,
    pub label: Option<String>,
}

pub fn json_to_graph(val: &serde_json::Value) -> Graph {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    parse_value(val, "root", "root", &mut nodes, &mut edges);

    Graph { nodes, edges }
}

fn parse_value(
    val: &serde_json::Value,
    node_id: &str,
    title: &str,
    nodes: &mut Vec<GraphNode>,
    edges: &mut Vec<GraphEdge>,
) {
    match val {
        serde_json::Value::Object(map) => {
            let mut fields = Vec::new();
            for (k, v) in map {
                match v {
                    serde_json::Value::Object(_) | serde_json::Value::Array(_) => {
                        let child_id = format!("{}_{}", node_id, k);
                        edges.push(GraphEdge {
                            from_node_id: node_id.to_string(),
                            to_node_id: child_id.clone(),
                            label: Some(k.clone()),
                        });
                        parse_value(v, &child_id, k, nodes, edges);
                    }
                    serde_json::Value::String(s) => {
                        fields.push(NodeField {
                            key: k.clone(),
                            value_repr: s.clone(),
                            value_type: ValueType::String,
                        });
                    }
                    serde_json::Value::Number(n) => {
                        fields.push(NodeField {
                            key: k.clone(),
                            value_repr: n.to_string(),
                            value_type: ValueType::Number,
                        });
                    }
                    serde_json::Value::Bool(b) => {
                        fields.push(NodeField {
                            key: k.clone(),
                            value_repr: b.to_string(),
                            value_type: ValueType::Boolean,
                        });
                    }
                    serde_json::Value::Null => {
                        fields.push(NodeField {
                            key: k.clone(),
                            value_repr: "null".to_string(),
                            value_type: ValueType::Null,
                        });
                    }
                }
            }
            nodes.push(GraphNode {
                id: node_id.to_string(),
                title: title.to_string(),
                fields,
                is_expandable: true,
                is_collapsed: false,
            });
        }
        serde_json::Value::Array(arr) => {
            let mut fields = Vec::new();
            for (idx, item) in arr.iter().enumerate() {
                match item {
                    serde_json::Value::Object(_) | serde_json::Value::Array(_) => {
                        let child_id = format!("{}_{}", node_id, idx);
                        edges.push(GraphEdge {
                            from_node_id: node_id.to_string(),
                            to_node_id: child_id.clone(),
                            label: Some(format!("[{}]", idx)),
                        });
                        parse_value(item, &child_id, &format!("[{}]", idx), nodes, edges);
                    }
                    _ => {
                        fields.push(NodeField {
                            key: format!("[{}]", idx),
                            value_repr: item.to_string(),
                            value_type: match item {
                                serde_json::Value::String(_) => ValueType::String,
                                serde_json::Value::Number(_) => ValueType::Number,
                                serde_json::Value::Bool(_) => ValueType::Boolean,
                                _ => ValueType::Null,
                            },
                        });
                    }
                }
            }
            nodes.push(GraphNode {
                id: node_id.to_string(),
                title: title.to_string(),
                fields,
                is_expandable: true,
                is_collapsed: false,
            });
        }
        _ => {
            nodes.push(GraphNode {
                id: node_id.to_string(),
                title: title.to_string(),
                fields: vec![NodeField {
                    key: "value".to_string(),
                    value_repr: val.to_string(),
                    value_type: ValueType::String,
                }],
                is_expandable: false,
                is_collapsed: false,
            });
        }
    }
}
