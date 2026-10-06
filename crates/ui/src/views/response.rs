use gpui::{
    Context, FontWeight, IntoElement, ParentElement as _, Render, Styled, Window, div,
    prelude::FluentBuilder as _, rgb,
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
    is_loading: bool,
    error: Option<String>,
}

impl ResponsePanel {
    pub fn new() -> Self {
        Self {
            response: None,
            active_tab: ResponseTab::Pretty,
            is_loading: false,
            error: None,
        }
    }

    pub fn set_loading(&mut self, is_loading: bool, cx: &mut Context<Self>) {
        self.is_loading = is_loading;
        if is_loading {
            self.error = None;
        }
        cx.notify();
    }

    pub fn set_response(&mut self, resp: HttpResponse, cx: &mut Context<Self>) {
        self.response = Some(resp);
        self.is_loading = false;
        self.error = None;
        cx.notify();
    }

    pub fn set_error(&mut self, err: String, cx: &mut Context<Self>) {
        self.error = Some(err);
        self.is_loading = false;
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

        let (status_str, time_str, size_str, is_success, body_preview) = if self.is_loading {
            (
                "Enviando...".to_string(),
                "...".to_string(),
                "-".to_string(),
                true,
                "Ejecutando solicitud en segundo plano...".to_string(),
            )
        } else if let Some(err) = &self.error {
            (
                "Error".to_string(),
                "-".to_string(),
                "-".to_string(),
                false,
                format!("Error al ejecutar la solicitud:\n\n{}", err),
            )
        } else {
            match &self.response {
                Some(resp) => {
                    let status_text = format!("{} {}", resp.status, resp.status_text);
                    let duration_ms = resp.timing.total.as_secs_f64() * 1000.0;
                    let duration_text = if duration_ms >= 1000.0 {
                        format!("{:.2} s", duration_ms / 1000.0)
                    } else {
                        format!("{:.2} ms", duration_ms)
                    };
                    let size_text = if resp.size.body_bytes < 1024 {
                        format!("{} B", resp.size.body_bytes)
                    } else if resp.size.body_bytes < 1024 * 1024 {
                        format!("{:.1} KB", resp.size.body_bytes as f64 / 1024.0)
                    } else {
                        format!("{:.2} MB", resp.size.body_bytes as f64 / (1024.0 * 1024.0))
                    };

                    let is_ok = resp.status >= 200 && resp.status < 400;

                    let body_str = match active_tab {
                        ResponseTab::Pretty => {
                            if let Ok(json_val) =
                                serde_json::from_slice::<serde_json::Value>(&resp.body)
                            {
                                serde_json::to_string_pretty(&json_val).unwrap_or_else(|_| {
                                    String::from_utf8_lossy(&resp.body).to_string()
                                })
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
            }
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
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .font_weight(FontWeight::BOLD)
                                    .text_sm()
                                    .child("Respuesta"),
                            )
                            .when(self.is_loading, |this| {
                                this.child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(rgb(0xe06c1b))
                                        .child("• Procesando..."),
                                )
                            }),
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
