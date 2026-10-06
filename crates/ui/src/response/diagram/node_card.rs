use ely_gpui_component::primitives::{Icon, IconName};
use gpui::{
    ClickEvent, CursorStyle, FontWeight, Hsla, InteractiveElement as _, IntoElement,
    ParentElement as _, RenderOnce, StatefulInteractiveElement as _, Styled, Window, div, px,
};
use kestrel_json_graph::{GraphNode, LayoutNode, NodeKind, PropertyRow};

use crate::theme::{ThemeExt as _, h_flex, syntax_type_color, v_flex};

#[derive(IntoElement)]
pub struct NodeCardView<F1, F2>
where
    F1: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F2: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
{
    node: GraphNode,
    layout_node: LayoutNode,
    is_selected: bool,
    on_select: F1,
    on_toggle_collapse: F2,
}

impl<F1, F2> NodeCardView<F1, F2>
where
    F1: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F2: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
{
    pub fn new(
        node: GraphNode,
        layout_node: LayoutNode,
        is_selected: bool,
        on_select: F1,
        on_toggle_collapse: F2,
    ) -> Self {
        Self {
            node,
            layout_node,
            is_selected,
            on_select,
            on_toggle_collapse,
        }
    }
}

impl<F1, F2> RenderOnce for NodeCardView<F1, F2>
where
    F1: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    F2: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
{
    fn render(self, _window: &mut Window, cx: &mut gpui::App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let bounds = self.layout_node.bounds;
        let is_selected = self.is_selected;
        let is_collapsed = self.layout_node.is_collapsed;
        let has_children = self.layout_node.has_children;

        let border_color: Hsla = if is_selected {
            colors.accent
        } else {
            colors.border
        };

        let card_bg = if is_selected {
            colors.surface
        } else {
            colors.surface.opacity(0.95)
        };

        let header_bg = colors.bg.opacity(0.6);

        let kind_badge_text = match &self.node.kind {
            NodeKind::ObjectCard => {
                if has_children {
                    Some(format!("{} hijos", self.node.children.len()))
                } else {
                    None
                }
            }
            NodeKind::ArrayContainer { count } => Some(format!("array[{}]", count)),
            NodeKind::PrimitiveLeaf => Some("valor".to_string()),
            NodeKind::ArrayChunk { start, end } => Some(format!("[{}..{}]", start, end)),
        };

        v_flex()
            .id(format!("node-card-{}", self.node.id))
            .w(px(bounds.width))
            .min_w(px(bounds.width))
            .max_w(px(bounds.width))
            .bg(card_bg)
            .border_1()
            .border_color(border_color)
            .rounded_md()
            .shadow_sm()
            .overflow_hidden()
            .cursor(CursorStyle::PointingHand)
            .on_click(self.on_select)
            // Header
            .child(
                h_flex()
                    .h(px(32.))
                    .px_2()
                    .items_center()
                    .justify_between()
                    .bg(header_bg)
                    .border_b_1()
                    .border_color(colors.border)
                    .child(
                        h_flex()
                            .items_center()
                            .gap_1p5()
                            .children(if has_children {
                                Some(
                                    div()
                                        .id(format!("toggle-{}", self.node.id))
                                        .p_1()
                                        .rounded_sm()
                                        .hover(|s| s.bg(colors.surface))
                                        .cursor(CursorStyle::PointingHand)
                                        .on_click(self.on_toggle_collapse)
                                        .child(if is_collapsed {
                                            Icon::new(IconName::ChevronRight).color(colors.fg_muted)
                                        } else {
                                            Icon::new(IconName::ChevronDown).color(colors.fg_muted)
                                        }),
                                )
                            } else {
                                None
                            })
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(colors.fg)
                                    .child(self.node.label.clone()),
                            ),
                    )
                    .children(kind_badge_text.map(|txt| {
                        div()
                            .px_1p5()
                            .py_0p5()
                            .rounded_full()
                            .bg(colors.surface)
                            .border_1()
                            .border_color(colors.border)
                            .text_xs()
                            .text_color(colors.fg_muted)
                            .child(txt)
                    })),
            )
            // Body Rows (if any)
            .children(if self.node.properties.is_empty() {
                None
            } else {
                Some(
                    v_flex()
                        .p_2()
                        .gap_1()
                        .children(
                            self.node
                                .properties
                                .iter()
                                .map(|prop| render_property_row(prop, colors)),
                        )
                        .children(if self.node.truncated_rows > 0 {
                            Some(
                                div()
                                    .text_xs()
                                    .text_color(colors.fg_muted)
                                    .pt_1()
                                    .child(format!("+ {} campos más…", self.node.truncated_rows)),
                            )
                        } else {
                            None
                        }),
                )
            })
    }
}

fn render_property_row(
    prop: &PropertyRow,
    colors: &ely_gpui_component::theme::Palette,
) -> impl IntoElement {
    let type_name = prop.value.primitive_type().name();
    let val_color = syntax_type_color(type_name, colors);

    h_flex()
        .w_full()
        .items_center()
        .justify_between()
        .gap_2()
        .text_xs()
        .font_family("monospace")
        .child(
            div()
                .text_color(colors.syntax.property)
                .child(format!("{}:", prop.key)),
        )
        .child(
            div()
                .text_color(val_color)
                .overflow_hidden()
                .whitespace_nowrap()
                .child(prop.value.display_text()),
        )
}
