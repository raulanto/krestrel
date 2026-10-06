//! Row component (`RenderOnce`) for individual tree nodes in the sidebar.

use crate::sidebar::tree_state::{FlatNodeKind, FlatTreeNode};
use crate::theme::{ThemeExt as _, h_flex, http_method_badge};
use ely_gpui_component::primitives::{Icon, IconName};
use gpui::{
    AnyElement, App, ClickEvent, FontWeight, InteractiveElement as _, IntoElement,
    ParentElement as _, RenderOnce, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use kestrel_core::HttpMethod;

pub type RowClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

#[derive(IntoElement)]
pub struct SidebarRow {
    pub node: FlatTreeNode,
    pub is_selected: bool,
    pub is_focused: bool,
    pub on_click: Option<RowClickHandler>,
    pub on_toggle: Option<RowClickHandler>,
}

impl SidebarRow {
    pub fn new(node: FlatTreeNode) -> Self {
        Self {
            node,
            is_selected: false,
            is_focused: false,
            on_click: None,
            on_toggle: None,
        }
    }

    pub fn selected(mut self, is_selected: bool) -> Self {
        self.is_selected = is_selected;
        self
    }

    pub fn focused(mut self, is_focused: bool) -> Self {
        self.is_focused = is_focused;
        self
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    pub fn on_toggle(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_toggle = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for SidebarRow {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let indent = (self.node.depth as f32) * 12.0;

        let mut row_container = div()
            .id(format!("row-node-{}", self.node.id))
            .w_full()
            .py_1()
            .px_1()
            .rounded_md()
            .cursor_pointer()
            .hover(|s| s.bg(theme.colors.hover))
            .active(|s| s.bg(theme.colors.active));

        if self.is_selected {
            row_container = row_container.bg(theme.colors.active);
        }

        if let Some(handler) = self.on_click {
            row_container = row_container.on_click(handler);
        }

        match &self.node.kind {
            FlatNodeKind::Folder {
                is_collapsed,
                has_children,
            } => {
                let icon_name = if *is_collapsed {
                    IconName::Folder
                } else {
                    IconName::FolderOpen
                };

                let chevron_name = if *is_collapsed {
                    IconName::ChevronRight
                } else {
                    IconName::ChevronDown
                };

                let chevron = Icon::new(chevron_name).color(theme.colors.fg_muted);

                let folder_content = h_flex()
                    .w_full()
                    .pl(px(indent + 4.0))
                    .items_center()
                    .gap_2()
                    .child(if *has_children {
                        chevron.into_any_element()
                    } else {
                        div().w(px(12.0)).into_any_element()
                    })
                    .child(Icon::new(icon_name).color(theme.colors.warning))
                    .child(render_highlighted_text(
                        &self.node.name,
                        &self.node.matched_indices,
                        cx,
                    ));

                row_container.child(folder_content)
            }
            FlatNodeKind::Request { method, .. } => {
                let req_content = h_flex()
                    .w_full()
                    .pl(px(indent + 16.0))
                    .items_center()
                    .gap_2()
                    .child(method_badge(*method, cx))
                    .child(render_highlighted_text(
                        &self.node.name,
                        &self.node.matched_indices,
                        cx,
                    ));

                row_container.child(req_content)
            }
            FlatNodeKind::ErrorNode {
                error_message,
                line,
                column,
                ..
            } => {
                let err_label = format!(
                    "⚠️ {} (L{}:C{}) - {}",
                    self.node.name,
                    line.unwrap_or(0),
                    column.unwrap_or(0),
                    error_message
                );

                let err_content = h_flex()
                    .w_full()
                    .pl(px(indent + 16.0))
                    .items_center()
                    .gap_2()
                    .child(div().text_color(theme.colors.danger).child(err_label));

                row_container.child(err_content)
            }
        }
    }
}

fn method_badge(method: HttpMethod, cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    let (bg, fg, label) = http_method_badge(method, &theme.colors);

    div()
        .px_1()
        .py_0p5()
        .rounded_sm()
        .text_xs()
        .font_weight(FontWeight::BOLD)
        .bg(bg)
        .text_color(fg)
        .child(label)
}

fn render_highlighted_text(text: &str, matched_indices: &[usize], cx: &App) -> AnyElement {
    let theme = cx.theme();
    if matched_indices.is_empty() {
        return div()
            .text_color(theme.colors.fg)
            .child(text.to_string())
            .into_any_element();
    }

    let mut container = div().flex().items_center();
    let char_indices: Vec<(usize, char)> = text.char_indices().collect();

    for (idx, ch) in char_indices {
        let is_matched = matched_indices.contains(&idx);
        let span = if is_matched {
            div()
                .font_weight(FontWeight::BOLD)
                .text_color(theme.colors.on_accent)
                .bg(theme.colors.accent)
                .child(ch.to_string())
        } else {
            div().text_color(theme.colors.fg).child(ch.to_string())
        };
        container = container.child(span);
    }

    container.into_any_element()
}
