use ely_gpui_component::buttons::IconButton;
use ely_gpui_component::primitives::{Icon, IconName};
use ely_gpui_component::theme::ControlSize;
use gpui::{
    ClickEvent, FontWeight, IntoElement, ParentElement as _, RenderOnce, Styled, Window, div, px,
};

use crate::theme::{ThemeExt as _, h_flex};

#[derive(IntoElement)]
pub struct DiagramBottomBar<F1>
where
    F1: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
{
    selected_path: Option<String>,
    total_nodes: usize,
    visible_nodes: usize,
    warning_message: Option<String>,
    on_copy_path: F1,
}

impl<F1> DiagramBottomBar<F1>
where
    F1: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
{
    pub fn new(
        selected_path: Option<String>,
        total_nodes: usize,
        visible_nodes: usize,
        warning_message: Option<String>,
        on_copy_path: F1,
    ) -> Self {
        Self {
            selected_path,
            total_nodes,
            visible_nodes,
            warning_message,
            on_copy_path,
        }
    }
}

impl<F1> RenderOnce for DiagramBottomBar<F1>
where
    F1: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
{
    fn render(self, _window: &mut Window, cx: &mut gpui::App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;

        let path_text = self
            .selected_path
            .clone()
            .unwrap_or_else(|| "Selecciona un nodo para ver su ruta JSON".to_string());

        let has_path = self.selected_path.is_some();

        h_flex()
            .h(px(32.))
            .px_3()
            .items_center()
            .justify_between()
            .bg(colors.surface)
            .border_t_1()
            .border_color(colors.border)
            // Left: Path and copy button
            .child(
                h_flex()
                    .items_center()
                    .gap_2()
                    .min_w_0()
                    .child(
                        div()
                            .text_xs()
                            .font_family("monospace")
                            .font_weight(if has_path {
                                FontWeight::MEDIUM
                            } else {
                                FontWeight::NORMAL
                            })
                            .text_color(if has_path {
                                colors.syntax.property
                            } else {
                                colors.fg_muted
                            })
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .child(path_text),
                    )
                    .children(if has_path {
                        Some(
                            IconButton::new("diag-copy-path", IconName::Copy)
                                .size(ControlSize::Sm)
                                .on_click(self.on_copy_path),
                        )
                    } else {
                        None
                    }),
            )
            // Right: Warnings and metrics
            .child(
                h_flex()
                    .items_center()
                    .gap_3()
                    .flex_shrink_0()
                    .children(self.warning_message.map(|msg| {
                        h_flex()
                            .items_center()
                            .gap_1()
                            .child(Icon::new(IconName::Globe).color(colors.warning))
                            .child(div().text_xs().text_color(colors.warning).child(msg))
                    }))
                    .child(div().text_xs().text_color(colors.fg_muted).child(format!(
                        "Nodos: {} visible{} de {}",
                        self.visible_nodes,
                        if self.visible_nodes == 1 { "" } else { "s" },
                        self.total_nodes
                    ))),
            )
    }
}
