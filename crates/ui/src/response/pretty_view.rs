use bytes::Bytes;
use ely_gpui_component::buttons::{Button, ButtonVariant};
use ely_gpui_component::primitives::{Icon, IconName};
use gpui::{
    FontWeight, InteractiveElement as _, IntoElement, ParentElement as _, RenderOnce,
    StatefulInteractiveElement as _, Styled, div, px,
};
use kestrel_http::{
    ContentTypeCategory, HighlightRange, decode_body, detect_content_type, format_byte_size,
    format_pretty_json, format_pretty_xml, highlight_json,
};

use super::highlight::render_highlighted_line;
use super::raw_view::RawView;
use crate::theme::{ThemeExt as _, h_flex, v_flex};

const LARGE_BODY_THRESHOLD_BYTES: usize = 5 * 1024 * 1024; // 5 MB

#[derive(IntoElement)]
pub struct PrettyView<F1, F2>
where
    F1: Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    F2: Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
{
    body: Bytes,
    content_type: Option<String>,
    is_word_wrap: bool,
    force_format_large: bool,
    search_query: Option<String>,
    on_force_format: F1,
    on_save: F2,
}

impl<F1, F2> PrettyView<F1, F2>
where
    F1: Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    F2: Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
{
    pub fn new(
        body: Bytes,
        content_type: Option<String>,
        is_word_wrap: bool,
        force_format_large: bool,
        search_query: Option<String>,
        on_force_format: F1,
        on_save: F2,
    ) -> Self {
        Self {
            body,
            content_type,
            is_word_wrap,
            force_format_large,
            search_query,
            on_force_format,
            on_save,
        }
    }
}

impl<F1, F2> RenderOnce for PrettyView<F1, F2>
where
    F1: Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    F2: Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
{
    fn render(self, _window: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let body_len = self.body.len();

        let category = detect_content_type(self.content_type.as_deref(), &self.body);

        // If body is Image or Binary, delegate to RawView
        if matches!(
            category,
            ContentTypeCategory::Image | ContentTypeCategory::Binary
        ) {
            return RawView::new(
                self.body,
                self.content_type,
                self.is_word_wrap,
                self.search_query,
                self.on_save,
            )
            .into_any_element();
        }

        // Check large payload threshold (> 5MB)
        if body_len > LARGE_BODY_THRESHOLD_BYTES && !self.force_format_large {
            let size_str = format_byte_size(body_len);
            return v_flex()
                .flex_1()
                .items_center()
                .justify_center()
                .gap_4()
                .p_8()
                .child(
                    div()
                        .p_4()
                        .rounded_full()
                        .bg(colors.surface)
                        .border_1()
                        .border_color(colors.border)
                        .child(Icon::new(IconName::Globe).color(colors.warning)),
                )
                .child(
                    v_flex()
                        .items_center()
                        .gap_1()
                        .child(
                            div()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_sm()
                                .text_color(colors.fg)
                                .child(format!("Respuesta de gran tamaño ({})", size_str)),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(colors.fg_muted)
                                .child("El formateo Pretty y el resaltado están desactivados para mantener fluida la interfaz."),
                        ),
                )
                .child(
                    h_flex()
                        .gap_2()
                        .child(
                            Button::new("force-format-btn", "Formatear de todos modos")
                                .variant(ButtonVariant::Primary)
                                .on_click(self.on_force_format),
                        )
                        .child(
                            Button::new("save-large-file-btn", "Guardar a archivo")
                                .variant(ButtonVariant::Ghost)
                                .on_click(self.on_save),
                        ),
                )
                .into_any_element();
        }

        let (decoded_raw, _is_lossy) = decode_body(&self.body, self.content_type.as_deref());

        match category {
            ContentTypeCategory::Json => {
                match format_pretty_json(&decoded_raw) {
                    Ok(formatted_json) => {
                        let highlights: Vec<HighlightRange> = highlight_json(&formatted_json);
                        let lines: Vec<&str> = formatted_json.lines().collect();
                        let line_count = lines.len();

                        // Compute line byte offsets
                        let mut line_offsets = Vec::with_capacity(line_count);
                        let mut current_offset = 0;
                        for l in &lines {
                            line_offsets.push(current_offset);
                            current_offset += l.len() + 1; // account for newline
                        }

                        v_flex()
                            .id("pretty-json-scroll")
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
                                    .children(lines.into_iter().enumerate().map(
                                        |(idx, line_str)| {
                                            let line_no = idx + 1;
                                            let offset = line_offsets[idx];

                                            h_flex()
                                                .items_start()
                                                .gap_3()
                                                .child(
                                                    div()
                                                        .w(px(if line_count > 999 {
                                                            44.
                                                        } else {
                                                            32.
                                                        }))
                                                        .flex_shrink_0()
                                                        .text_right()
                                                        .font_family("monospace")
                                                        .text_xs()
                                                        .text_color(colors.fg_muted.opacity(0.6))
                                                        .child(format!("{}", line_no)),
                                                )
                                                .child(div().flex_1().min_w_0().child(
                                                    render_highlighted_line(
                                                        line_str,
                                                        offset,
                                                        &highlights,
                                                        self.search_query.as_deref(),
                                                        self.is_word_wrap,
                                                        cx,
                                                    ),
                                                ))
                                        },
                                    )),
                            )
                            .into_any_element()
                    }
                    Err(parse_err) => {
                        // Fallback to raw with invalid JSON banner
                        v_flex()
                            .flex_1()
                            .min_h_0()
                            .w_full()
                            .gap_2()
                            .child(
                                h_flex()
                                    .flex_shrink_0()
                                    .px_3()
                                    .py_2()
                                    .rounded_md()
                                    .bg(colors.danger.opacity(0.1))
                                    .border_1()
                                    .border_color(colors.danger.opacity(0.35))
                                    .gap_2p5()
                                    .items_center()
                                    .child(Icon::new(IconName::X).color(colors.danger))
                                    .child(
                                        div()
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_xs()
                                            .text_color(colors.danger)
                                            .child(format!("{}", parse_err)),
                                    ),
                            )
                            .child(RawView::new(
                                self.body,
                                self.content_type,
                                self.is_word_wrap,
                                self.search_query,
                                self.on_save,
                            ))
                            .into_any_element()
                    }
                }
            }
            ContentTypeCategory::Xml | ContentTypeCategory::Html => {
                let formatted_xml = format_pretty_xml(&decoded_raw);
                let lines: Vec<&str> = formatted_xml.lines().collect();
                let line_count = lines.len();

                v_flex()
                    .id("pretty-xml-scroll")
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
                                            .text_color(colors.accent)
                                            .child(line_str.to_string()),
                                    )
                            })),
                    )
                    .into_any_element()
            }
            _ => RawView::new(
                self.body,
                self.content_type,
                self.is_word_wrap,
                self.search_query,
                self.on_save,
            )
            .into_any_element(),
        }
    }
}
