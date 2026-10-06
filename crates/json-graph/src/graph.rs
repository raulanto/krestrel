use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::path::JsonPath;

pub const DEFAULT_MAX_NODES: usize = 10_000;
pub const DEFAULT_MAX_ROWS_PER_CARD: usize = 40;
pub const DEFAULT_MAX_STRING_LEN: usize = 120;
pub const DEFAULT_ARRAY_CHUNK_SIZE: usize = 100;
pub const DEFAULT_MAX_DEPTH: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrimitiveType {
    String,
    Number,
    Boolean,
    Null,
}

impl PrimitiveType {
    pub fn name(&self) -> &'static str {
        match self {
            Self::String => "string",
            Self::Number => "number",
            Self::Boolean => "bool",
            Self::Null => "null",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PrimitiveValue {
    String(String),
    Number(String),
    Boolean(bool),
    Null,
}

impl PrimitiveValue {
    pub fn display_text(&self) -> String {
        match self {
            Self::String(s) => {
                if s.len() > DEFAULT_MAX_STRING_LEN {
                    format!("\"{}…\"", &s[..DEFAULT_MAX_STRING_LEN.saturating_sub(1)])
                } else {
                    format!("\"{}\"", s)
                }
            }
            Self::Number(n) => n.clone(),
            Self::Boolean(b) => b.to_string(),
            Self::Null => "null".to_string(),
        }
    }

    pub fn raw_text(&self) -> String {
        match self {
            Self::String(s) => s.clone(),
            Self::Number(n) => n.clone(),
            Self::Boolean(b) => b.to_string(),
            Self::Null => "null".to_string(),
        }
    }

    pub fn primitive_type(&self) -> PrimitiveType {
        match self {
            Self::String(_) => PrimitiveType::String,
            Self::Number(_) => PrimitiveType::Number,
            Self::Boolean(_) => PrimitiveType::Boolean,
            Self::Null => PrimitiveType::Null,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PropertyRow {
    pub key: String,
    pub value: PrimitiveValue,
    pub path: JsonPath,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeKind {
    /// Object card listing primitive key-value pairs
    ObjectCard,
    /// Container node for an array (e.g. `members: array[3]`)
    ArrayContainer { count: usize },
    /// Leaf node representing a primitive element in an array
    PrimitiveLeaf,
    /// Chunk node grouping large arrays (e.g. `[0..99]`)
    ArrayChunk { start: usize, end: usize },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub path: JsonPath,
    pub label: String,
    pub kind: NodeKind,
    pub properties: Vec<PropertyRow>,
    pub children: Vec<String>,
    pub truncated_rows: usize,
    pub depth: usize,
}

impl GraphNode {
    pub fn is_collapsible(&self) -> bool {
        !self.children.is_empty()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Graph {
    pub root_id: String,
    pub nodes: HashMap<String, GraphNode>,
    pub total_nodes: usize,
    pub has_exceeded_limits: bool,
    pub warning_message: Option<String>,
}

pub struct GraphBuilder {
    pub max_nodes: usize,
    pub max_rows_per_card: usize,
    pub array_chunk_size: usize,
    pub max_depth: usize,
}

impl Default for GraphBuilder {
    fn default() -> Self {
        Self {
            max_nodes: DEFAULT_MAX_NODES,
            max_rows_per_card: DEFAULT_MAX_ROWS_PER_CARD,
            array_chunk_size: DEFAULT_ARRAY_CHUNK_SIZE,
            max_depth: DEFAULT_MAX_DEPTH,
        }
    }
}

impl GraphBuilder {
    pub fn build(&self, value: &Value) -> Graph {
        let mut graph = Graph::default();
        let root_path = JsonPath::root();
        let root_id = root_path.as_str().to_string();
        graph.root_id = root_id.clone();

        self.process_value(value, &root_path, "root", 0, &mut graph);
        graph.total_nodes = graph.nodes.len();

        graph
    }

    fn process_value(
        &self,
        value: &Value,
        path: &JsonPath,
        label: &str,
        depth: usize,
        graph: &mut Graph,
    ) -> String {
        let node_id = path.as_str().to_string();

        if graph.nodes.len() >= self.max_nodes {
            graph.has_exceeded_limits = true;
            graph.warning_message = Some(format!(
                "Límite de visualización alcanzado ({} nodos).",
                self.max_nodes
            ));
            let node = GraphNode {
                id: node_id.clone(),
                path: path.clone(),
                label: "… [límite]".to_string(),
                kind: NodeKind::PrimitiveLeaf,
                properties: vec![PropertyRow {
                    key: label.to_string(),
                    value: PrimitiveValue::String("… [límite alcanzado]".to_string()),
                    path: path.clone(),
                }],
                children: Vec::new(),
                truncated_rows: 0,
                depth,
            };
            graph.nodes.insert(node_id.clone(), node);
            return node_id;
        }

        if depth >= self.max_depth {
            graph.has_exceeded_limits = true;
            graph.warning_message = Some(format!(
                "Profundidad máxima alcanzada ({} niveles).",
                self.max_depth
            ));
            let node = GraphNode {
                id: node_id.clone(),
                path: path.clone(),
                label: "… [profundidad]".to_string(),
                kind: NodeKind::PrimitiveLeaf,
                properties: vec![PropertyRow {
                    key: label.to_string(),
                    value: PrimitiveValue::String("… [profundidad máxima]".to_string()),
                    path: path.clone(),
                }],
                children: Vec::new(),
                truncated_rows: 0,
                depth,
            };
            graph.nodes.insert(node_id.clone(), node);
            return node_id;
        }

        match value {
            Value::Object(map) => {
                let mut primitive_rows = Vec::new();
                let mut complex_entries = Vec::new();

                for (k, v) in map {
                    let child_path = path.child(k);
                    match v {
                        Value::Null => {
                            primitive_rows.push(PropertyRow {
                                key: k.clone(),
                                value: PrimitiveValue::Null,
                                path: child_path,
                            });
                        }
                        Value::Bool(b) => {
                            primitive_rows.push(PropertyRow {
                                key: k.clone(),
                                value: PrimitiveValue::Boolean(*b),
                                path: child_path,
                            });
                        }
                        Value::Number(n) => {
                            primitive_rows.push(PropertyRow {
                                key: k.clone(),
                                value: PrimitiveValue::Number(n.to_string()),
                                path: child_path,
                            });
                        }
                        Value::String(s) => {
                            primitive_rows.push(PropertyRow {
                                key: k.clone(),
                                value: PrimitiveValue::String(s.clone()),
                                path: child_path,
                            });
                        }
                        Value::Array(_) | Value::Object(_) => {
                            complex_entries.push((k.clone(), v, child_path));
                        }
                    }
                }

                let total_rows = primitive_rows.len();
                let truncated_rows = total_rows.saturating_sub(self.max_rows_per_card);
                if truncated_rows > 0 {
                    primitive_rows.truncate(self.max_rows_per_card);
                }

                let mut child_ids = Vec::new();
                for (k, v, child_path) in complex_entries {
                    let child_id = self.process_value(v, &child_path, &k, depth + 1, graph);
                    child_ids.push(child_id);
                }

                let node = GraphNode {
                    id: node_id.clone(),
                    path: path.clone(),
                    label: label.to_string(),
                    kind: NodeKind::ObjectCard,
                    properties: primitive_rows,
                    children: child_ids,
                    truncated_rows,
                    depth,
                };
                graph.nodes.insert(node_id.clone(), node);
                node_id
            }
            Value::Array(arr) => {
                let count = arr.len();
                let mut child_ids = Vec::new();

                if count > self.array_chunk_size {
                    // Chunk large arrays
                    for (chunk_idx, chunk) in arr.chunks(self.array_chunk_size).enumerate() {
                        let start = chunk_idx * self.array_chunk_size;
                        let end = (start + chunk.len()).saturating_sub(1);
                        let chunk_path = path.range(start, end);
                        let chunk_id = chunk_path.as_str().to_string();
                        let mut chunk_child_ids = Vec::new();

                        for (offset, item) in chunk.iter().enumerate() {
                            let item_idx = start + offset;
                            let item_path = path.index(item_idx);
                            let item_label = format!("[{}]", item_idx);
                            let item_id =
                                self.process_value(item, &item_path, &item_label, depth + 2, graph);
                            chunk_child_ids.push(item_id);
                        }

                        let chunk_node = GraphNode {
                            id: chunk_id.clone(),
                            path: chunk_path,
                            label: format!("[{} … {}]", start, end),
                            kind: NodeKind::ArrayChunk { start, end },
                            properties: Vec::new(),
                            children: chunk_child_ids,
                            truncated_rows: 0,
                            depth: depth + 1,
                        };
                        graph.nodes.insert(chunk_id.clone(), chunk_node);
                        child_ids.push(chunk_id);
                    }
                } else {
                    for (idx, item) in arr.iter().enumerate() {
                        let item_path = path.index(idx);
                        let item_label = format!("[{}]", idx);
                        let item_id =
                            self.process_value(item, &item_path, &item_label, depth + 1, graph);
                        child_ids.push(item_id);
                    }
                }

                let node = GraphNode {
                    id: node_id.clone(),
                    path: path.clone(),
                    label: if label == "root" {
                        format!("Array [{}]", count)
                    } else {
                        label.to_string()
                    },
                    kind: NodeKind::ArrayContainer { count },
                    properties: Vec::new(),
                    children: child_ids,
                    truncated_rows: 0,
                    depth,
                };
                graph.nodes.insert(node_id.clone(), node);
                node_id
            }
            Value::Null => {
                let node = GraphNode {
                    id: node_id.clone(),
                    path: path.clone(),
                    label: label.to_string(),
                    kind: NodeKind::PrimitiveLeaf,
                    properties: vec![PropertyRow {
                        key: label.to_string(),
                        value: PrimitiveValue::Null,
                        path: path.clone(),
                    }],
                    children: Vec::new(),
                    truncated_rows: 0,
                    depth,
                };
                graph.nodes.insert(node_id.clone(), node);
                node_id
            }
            Value::Bool(b) => {
                let node = GraphNode {
                    id: node_id.clone(),
                    path: path.clone(),
                    label: label.to_string(),
                    kind: NodeKind::PrimitiveLeaf,
                    properties: vec![PropertyRow {
                        key: label.to_string(),
                        value: PrimitiveValue::Boolean(*b),
                        path: path.clone(),
                    }],
                    children: Vec::new(),
                    truncated_rows: 0,
                    depth,
                };
                graph.nodes.insert(node_id.clone(), node);
                node_id
            }
            Value::Number(n) => {
                let node = GraphNode {
                    id: node_id.clone(),
                    path: path.clone(),
                    label: label.to_string(),
                    kind: NodeKind::PrimitiveLeaf,
                    properties: vec![PropertyRow {
                        key: label.to_string(),
                        value: PrimitiveValue::Number(n.to_string()),
                        path: path.clone(),
                    }],
                    children: Vec::new(),
                    truncated_rows: 0,
                    depth,
                };
                graph.nodes.insert(node_id.clone(), node);
                node_id
            }
            Value::String(s) => {
                let node = GraphNode {
                    id: node_id.clone(),
                    path: path.clone(),
                    label: label.to_string(),
                    kind: NodeKind::PrimitiveLeaf,
                    properties: vec![PropertyRow {
                        key: label.to_string(),
                        value: PrimitiveValue::String(s.clone()),
                        path: path.clone(),
                    }],
                    children: Vec::new(),
                    truncated_rows: 0,
                    depth,
                };
                graph.nodes.insert(node_id.clone(), node);
                node_id
            }
        }
    }
}

impl Graph {
    pub fn from_json(value: &Value) -> Self {
        GraphBuilder::default().build(value)
    }

    pub fn node(&self, id: &str) -> Option<&GraphNode> {
        self.nodes.get(id)
    }

    pub fn root(&self) -> Option<&GraphNode> {
        self.nodes.get(&self.root_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reference_superheroes_json() {
        let json_str = r#"{
            "squadName": "Super hero squad",
            "homeTown": "Metro City",
            "formed": 2016,
            "secretBase": "Super tower",
            "active": true,
            "members": [
                {
                    "name": "Molecule Man",
                    "age": 29,
                    "secretIdentity": "Dan Jukes",
                    "powers": [
                        "Radiation resistance",
                        "Turning tiny",
                        "Radiation blast"
                    ]
                },
                {
                    "name": "Madame Uppercut",
                    "age": 39,
                    "secretIdentity": "Jane Wilson",
                    "powers": [
                        "Million tonne punch",
                        "Damage resistance",
                        "Superhuman reflexes"
                    ]
                }
            ]
        }"#;

        let val: Value = serde_json::from_str(json_str).unwrap();
        let graph = Graph::from_json(&val);

        let root = graph.root().expect("Root node should exist");
        assert_eq!(root.kind, NodeKind::ObjectCard);
        assert_eq!(root.properties.len(), 5); // squadName, homeTown, formed, secretBase, active
        assert_eq!(root.children.len(), 1); // members

        let members_id = &root.children[0];
        let members_node = graph.node(members_id).expect("Members node should exist");
        assert_eq!(members_node.kind, NodeKind::ArrayContainer { count: 2 });
        assert_eq!(members_node.children.len(), 2);

        // Member 0
        let member0 = graph
            .node(&members_node.children[0])
            .expect("Member 0 should exist");
        assert_eq!(member0.kind, NodeKind::ObjectCard);
        assert_eq!(member0.properties.len(), 3); // name, age, secretIdentity
        assert_eq!(member0.children.len(), 1); // powers

        // Powers of Member 0
        let powers0 = graph
            .node(&member0.children[0])
            .expect("Powers 0 should exist");
        assert_eq!(powers0.kind, NodeKind::ArrayContainer { count: 3 });
        assert_eq!(powers0.children.len(), 3); // 3 leaf nodes
    }

    #[test]
    fn test_primitive_roots() {
        let val = Value::String("plain string".to_string());
        let graph = Graph::from_json(&val);
        let root = graph.root().unwrap();
        assert_eq!(root.kind, NodeKind::PrimitiveLeaf);

        let val = Value::Null;
        let graph = Graph::from_json(&val);
        assert_eq!(graph.root().unwrap().kind, NodeKind::PrimitiveLeaf);

        let val = Value::Array(vec![Value::Number(1.into()), Value::Number(2.into())]);
        let graph = Graph::from_json(&val);
        assert_eq!(
            graph.root().unwrap().kind,
            NodeKind::ArrayContainer { count: 2 }
        );
    }

    #[test]
    fn test_large_array_chunking() {
        let items: Vec<Value> = (0..250).map(|i| Value::Number(i.into())).collect();
        let val = Value::Array(items);
        let graph = Graph::from_json(&val);

        let root = graph.root().unwrap();
        // 250 items with chunk size 100 -> 3 chunks
        assert_eq!(root.children.len(), 3);
        let chunk0 = graph.node(&root.children[0]).unwrap();
        assert!(matches!(
            chunk0.kind,
            NodeKind::ArrayChunk { start: 0, end: 99 }
        ));
        assert_eq!(chunk0.children.len(), 100);
    }
}
