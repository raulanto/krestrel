use ely_gpui_component::buttons::IconButton;
use ely_gpui_component::primitives::IconName;
use ely_gpui_component::theme::ControlSize;
use gpui::{FontWeight, IntoElement, ParentElement as _, RenderOnce, Styled, div, px};
use kestrel_http::{ResponseData, format_byte_size, format_duration};

use crate::theme::{ThemeExt as _, h_flex, status_color};

#[derive(IntoElement)]
pub struct StatusBar<F1, F2, F3, F4>
where
    F1: Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    F2: Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    F3: Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    F4: Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
{
    response: Option<ResponseData>,
    is_word_wrap: bool,
    is_search_open: bool,
    on_copy: F1,
    on_save: F2,
    on_toggle_wrap: F3,
    on_toggle_search: F4,
}

impl<F1, F2, F3, F4> StatusBar<F1, F2, F3, F4>
where
    F1: Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    F2: Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    F3: Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    F4: Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
{
    pub fn new(
        response: Option<ResponseData>,
        is_word_wrap: bool,
        is_search_open: bool,
        on_copy: F1,
        on_save: F2,
        on_toggle_wrap: F3,
        on_toggle_search: F4,
    ) -> Self {
        Self {
            response,
            is_word_wrap,
            is_search_open,
            on_copy,
            on_save,
            on_toggle_wrap,
            on_toggle_search,
        }
    }
}

impl<F1, F2, F3, F4> RenderOnce for StatusBar<F1, F2, F3, F4>
where
    F1: Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    F2: Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    F3: Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    F4: Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
{
    fn render(self, _window: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;

        let Some(resp) = self.response else {
            return h_flex()
                .h(px(36.))
                .flex_shrink_0()
                .px_3()
                .items_center()
                .justify_between()
                .border_t_1()
                .border_color(colors.border)
                .bg(colors.surface)
                .child(
                    div()
                        .text_xs()
                        .text_color(colors.fg_muted)
                        .child("Sin respuesta"),
                )
                .into_any_element();
        };

        let status_num = resp.status;
        let status_color_val = status_color(status_num, colors);
        let time_str = format_duration(resp.timing.total);
        let size_str = format_byte_size(resp.size.body_bytes);
        let headers_size_str = format_byte_size(resp.size.headers_bytes);

        h_flex()
            .h(px(36.))
            .flex_shrink_0()
            .px_3()
            .items_center()
            .justify_between()
            .border_t_1()
            .border_color(colors.border)
            .bg(colors.surface)
            // Left: Metrics (Status Code, Time, Size)
            .child(
                h_flex()
                    .items_center()
                    .gap_3()
                    // Status Badge
                    .child(
                        h_flex()
                            .items_center()
                            .gap_1p5()
                            .px_2()
                            .py_0p5()
                            .rounded_md()
                            .bg(status_color_val.opacity(0.15))
                            .border_1()
                            .border_color(status_color_val.opacity(0.35))
                            .child(div().size(px(6.)).rounded_full().bg(status_color_val))
                            .child(
                                div()
                                    .font_weight(FontWeight::BOLD)
                                    .font_family("monospace")
                                    .text_xs()
                                    .text_color(status_color_val)
                                    .child(format!("{} {}", resp.status, resp.status_text)),
                            ),
                    )
                    // Timing Badge
                    .child(
                        h_flex()
                            .items_center()
                            .gap_1()
                            .text_xs()
                            .font_family("monospace")
                            .text_color(colors.fg_muted)
                            .child("⏱")
                            .child(div().font_weight(FontWeight::MEDIUM).child(time_str)),
                    )
                    // Size Badge (body + headers breakdown)
                    .child(
                        h_flex()
                            .items_center()
                            .gap_1()
                            .text_xs()
                            .font_family("monospace")
                            .text_color(colors.fg_muted)
                            .child("📦")
                            .child(
                                div()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(format!("{} (h: {})", size_str, headers_size_str)),
                            ),
                    ),
            )
            // Right: Action Buttons (Search, Word Wrap, Copy, Save)
            .child(
                h_flex()
                    .items_center()
                    .gap_1p5()
                    .child(
                        IconButton::new("search-btn", IconName::Search)
                            .size(ControlSize::Sm)
                            .variant(if self.is_search_open {
                                ely_gpui_component::buttons::ButtonVariant::Secondary
                            } else {
                                ely_gpui_component::buttons::ButtonVariant::Ghost
                            })
                            .tooltip("Buscar (Ctrl+F)")
                            .on_click(self.on_toggle_search),
                    )
                    .child(
                        IconButton::new("wrap-btn", IconName::ChevronsUpDown)
                            .size(ControlSize::Sm)
                            .variant(if self.is_word_wrap {
                                ely_gpui_component::buttons::ButtonVariant::Secondary
                            } else {
                                ely_gpui_component::buttons::ButtonVariant::Ghost
                            })
                            .tooltip("Ajuste de línea")
                            .on_click(self.on_toggle_wrap),
                    )
                    .child(
                        IconButton::new("copy-body-btn", IconName::Copy)
                            .size(ControlSize::Sm)
                            .variant(ely_gpui_component::buttons::ButtonVariant::Ghost)
                            .tooltip("Copiar respuesta")
                            .on_click(self.on_copy),
                    )
                    .child(
                        IconButton::new("save-body-btn", IconName::HardDrive)
                            .size(ControlSize::Sm)
                            .variant(ely_gpui_component::buttons::ButtonVariant::Ghost)
                            .tooltip("Guardar en archivo")
                            .on_click(self.on_save),
                    ),
            )
            .into_any_element()
    }
}
