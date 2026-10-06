use kestrel_json_graph::{
    ApproximateMeasurer, Graph, GraphBuilder, Layout, LayoutEngine, LayoutOptions,
};
use serde_json::Value;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DiagramViewMode {
    #[default]
    Types,
    Data,
}

#[derive(Debug, Clone)]
pub struct DiagramState {
    pub pan_x: f32,
    pub pan_y: f32,
    pub zoom: f32,
    pub view_mode: DiagramViewMode,
    pub is_dragging: bool,
    pub drag_start_mouse: Option<(f32, f32)>,
    pub drag_start_pan: (f32, f32),
    pub collapsed_ids: HashSet<String>,
    pub selected_node_id: Option<String>,
    pub graph: Option<Graph>,
    pub layout: Option<Layout>,
    pub raw_json: String,
    pub error_message: Option<String>,
}

impl Default for DiagramState {
    fn default() -> Self {
        Self {
            pan_x: 40.0,
            pan_y: 40.0,
            zoom: 1.0,
            view_mode: DiagramViewMode::Types,
            is_dragging: false,
            drag_start_mouse: None,
            drag_start_pan: (40.0, 40.0),
            collapsed_ids: HashSet::new(),
            selected_node_id: None,
            graph: None,
            layout: None,
            raw_json: String::new(),
            error_message: None,
        }
    }
}

impl DiagramState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_json(&mut self, json_str: &str) {
        if json_str.trim().is_empty() {
            self.raw_json.clear();
            self.graph = None;
            self.layout = None;
            self.error_message = Some("El cuerpo de la respuesta está vacío".into());
            return;
        }

        self.raw_json = json_str.to_string();
        match serde_json::from_str::<Value>(json_str) {
            Ok(value) => {
                let builder = GraphBuilder::default();
                let graph = builder.build(&value);
                self.graph = Some(graph);
                self.error_message = None;
                self.recompute_layout();
            }
            Err(err) => {
                self.graph = None;
                self.layout = None;
                self.error_message = Some(format!("No es un JSON válido: {}", err));
            }
        }
    }

    pub fn recompute_layout(&mut self) {
        let Some(ref graph) = self.graph else {
            self.layout = None;
            return;
        };

        let measurer = ApproximateMeasurer::default();
        let options = LayoutOptions::default();
        let engine = LayoutEngine::new(graph, &self.collapsed_ids, &measurer, options);
        let layout = engine.calculate();
        self.layout = Some(layout);
    }

    pub fn toggle_collapse(&mut self, node_id: &str) {
        if self.collapsed_ids.contains(node_id) {
            self.collapsed_ids.remove(node_id);
        } else {
            self.collapsed_ids.insert(node_id.to_string());
        }
        self.recompute_layout();
    }

    pub fn collapse_all(&mut self) {
        if let Some(ref graph) = self.graph {
            for (id, node) in &graph.nodes {
                if node.is_collapsible() {
                    self.collapsed_ids.insert(id.clone());
                }
            }
            self.recompute_layout();
        }
    }

    pub fn expand_all(&mut self) {
        self.collapsed_ids.clear();
        self.recompute_layout();
    }

    pub fn select_node(&mut self, node_id: Option<String>) {
        self.selected_node_id = node_id;
    }

    pub fn zoom_in(&mut self) {
        self.zoom = (self.zoom * 1.2).clamp(0.2, 3.0);
    }

    pub fn zoom_out(&mut self) {
        self.zoom = (self.zoom / 1.2).clamp(0.2, 3.0);
    }

    pub fn reset_zoom(&mut self) {
        self.zoom = 1.0;
        self.pan_x = 40.0;
        self.pan_y = 40.0;
    }

    pub fn zoom_at(&mut self, factor: f32, cursor_x: f32, cursor_y: f32) {
        let old_zoom = self.zoom;
        let new_zoom = (old_zoom * factor).clamp(0.2, 3.0);
        if (new_zoom - old_zoom).abs() < f32::EPSILON {
            return;
        }

        self.pan_x = cursor_x - (cursor_x - self.pan_x) * (new_zoom / old_zoom);
        self.pan_y = cursor_y - (cursor_y - self.pan_y) * (new_zoom / old_zoom);
        self.zoom = new_zoom;
    }

    pub fn fit_view(&mut self, viewport_width: f32, viewport_height: f32) {
        let Some(ref layout) = self.layout else {
            return;
        };
        if layout.bounds.width <= 0.0 || layout.bounds.height <= 0.0 {
            return;
        }

        let padding = 40.0;
        let available_w = (viewport_width - padding * 2.0).max(100.0);
        let available_h = (viewport_height - padding * 2.0).max(100.0);

        let scale_x = available_w / layout.bounds.width;
        let scale_y = available_h / layout.bounds.height;
        let fit_scale = scale_x.min(scale_y).clamp(0.2, 1.5);

        self.zoom = fit_scale;
        self.pan_x = padding;
        self.pan_y = padding;
    }

    pub fn start_drag(&mut self, mouse_x: f32, mouse_y: f32) {
        self.is_dragging = true;
        self.drag_start_mouse = Some((mouse_x, mouse_y));
        self.drag_start_pan = (self.pan_x, self.pan_y);
    }

    pub fn update_drag(&mut self, mouse_x: f32, mouse_y: f32) {
        if !self.is_dragging {
            return;
        }
        if let Some((start_x, start_y)) = self.drag_start_mouse {
            let dx = mouse_x - start_x;
            let dy = mouse_y - start_y;
            self.pan_x = self.drag_start_pan.0 + dx;
            self.pan_y = self.drag_start_pan.1 + dy;
        }
    }

    pub fn end_drag(&mut self) {
        self.is_dragging = false;
        self.drag_start_mouse = None;
    }

    pub fn set_view_mode(&mut self, mode: DiagramViewMode) {
        self.view_mode = mode;
    }

    pub fn toggle_view_mode(&mut self) {
        self.view_mode = match self.view_mode {
            DiagramViewMode::Types => DiagramViewMode::Data,
            DiagramViewMode::Data => DiagramViewMode::Types,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagram_state_view_mode() {
        let mut state = DiagramState::new();
        assert_eq!(state.view_mode, DiagramViewMode::Types);

        state.toggle_view_mode();
        assert_eq!(state.view_mode, DiagramViewMode::Data);

        state.set_view_mode(DiagramViewMode::Types);
        assert_eq!(state.view_mode, DiagramViewMode::Types);
    }
}
