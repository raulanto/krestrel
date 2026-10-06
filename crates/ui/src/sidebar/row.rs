//! Row component (`RenderOnce`) for individual tree nodes in the sidebar.

use crate::sidebar::tree_state::{FlatNodeKind, FlatTreeNode};
use gpui::{
    AnyElement, App, ClickEvent, FontWeight, IntoElement, ParentElement as _, RenderOnce,
    Styled as _, Window, div, px, rgb,
};
use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName,
    button::{Button, ButtonVariants as _},
    h_flex,
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

        let mut btn = Button::new(format!("row-node-{}", self.node.id))
            .ghost()
            .w_full();

        if let Some(handler) = self.on_click {
            btn = btn.on_click(handler);
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

                let chevron = Icon::new(chevron_name).text_color(theme.muted_foreground);

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
                    .child(Icon::new(icon_name).text_color(rgb(0xe06c1b)))
                    .child(render_highlighted_text(
                        &self.node.name,
                        &self.node.matched_indices,
                        cx,
                    ));

                btn.child(folder_content)
            }
            FlatNodeKind::Request { method, .. } => {
                let req_content = h_flex()
                    .w_full()
                    .pl(px(indent + 16.0))
                    .items_center()
                    .gap_2()
                    .child(method_badge(*method))
                    .child(render_highlighted_text(
                        &self.node.name,
                        &self.node.matched_indices,
                        cx,
                    ));

                btn.child(req_content)
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
                    .child(div().text_color(rgb(0xef4444)).child(err_label));

                btn.child(err_content)
            }
        }
    }
}

fn method_badge(method: HttpMethod) -> impl IntoElement {
    let (bg, fg, label) = match method {
        HttpMethod::GET => (rgb(0x22c55e), rgb(0xffffff), "GET"),
        HttpMethod::POST => (rgb(0xeab308), rgb(0x000000), "POST"),
        HttpMethod::PUT => (rgb(0x3b82f6), rgb(0xffffff), "PUT"),
        HttpMethod::DELETE => (rgb(0xef4444), rgb(0xffffff), "DEL"),
        HttpMethod::PATCH => (rgb(0xa855f7), rgb(0xffffff), "PATCH"),
        HttpMethod::HEAD => (rgb(0x6b7280), rgb(0xffffff), "HEAD"),
        HttpMethod::OPTIONS => (rgb(0x6b7280), rgb(0xffffff), "OPT"),
    };

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
            .text_color(theme.foreground)
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
                .text_color(theme.accent_foreground)
                .bg(theme.accent)
                .child(ch.to_string())
        } else {
            div().text_color(theme.foreground).child(ch.to_string())
        };
        container = container.child(span);
    }

    container.into_any_element()
}
