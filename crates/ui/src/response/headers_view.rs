use ely_gpui_component::buttons::IconButton;
use ely_gpui_component::primitives::IconName;
use ely_gpui_component::theme::ControlSize;
use gpui::{
    FontWeight, InteractiveElement as _, IntoElement, ParentElement as _, RenderOnce,
    StatefulInteractiveElement as _, Styled, div, px,
};

use crate::theme::{ThemeExt as _, h_flex, v_flex};

#[derive(IntoElement)]
pub struct HeadersView {
    headers: Vec<(String, String)>,
    url_final: String,
    http_version: String,
}

impl HeadersView {
    pub fn new(headers: Vec<(String, String)>, url_final: String, http_version: String) -> Self {
        Self {
            headers,
            url_final,
            http_version,
        }
    }
}

impl RenderOnce for HeadersView {
    fn render(self, _window: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;

        let total_headers = self.headers.len();

        v_flex()
            .flex_1()
            .min_h_0()
            .w_full()
            .gap_3()
            // Info Header Banner
            .child(
                h_flex()
                    .flex_shrink_0()
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .bg(colors.surface)
                    .border_1()
                    .border_color(colors.border)
                    .items_center()
                    .justify_between()
                    .child(
                        h_flex()
                            .gap_4()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(colors.fg_muted)
                                    .child(format!("Total: {} encabezados", total_headers)),
                            )
                            .child(
                                div()
                                    .font_family("monospace")
                                    .text_xs()
                                    .text_color(colors.fg_muted)
                                    .child(format!("Versión: {}", self.http_version)),
                            )
                            .child(
                                div()
                                    .font_family("monospace")
                                    .text_xs()
                                    .text_color(colors.fg_muted)
                                    .child(format!("URL final: {}", self.url_final)),
                            ),
                    )
                    .child(
                        IconButton::new("copy-all-headers-btn", IconName::Copy)
                            .size(ControlSize::Sm)
                            .tooltip("Copiar todos los encabezados")
                            .on_click({
                                let headers_clone = self.headers.clone();
                                move |_, _window, cx| {
                                    let text = headers_clone
                                        .iter()
                                        .map(|(k, v)| format!("{}: {}", k, v))
                                        .collect::<Vec<_>>()
                                        .join("\n");
                                    cx.write_to_clipboard(gpui::ClipboardItem::new_string(text));
                                }
                            }),
                    ),
            )
            // Table Canvas
            .child(
                v_flex()
                    .id("headers-table-scroll")
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .rounded_md()
                    .border_1()
                    .border_color(colors.border)
                    .bg(colors.bg)
                    .overflow_y_scroll()
                    .child(if self.headers.is_empty() {
                        div()
                            .p_4()
                            .text_sm()
                            .text_color(colors.fg_muted)
                            .child("No hay encabezados en la respuesta")
                            .into_any_element()
                    } else {
                        v_flex()
                            .children(self.headers.into_iter().enumerate().map(
                                |(idx, (name, val))| {
                                    let is_even = idx % 2 == 0;
                                    let row_bg = if is_even { colors.bg } else { colors.surface };
                                    let name_copy = name.clone();
                                    let val_copy = val.clone();

                                    h_flex()
                                        .px_3()
                                        .py_2()
                                        .bg(row_bg)
                                        .border_b_1()
                                        .border_color(colors.border)
                                        .items_center()
                                        .justify_between()
                                        .hover(|s| s.bg(colors.hover))
                                        .child(
                                            h_flex()
                                                .flex_1()
                                                .min_w_0()
                                                .items_start()
                                                .gap_3()
                                                .child(
                                                    div()
                                                        .w(px(220.))
                                                        .flex_shrink_0()
                                                        .font_family("monospace")
                                                        .font_weight(FontWeight::SEMIBOLD)
                                                        .text_xs()
                                                        .text_color(colors.accent)
                                                        .child(name),
                                                )
                                                .child(
                                                    div()
                                                        .flex_1()
                                                        .min_w_0()
                                                        .font_family("monospace")
                                                        .text_xs()
                                                        .text_color(colors.fg)
                                                        .child(val),
                                                ),
                                        )
                                        .child(
                                            IconButton::new(
                                                ("copy-header-row", idx),
                                                IconName::Copy,
                                            )
                                            .size(ControlSize::Sm)
                                            .on_click(
                                                move |_, _window, cx| {
                                                    let text =
                                                        format!("{}: {}", name_copy, val_copy);
                                                    cx.write_to_clipboard(
                                                        gpui::ClipboardItem::new_string(text),
                                                    );
                                                },
                                            ),
                                        )
                                },
                            ))
                            .into_any_element()
                    }),
            )
    }
}
