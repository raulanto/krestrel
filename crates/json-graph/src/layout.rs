use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::graph::{Graph, GraphNode, NodeKind};
use crate::measure::TextMeasurer;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn max_x(&self) -> f32 {
        self.x + self.width
    }

    pub fn max_y(&self) -> f32 {
        self.y + self.height
    }

    pub fn contains_point(&self, p: Point) -> bool {
        p.x >= self.x && p.x <= self.max_x() && p.y >= self.y && p.y <= self.max_y()
    }

    pub fn intersects(&self, other: &Rect) -> bool {
        self.x < other.max_x()
            && self.max_x() > other.x
            && self.y < other.max_y()
            && self.max_y() > other.y
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutNode {
    pub id: String,
    pub bounds: Rect,
    pub depth: usize,
    pub is_collapsed: bool,
    pub has_children: bool,
    pub visible_children_count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutEdge {
    pub source_id: String,
    pub target_id: String,
    pub start: Point,
    pub end: Point,
    pub control1: Point,
    pub control2: Point,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Layout {
    pub nodes: HashMap<String, LayoutNode>,
    pub edges: Vec<LayoutEdge>,
    pub bounds: Rect,
}

#[derive(Debug, Clone, Copy)]
pub struct LayoutOptions {
    pub col_spacing: f32,
    pub row_spacing: f32,
    pub card_padding: f32,
    pub font_size_header: f32,
    pub font_size_row: f32,
    pub header_height: f32,
    pub row_height: f32,
    pub min_card_width: f32,
    pub max_card_width: f32,
}

impl Default for LayoutOptions {
    fn default() -> Self {
        Self {
            col_spacing: 64.0,
            row_spacing: 20.0,
            card_padding: 12.0,
            font_size_header: 13.0,
            font_size_row: 12.0,
            header_height: 32.0,
            row_height: 22.0,
            min_card_width: 170.0,
            max_card_width: 380.0,
        }
    }
}

struct NodeMetrics {
    width: f32,
    height: f32,
    subtree_height: f32,
}

pub struct LayoutEngine<'a> {
    graph: &'a Graph,
    collapsed: &'a HashSet<String>,
    measurer: &'a dyn TextMeasurer,
    options: LayoutOptions,
}

impl<'a> LayoutEngine<'a> {
    pub fn new(
        graph: &'a Graph,
        collapsed: &'a HashSet<String>,
        measurer: &'a dyn TextMeasurer,
        options: LayoutOptions,
    ) -> Self {
        Self {
            graph,
            collapsed,
            measurer,
            options,
        }
    }

    pub fn calculate(&self) -> Layout {
        let Some(root) = self.graph.root() else {
            return Layout {
                nodes: HashMap::new(),
                edges: Vec::new(),
                bounds: Rect::new(0.0, 0.0, 0.0, 0.0),
            };
        };

        // Phase 1: Compute dimensions and subtree heights bottom-up
        let mut metrics = HashMap::new();
        self.compute_metrics(&root.id, &mut metrics);

        // Phase 2: Compute max column widths
        let mut col_widths: HashMap<usize, f32> = HashMap::new();
        let mut col_offsets: HashMap<usize, f32> = HashMap::new();
        self.collect_column_widths(&root.id, 0, &metrics, &mut col_widths);

        let max_depth = col_widths.keys().max().copied().unwrap_or(0);
        let mut current_x = 24.0; // Margin left
        for d in 0..=max_depth {
            col_offsets.insert(d, current_x);
            let w = col_widths
                .get(&d)
                .copied()
                .unwrap_or(self.options.min_card_width);
            current_x += w + self.options.col_spacing;
        }

        // Phase 3: Place nodes top-down (left-to-right tree positioning)
        let mut layout_nodes = HashMap::new();
        let start_y = 24.0; // Margin top
        self.place_nodes(
            &root.id,
            0,
            start_y,
            &col_offsets,
            &metrics,
            &mut layout_nodes,
        );

        // Phase 4: Generate connecting edges
        let mut edges = Vec::new();
        self.generate_edges(&root.id, &layout_nodes, &mut edges);

        // Calculate total bounding box
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;

        for node in layout_nodes.values() {
            min_x = min_x.min(node.bounds.x);
            min_y = min_y.min(node.bounds.y);
            max_x = max_x.max(node.bounds.max_x());
            max_y = max_y.max(node.bounds.max_y());
        }

        let bounds = if layout_nodes.is_empty() {
            Rect::new(0.0, 0.0, 0.0, 0.0)
        } else {
            Rect::new(
                min_x,
                min_y,
                (max_x - min_x).max(100.0),
                (max_y - min_y).max(100.0),
            )
        };

        Layout {
            nodes: layout_nodes,
            edges,
            bounds,
        }
    }

    fn compute_metrics(&self, node_id: &str, out: &mut HashMap<String, NodeMetrics>) -> (f32, f32) {
        let Some(node) = self.graph.node(node_id) else {
            return (0.0, 0.0);
        };

        let (width, height) = self.measure_node(node);
        let is_collapsed = self.collapsed.contains(node_id);

        let subtree_height = if is_collapsed || node.children.is_empty() {
            height
        } else {
            let mut children_total_height = 0.0;
            let mut visible_children = 0;

            for child_id in &node.children {
                let (_, child_sub_h) = self.compute_metrics(child_id, out);
                children_total_height += child_sub_h;
                visible_children += 1;
            }

            if visible_children > 1 {
                children_total_height += (visible_children - 1) as f32 * self.options.row_spacing;
            }

            height.max(children_total_height)
        };

        out.insert(
            node_id.to_string(),
            NodeMetrics {
                width,
                height,
                subtree_height,
            },
        );

        (width, subtree_height)
    }

    fn measure_node(&self, node: &GraphNode) -> (f32, f32) {
        let opt = &self.options;
        match node.kind {
            NodeKind::ObjectCard => {
                let (header_w, _) = self
                    .measurer
                    .measure_text(&node.label, opt.font_size_header);
                let mut max_content_w = header_w + 40.0; // room for icon/toggle

                for prop in &node.properties {
                    let text = format!("{}: {}", prop.key, prop.value.display_text());
                    let (row_w, _) = self.measurer.measure_text(&text, opt.font_size_row);
                    max_content_w = max_content_w.max(row_w + opt.card_padding * 2.0);
                }

                let width = max_content_w.clamp(opt.min_card_width, opt.max_card_width);
                let rows_count =
                    node.properties.len() + if node.truncated_rows > 0 { 1 } else { 0 };
                let height =
                    opt.header_height + (rows_count as f32 * opt.row_height) + opt.card_padding;

                (width, height)
            }
            NodeKind::ArrayContainer { .. } => {
                let (label_w, _) = self
                    .measurer
                    .measure_text(&node.label, opt.font_size_header);
                let width = (label_w + 56.0).clamp(opt.min_card_width * 0.7, opt.max_card_width);
                let height = opt.header_height + 4.0;
                (width, height)
            }
            NodeKind::ArrayChunk { .. } => {
                let (label_w, _) = self
                    .measurer
                    .measure_text(&node.label, opt.font_size_header);
                let width = (label_w + 48.0).clamp(opt.min_card_width * 0.6, opt.max_card_width);
                let height = opt.header_height;
                (width, height)
            }
            NodeKind::PrimitiveLeaf => {
                let text = node
                    .properties
                    .first()
                    .map(|p| p.value.display_text())
                    .unwrap_or_else(|| node.label.clone());
                let (text_w, _) = self.measurer.measure_text(&text, opt.font_size_row);
                let width = (text_w + 32.0).clamp(opt.min_card_width * 0.6, opt.max_card_width);
                let height = opt.header_height;
                (width, height)
            }
        }
    }

    fn collect_column_widths(
        &self,
        node_id: &str,
        depth: usize,
        metrics: &HashMap<String, NodeMetrics>,
        col_widths: &mut HashMap<usize, f32>,
    ) {
        let Some(m) = metrics.get(node_id) else {
            return;
        };

        let current_max = col_widths.entry(depth).or_insert(0.0);
        *current_max = current_max.max(m.width);

        if !self.collapsed.contains(node_id) {
            let Some(node) = self.graph.node(node_id) else {
                return;
            };
            for child_id in &node.children {
                self.collect_column_widths(child_id, depth + 1, metrics, col_widths);
            }
        }
    }

    fn place_nodes(
        &self,
        node_id: &str,
        depth: usize,
        y_top: f32,
        col_offsets: &HashMap<usize, f32>,
        metrics: &HashMap<String, NodeMetrics>,
        out: &mut HashMap<String, LayoutNode>,
    ) -> Rect {
        let node = self.graph.node(node_id).unwrap();
        let m = metrics.get(node_id).unwrap();
        let x = *col_offsets.get(&depth).unwrap_or(&24.0);
        let is_collapsed = self.collapsed.contains(node_id);

        let (node_y, bounds) = if is_collapsed || node.children.is_empty() {
            let bounds = Rect::new(x, y_top, m.width, m.height);
            (y_top, bounds)
        } else {
            let mut current_child_y = y_top;
            let mut first_child_y = None;
            let mut last_child_bottom = y_top + m.height;

            for child_id in &node.children {
                let child_m = metrics.get(child_id).unwrap();
                let child_bounds = self.place_nodes(
                    child_id,
                    depth + 1,
                    current_child_y,
                    col_offsets,
                    metrics,
                    out,
                );

                if first_child_y.is_none() {
                    first_child_y = Some(child_bounds.y);
                }
                last_child_bottom = child_bounds.max_y();

                current_child_y += child_m.subtree_height + self.options.row_spacing;
            }

            let first_y = first_child_y.unwrap_or(y_top);
            let span_center = (first_y + last_child_bottom) / 2.0;
            let parent_y = (span_center - m.height / 2.0).max(y_top);

            let bounds = Rect::new(x, parent_y, m.width, m.height);
            (parent_y, bounds)
        };

        out.insert(
            node_id.to_string(),
            LayoutNode {
                id: node_id.to_string(),
                bounds,
                depth,
                is_collapsed,
                has_children: !node.children.is_empty(),
                visible_children_count: if is_collapsed { 0 } else { node.children.len() },
            },
        );

        Rect::new(x, node_y, m.width, m.subtree_height)
    }

    fn generate_edges(
        &self,
        node_id: &str,
        layout_nodes: &HashMap<String, LayoutNode>,
        out: &mut Vec<LayoutEdge>,
    ) {
        if self.collapsed.contains(node_id) {
            return;
        }

        let Some(parent_layout) = layout_nodes.get(node_id) else {
            return;
        };
        let Some(node) = self.graph.node(node_id) else {
            return;
        };

        let start = Point::new(
            parent_layout.bounds.max_x(),
            parent_layout.bounds.y + parent_layout.bounds.height / 2.0,
        );

        for child_id in &node.children {
            if let Some(child_layout) = layout_nodes.get(child_id) {
                let end = Point::new(
                    child_layout.bounds.x,
                    child_layout.bounds.y + child_layout.bounds.height / 2.0,
                );

                let dx = (end.x - start.x).abs() * 0.5;
                let control1 = Point::new(start.x + dx, start.y);
                let control2 = Point::new(end.x - dx, end.y);

                out.push(LayoutEdge {
                    source_id: node_id.to_string(),
                    target_id: child_id.clone(),
                    start,
                    end,
                    control1,
                    control2,
                });

                self.generate_edges(child_id, layout_nodes, out);
            }
        }
    }
}

pub fn calculate_layout(
    graph: &Graph,
    collapsed: &HashSet<String>,
    measurer: &dyn TextMeasurer,
    options: LayoutOptions,
) -> Layout {
    LayoutEngine::new(graph, collapsed, measurer, options).calculate()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::measure::ApproximateMeasurer;

    #[test]
    fn test_layout_calculation_simple() {
        let json_str = r#"{
            "title": "Kestrel",
            "version": 1,
            "metadata": {
                "author": "Raul",
                "tag": "alpha"
            }
        }"#;

        let val = serde_json::from_str(json_str).unwrap();
        let graph = Graph::from_json(&val);
        let collapsed = HashSet::new();
        let measurer = ApproximateMeasurer::new();
        let layout = calculate_layout(&graph, &collapsed, &measurer, LayoutOptions::default());

        assert_eq!(layout.nodes.len(), 2); // root and metadata
        assert_eq!(layout.edges.len(), 1); // edge from root to metadata

        let root_layout = layout.nodes.get(&graph.root_id).unwrap();
        let meta_layout = layout.nodes.get("$.metadata").unwrap();

        // Left-to-right check
        assert!(root_layout.bounds.max_x() < meta_layout.bounds.x);
        // Bounding box spans both
        assert_eq!(layout.bounds.max_x(), meta_layout.bounds.max_x());
    }

    #[test]
    fn test_layout_collapsing() {
        let json_str = r#"{
            "items": [1, 2, 3]
        }"#;
        let val = serde_json::from_str(json_str).unwrap();
        let graph = Graph::from_json(&val);

        let mut collapsed = HashSet::new();
        collapsed.insert("$.items".to_string());

        let measurer = ApproximateMeasurer::new();
        let layout = calculate_layout(&graph, &collapsed, &measurer, LayoutOptions::default());

        // items children should not be in layout
        assert!(layout.nodes.contains_key("$"));
        assert!(layout.nodes.contains_key("$.items"));
        assert!(!layout.nodes.contains_key("$.items[0]"));
    }
}
