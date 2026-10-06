use ely_gpui_component::primitives::{Icon, IconName};
use gpui::{
    App, Bounds, Context, CursorStyle, FontWeight, InteractiveElement as _, IntoElement,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement as _, PathBuilder,
    Pixels, Render, ScrollWheelEvent, Styled, Window, canvas, div, point, px,
};
use kestrel_json_graph::{
    DisplayMode, ExportError, Rect, SvgTheme, generate_svg_with_mode, render_svg_to_png,
};

use super::bottom_bar::DiagramBottomBar;
use super::node_card::NodeCardView;
use super::state::{DiagramState, DiagramViewMode};
use super::toolbar::{DiagramToolbar, DiagramToolbarActions};
use crate::theme::{ThemeExt as _, v_flex};

pub struct DiagramView {
    state: DiagramState,
}

impl DiagramView {
    pub fn new(json_str: &str, _window: &mut Window, _cx: &mut Context<Self>) -> Self {
        let mut state = DiagramState::new();
        state.set_json(json_str);
        Self { state }
    }

    pub fn set_json(&mut self, json_str: &str, cx: &mut Context<Self>) {
        self.state.set_json(json_str);
        cx.notify();
    }

    pub fn state(&self) -> &DiagramState {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut DiagramState {
        &mut self.state
    }

    pub fn export_svg(&self, is_dark: bool) -> Option<String> {
        let graph = self.state.graph.as_ref()?;
        let layout = self.state.layout.as_ref()?;
        let theme = if is_dark {
            SvgTheme::dark()
        } else {
            SvgTheme::light()
        };
        let mode = match self.state.view_mode {
            DiagramViewMode::Types => DisplayMode::Types,
            DiagramViewMode::Data => DisplayMode::Values,
        };
        Some(generate_svg_with_mode(layout, graph, &theme, mode))
    }

    pub fn export_png(&self, is_dark: bool, scale: f32) -> Result<Vec<u8>, ExportError> {
        let svg = self
            .export_svg(is_dark)
            .ok_or_else(|| ExportError::SvgParseError("No hay diagrama para exportar".into()))?;
        render_svg_to_png(&svg, scale)
    }
}

impl Render for DiagramView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;

        if let Some(ref err) = self.state.error_message {
            return v_flex()
                .size_full()
                .items_center()
                .justify_center()
                .gap_3()
                .p_8()
                .child(Icon::new(IconName::Globe).color(colors.danger))
                .child(
                    div()
                        .font_weight(FontWeight::MEDIUM)
                        .text_sm()
                        .text_color(colors.danger)
                        .child(err.clone()),
                )
                .into_any_element();
        }

        let Some(ref graph) = self.state.graph else {
            return v_flex()
                .size_full()
                .items_center()
                .justify_center()
                .p_8()
                .child(
                    div()
                        .text_sm()
                        .text_color(colors.fg_muted)
                        .child("Sin datos de diagrama"),
                )
                .into_any_element();
        };

        let Some(ref layout) = self.state.layout else {
            return v_flex()
                .size_full()
                .items_center()
                .justify_center()
                .p_8()
                .child(
                    div()
                        .text_sm()
                        .text_color(colors.fg_muted)
                        .child("Calculando layout del diagrama…"),
                )
                .into_any_element();
        };

        let pan_x = self.state.pan_x;
        let pan_y = self.state.pan_y;
        let zoom = self.state.zoom;
        let view_mode = self.state.view_mode;
        let selected_id = self.state.selected_node_id.clone();
        let total_nodes = graph.total_nodes;

        // Collect visible nodes with culling
        // Calculate viewport in world coordinates
        let world_vp = Rect::new(
            -pan_x / zoom - 200.0,
            -pan_y / zoom - 200.0,
            2000.0 / zoom,
            2000.0 / zoom,
        );

        let mut visible_node_views = Vec::new();
        let mut visible_count = 0;

        for (node_id, layout_node) in &layout.nodes {
            if world_vp.intersects(&layout_node.bounds) {
                visible_count += 1;
                if let Some(graph_node) = graph.nodes.get(node_id) {
                    let is_selected = selected_id.as_deref() == Some(node_id.as_str());
                    let node_id_clone1 = node_id.clone();
                    let node_id_clone2 = node_id.clone();

                    let card = NodeCardView::new(
                        graph_node.clone(),
                        layout_node.clone(),
                        is_selected,
                        view_mode,
                        cx.listener(move |this, _ev, _window, cx| {
                            this.state.select_node(Some(node_id_clone1.clone()));
                            cx.notify();
                        }),
                        cx.listener(move |this, _ev, _window, cx| {
                            this.state.toggle_collapse(&node_id_clone2);
                            cx.notify();
                        }),
                    );

                    let screen_x = layout_node.bounds.x * zoom + pan_x;
                    let screen_y = layout_node.bounds.y * zoom + pan_y;

                    visible_node_views.push(
                        div()
                            .id(format!("placed-node-{}", node_id))
                            .absolute()
                            .left(px(screen_x))
                            .top(px(screen_y))
                            .child(card),
                    );
                }
            }
        }

        let selected_path = selected_id
            .as_ref()
            .and_then(|id| graph.nodes.get(id))
            .map(|n| n.path.to_string());

        let edges_clone = layout.edges.clone();
        let edge_stroke_color = colors.fg_muted.opacity(0.4);

