use crate::graph::Graph;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutGraph {
    pub nodes: Vec<LayoutNode>,
    pub edges: Vec<LayoutEdge>,
    pub bounds: Rect,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutNode {
    pub id: String,
    pub rect: Rect,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutEdge {
    pub from: Point,
    pub to: Point,
    pub control_point1: Point,
    pub control_point2: Point,
}

pub fn compute_layout(graph: &Graph) -> LayoutGraph {
    let mut layout_nodes = Vec::new();
    let current_x = 20.0;
    let mut current_y = 20.0;
    let node_width = 240.0;
    let base_height = 40.0;
    let field_height = 20.0;

    for node in &graph.nodes {
        let h = base_height + (node.fields.len() as f32 * field_height);
        layout_nodes.push(LayoutNode {
            id: node.id.clone(),
            rect: Rect {
                x: current_x,
                y: current_y,
                width: node_width,
                height: h,
            },
        });
        current_y += h + 30.0;
    }

    LayoutGraph {
        nodes: layout_nodes,
        edges: vec![],
        bounds: Rect {
            x: 0.0,
            y: 0.0,
            width: current_x + node_width + 40.0,
            height: current_y + 40.0,
        },
    }
}
