use bytes::Bytes;
use ely_gpui_component::buttons::{Button, ButtonVariant};
use ely_gpui_component::primitives::{Icon, IconName};
use gpui::{
    FontWeight, InteractiveElement as _, IntoElement, ParentElement as _, RenderOnce,
    StatefulInteractiveElement as _, Styled, div, prelude::FluentBuilder as _, px,
};
use kestrel_http::{ContentTypeCategory, decode_body, detect_content_type, format_byte_size};

use crate::theme::{ThemeExt as _, h_flex, v_flex};

#[derive(IntoElement)]
pub struct RawView<F>
where
    F: Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
{
    body: Bytes,
    content_type: Option<String>,
    is_word_wrap: bool,
    _search_query: Option<String>,
    on_save: F,
}

impl<F> RawView<F>
where
    F: Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
{
    pub fn new(
        body: Bytes,
        content_type: Option<String>,
        is_word_wrap: bool,
        search_query: Option<String>,
        on_save: F,
    ) -> Self {
        Self {
            body,
            content_type,
            is_word_wrap,
            _search_query: search_query,
            on_save,
        }
    }
}

impl<F> RenderOnce for RawView<F>
where
    F: Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
{
    fn render(self, _window: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let category = detect_content_type(self.content_type.as_deref(), &self.body);

        match category {
            ContentTypeCategory::Image => {
                let size_str = format_byte_size(self.body.len());
                let ct_str = self.content_type.unwrap_or_else(|| "image/*".to_string());

                v_flex()
                    .flex_1()
                    .items_center()
                    .justify_center()
                    .gap_3()
                    .p_8()
                    .child(
                        div()
                            .p_4()
                            .rounded_full()
                            .bg(colors.surface)
                            .border_1()
                            .border_color(colors.border)
                            .child(Icon::new(IconName::FileText).color(colors.accent)),
                    )
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_sm()
                            .text_color(colors.fg)
                            .child(format!("Imagen recibida ({})", ct_str)),
                    )
                    .child(
                        div()
                            .font_family("monospace")
                            .text_xs()
                            .text_color(colors.fg_muted)
                            .child(format!("Tamaño: {}", size_str)),
                    )
                    .child(
                        Button::new("save-raw-image-btn", "Guardar imagen a disco")
                            .variant(ButtonVariant::Primary)
                            .on_click(self.on_save),
                    )
                    .into_any_element()
            }
            ContentTypeCategory::Binary => {
                let size_str = format_byte_size(self.body.len());
                let ct_str = self
                    .content_type
                    .unwrap_or_else(|| "application/octet-stream".to_string());

                v_flex()
                    .flex_1()
                    .items_center()
                    .justify_center()
                    .gap_3()
                    .p_8()
                    .child(
                        div()
                            .p_4()
                            .rounded_full()
                            .bg(colors.surface)
                            .border_1()
                            .border_color(colors.border)
                            .child(Icon::new(IconName::HardDrive).color(colors.warning)),
                    )
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_sm()
                            .text_color(colors.fg)
                            .child(format!("Contenido binario ({})", ct_str)),
                    )
                    .child(
                        div()
                            .font_family("monospace")
                            .text_xs()
                            .text_color(colors.fg_muted)
                            .child(format!("Tamaño del archivo: {}", size_str)),
                    )
                    .child(
                        Button::new("save-raw-binary-btn", "Guardar archivo a disco")
                            .variant(ButtonVariant::Primary)
                            .on_click(self.on_save),
                    )
                    .into_any_element()
            }
            _ => {
                let (decoded_text, is_lossy) =
                    decode_body(&self.body, self.content_type.as_deref());
                let lines: Vec<&str> = decoded_text.lines().collect();
                let line_count = lines.len();

                v_flex()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .gap_2()
                    .children(if is_lossy {
                        Some(
                            h_flex()
                                .flex_shrink_0()
                                .px_3()
                                .py_1p5()
                                .rounded_md()
                                .bg(colors.warning.opacity(0.1))
                                .border_1()
                                .border_color(colors.warning.opacity(0.3))
                                .gap_2()
                                .items_center()
                                .child(Icon::new(IconName::X).color(colors.warning))
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(colors.warning)
                                        .child("El cuerpo contiene caracteres no UTF-8 decodificados con pérdida."),
                                ),
                        )
                    } else {
                        None
                    })
                    .child(
                        v_flex()
                            .id("raw-body-scroll")
                            .flex_1()
                            .min_h_0()
                            .w_full()
                            .rounded_md()
                            .border_1()
                            .border_color(colors.border)
                            .bg(colors.bg)
                            .p_3()
                            .overflow_y_scroll()
                            .child(
                                v_flex()
                                    .gap_0p5()
                                    .children(lines.into_iter().enumerate().map(|(idx, line_str)| {
                                        let line_no = idx + 1;
                                        let is_wrap = self.is_word_wrap;

                                        h_flex()
                                            .items_start()
                                            .gap_3()
                                            .child(
                                                div()
                                                    .w(px(if line_count > 999 { 44. } else { 32. }))
                                                    .flex_shrink_0()
                                                    .text_right()
                                                    .font_family("monospace")
                                                    .text_xs()
                                                    .text_color(colors.fg_muted.opacity(0.6))
                                                    .child(format!("{}", line_no)),
                                            )
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .min_w_0()
                                                    .font_family("monospace")
                                                    .text_xs()
                                                    .text_color(colors.fg)
                                                    .when(is_wrap, |s| s.flex_wrap())
                                                    .child(line_str.to_string()),
                                            )
                                    })),
                            ),
                    )
                    .into_any_element()
            }
        }
    }
}