        v_flex()
            .id("diagram-view-root")
            .size_full()
            .relative()
            .overflow_hidden()
            .bg(colors.bg)
            // Floating Top Toolbar
            .child(
                div()
                    .absolute()
                    .top(px(12.))
                    .left(px(12.))
                    .child(DiagramToolbar::new(
                        zoom,
                        view_mode,
                        DiagramToolbarActions {
                            on_select_types: cx.listener(|this, _, _window, cx| {
                                this.state.set_view_mode(DiagramViewMode::Types);
                                cx.notify();
                            }),
                            on_select_data: cx.listener(|this, _, _window, cx| {
                                this.state.set_view_mode(DiagramViewMode::Data);
                                cx.notify();
                            }),
                            on_zoom_in: cx.listener(|this, _, _window, cx| {
                                this.state.zoom_in();
                                cx.notify();
                            }),
                            on_zoom_out: cx.listener(|this, _, _window, cx| {
                                this.state.zoom_out();
                                cx.notify();
                            }),
                            on_reset_zoom: cx.listener(|this, _, _window, cx| {
                                this.state.reset_zoom();
                                cx.notify();
                            }),
                            on_fit_view: cx.listener(|this, _, _window, cx| {
                                this.state.fit_view(1000.0, 700.0);
                                cx.notify();
                            }),
                            on_expand_all: cx.listener(|this, _, _window, cx| {
                                this.state.expand_all();
                                cx.notify();
                            }),
                            on_collapse_all: cx.listener(|this, _, _window, cx| {
                                this.state.collapse_all();
                                cx.notify();
                            }),
                            on_export_svg: cx.listener(|this, _, _window, cx| {
                                if let Some(svg) = this.export_svg(true) {
                                    cx.write_to_clipboard(gpui::ClipboardItem::new_string(svg));
                                }
                            }),
                            on_export_png: cx.listener(|this, _, _window, cx| {
                                if let Ok(png) = this.export_png(true, 2.0) {
                                    let b64 = format!("PNG export: {} bytes", png.len());
                                    cx.write_to_clipboard(gpui::ClipboardItem::new_string(b64));
                                }
                            }),
                        },
                    )),
            )
            // Interactive Canvas Area
            .child(
                div()
                    .id("diagram-canvas-viewport")
                    .flex_1()
                    .size_full()
                    .relative()
                    .overflow_hidden()
                    .cursor(CursorStyle::PointingHand)
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, ev: &MouseDownEvent, _window, cx| {
                            let mx = f32::from(ev.position.x);
                            let my = f32::from(ev.position.y);
                            this.state.start_drag(mx, my);
                            cx.notify();
                        }),
                    )
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(|this, _ev: &MouseUpEvent, _window, cx| {
                            this.state.end_drag();
                            cx.notify();
                        }),
                    )
                    .on_mouse_move(cx.listener(|this, ev: &MouseMoveEvent, _window, cx| {
                        if this.state.is_dragging {
                            let mx = f32::from(ev.position.x);
                            let my = f32::from(ev.position.y);
                            this.state.update_drag(mx, my);
                            cx.notify();
                        }
                    }))
                    .on_scroll_wheel(cx.listener(|this, ev: &ScrollWheelEvent, _window, cx| {
                        let delta = f32::from(ev.delta.pixel_delta(px(16.0)).y);
                        let factor = if delta > 0.0 { 1.1 } else { 0.9 };
                        let mx = f32::from(ev.position.x);
                        let my = f32::from(ev.position.y);
                        this.state.zoom_at(factor, mx, my);
                        cx.notify();
                    }))
                    // Canvas paint for curved Bézier edges
                    .child(
                        canvas(
                            move |_bounds: Bounds<Pixels>, _window: &mut Window, _cx: &mut App| {},
                            move |_bounds, _layout_state, window, _cx| {
                                let t = (1.5 * zoom.clamp(0.8, 2.0)).max(1.0);
                                for edge in &edges_clone {
                                    let x0 = edge.start.x * zoom + pan_x;
                                    let y0 = edge.start.y * zoom + pan_y;
                                    let x1 = edge.control1.x * zoom + pan_x;
                                    let y1 = edge.control1.y * zoom + pan_y;
                                    let x2 = edge.control2.x * zoom + pan_x;
                                    let y2 = edge.control2.y * zoom + pan_y;
                                    let x3 = edge.end.x * zoom + pan_x;
                                    let y3 = edge.end.y * zoom + pan_y;

                                    let mut builder = PathBuilder::fill();
                                    builder.move_to(point(px(x0), px(y0 - t * 0.5)));
                                    builder.cubic_bezier_to(
                                        point(px(x1), px(y1 - t * 0.5)),
                                        point(px(x2), px(y2 - t * 0.5)),
                                        point(px(x3), px(y3 - t * 0.5)),
                                    );
                                    builder.line_to(point(px(x3), px(y3 + t * 0.5)));
                                    builder.cubic_bezier_to(
                                        point(px(x2), px(y2 + t * 0.5)),
                                        point(px(x1), px(y1 + t * 0.5)),
                                        point(px(x0), px(y0 + t * 0.5)),
                                    );
                                    builder.line_to(point(px(x0), px(y0 - t * 0.5)));

                                    if let Ok(path) = builder.build() {
                                        window.paint_path(path, edge_stroke_color);
                                    }
                                }
                            },
                        )
                        .size_full()
                        .absolute()
                        .top_0()
                        .left_0(),
                    )
                    // Placed Node Cards
                    .children(visible_node_views),
            )
            // Bottom Path & Status Bar
            .child(DiagramBottomBar::new(
                selected_path.clone(),
                total_nodes,
                visible_count,
                graph.warning_message.clone(),
                cx.listener(move |_this, _, _window, cx| {
                    if let Some(ref path) = selected_path {
                        cx.write_to_clipboard(gpui::ClipboardItem::new_string(path.clone()));
                    }
                }),
            ))
            .into_any_element()
    }
}
