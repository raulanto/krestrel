use gpui::{
    Context, FontWeight, IntoElement, ParentElement as _, Render, Styled, Window, div, rgb,
};
use gpui_kit::component::{
    ActiveTheme as _,
    button::{Button, ButtonVariants as _},
    h_flex,
    scroll::ScrollableElement as _,
    v_flex,
};
use kestrel_http::HttpResponse;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseTab {
    Pretty,
    Raw,
    Headers,
    Inspect,
}

pub struct ResponsePanel {
    response: Option<HttpResponse>,
    active_tab: ResponseTab,
}

impl ResponsePanel {
    pub fn new() -> Self {
        Self {
            response: None,
            active_tab: ResponseTab::Pretty,
        }
    }

    pub fn set_response(&mut self, resp: HttpResponse, cx: &mut Context<Self>) {
        self.response = Some(resp);
        cx.notify();
    }
}

impl Default for ResponsePanel {
    fn default() -> Self {
        Self::new()
    }
}

impl Render for ResponsePanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let active_tab = self.active_tab;

        let (status_str, time_str, size_str, is_success, body_preview) = match &self.response {
            Some(resp) => {
                let status_text = format!("{} {}", resp.status, resp.status_text);
                let duration_text = format!("{:.2} ms", resp.duration.as_secs_f64() * 1000.0);
                let size_text = if resp.size_bytes < 1024 {
                    format!("{} B", resp.size_bytes)
                } else {
                    format!("{:.1} KB", resp.size_bytes as f64 / 1024.0)
                };

                let is_ok = (200..300).contains(&resp.status);

                let body_str = match active_tab {
                    ResponseTab::Pretty => {
                        if let Ok(json_val) =
                            serde_json::from_slice::<serde_json::Value>(&resp.body)
                        {
                            serde_json::to_string_pretty(&json_val)
                                .unwrap_or_else(|_| String::from_utf8_lossy(&resp.body).to_string())
                        } else {
                            String::from_utf8_lossy(&resp.body).to_string()
                        }
                    }
                    ResponseTab::Raw => String::from_utf8_lossy(&resp.body).to_string(),
                    ResponseTab::Headers => resp
                        .headers
                        .iter()
                        .map(|(k, v)| format!("{}: {}", k, v))
                        .collect::<Vec<_>>()
                        .join("\n"),
                    ResponseTab::Inspect => format!(
                        "Estado: {}\nTiempo: {}\nTamaño: {}\nHeaders: {}",
                        status_text,
                        duration_text,
                        size_text,
                        resp.headers.len()
                    ),
                };

                (status_text, duration_text, size_text, is_ok, body_str)
            }
            None => (
                "Sin respuesta".to_string(),
                "-".to_string(),
                "-".to_string(),
                true,
                "Envía una solicitud para ver la respuesta aquí...".to_string(),
            ),
        };

        v_flex()
            .size_full()
            .p_4()
            .gap_3()
            // Top Response Header: Title + Status Badge + Time + Size
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .w_full()
                    .child(
                        div()
                            .font_weight(FontWeight::BOLD)
                            .text_sm()
                            .child("Respuesta"),
                    )
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .child(
                                div()
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(if is_success {
                                        rgb(0x10b981)
                                    } else {
                                        rgb(0xef4444)
                                    })
                                    .child(status_str),
                            )
                            .child(div().text_color(theme.muted_foreground).child("•"))
                            .child(div().text_color(theme.muted_foreground).child(time_str))
                            .child(div().text_color(theme.muted_foreground).child("•"))
                            .child(div().text_color(theme.muted_foreground).child(size_str)),
                    ),
            )
            // Tabs: Pretty, Raw, Headers, Inspect
            .child(
                h_flex()
                    .gap_2()
                    .text_xs()
                    .font_weight(FontWeight::MEDIUM)
                    .child(render_tab_button(
                        "tab-pretty",
                        "Pretty",
                        active_tab == ResponseTab::Pretty,
                        cx.listener(|this, _, _, cx| {
                            this.active_tab = ResponseTab::Pretty;
                            cx.notify();
                        }),
                    ))
                    .child(render_tab_button(
                        "tab-raw",
                        "Raw",
                        active_tab == ResponseTab::Raw,
                        cx.listener(|this, _, _, cx| {
                            this.active_tab = ResponseTab::Raw;
                            cx.notify();
                        }),
                    ))
                    .child(render_tab_button(
                        "tab-headers",
                        "Headers",
                        active_tab == ResponseTab::Headers,
                        cx.listener(|this, _, _, cx| {
                            this.active_tab = ResponseTab::Headers;
                            cx.notify();
                        }),
                    ))
                    .child(render_tab_button(
                        "tab-inspect",
                        "Inspect",
                        active_tab == ResponseTab::Inspect,
                        cx.listener(|this, _, _, cx| {
                            this.active_tab = ResponseTab::Inspect;
                            cx.notify();
                        }),
                    )),
            )
            // Response Viewer Canvas
            .child(
                div()
                    .flex_1()
                    .rounded_md()
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.background)
                    .p_3()
                    .overflow_y_scrollbar()
                    .font_family("monospace")
                    .text_xs()
                    .text_color(theme.foreground)
                    .child(body_preview),
            )
    }
}

fn render_tab_button(
    id: impl Into<gpui::ElementId>,
    label: &str,
    is_active: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> Button {
    let btn = Button::new(id).label(label.to_string()).on_click(on_click);
    if is_active {
        btn.outline()
    } else {
        btn.ghost()
    }
}
