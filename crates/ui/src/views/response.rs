use crate::theme::{ThemeExt as _, h_flex, status_color, v_flex};
use ely_gpui_component::buttons::{Button, ButtonVariant};
use gpui::{
    Context, FontWeight, InteractiveElement as _, IntoElement, ParentElement as _, Render,
    StatefulInteractiveElement as _, Styled, Window, div, prelude::FluentBuilder as _, rgb,
};
use kestrel_http::{
    HttpResponse, Insight, InsightKind, JwtStatus, ResponseIntelligence, TimestampUnit,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseTab {
    Pretty,
    Raw,
    Headers,
    Insights,
}

pub struct ResponsePanel {
    response: Option<HttpResponse>,
    insights: Vec<Insight>,
    active_tab: ResponseTab,
    is_loading: bool,
    error: Option<String>,
    intelligence: ResponseIntelligence,
}

impl ResponsePanel {
    pub fn new() -> Self {
        Self {
            response: None,
            insights: Vec::new(),
            active_tab: ResponseTab::Pretty,
            is_loading: false,
            error: None,
            intelligence: ResponseIntelligence::new(),
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
        let body_str = String::from_utf8_lossy(&resp.body);
        let insights = self.intelligence.analyze(&body_str);

        self.response = Some(resp);
        self.insights = insights;
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
        let colors = &theme.colors;
        let active_tab = self.active_tab;
        let insights_count = self.insights.len();

        let (status_str, time_str, size_str, status_num) = if self.is_loading {
            (
                "Enviando...".to_string(),
                "...".to_string(),
                "-".to_string(),
                200,
            )
        } else if let Some(_err) = &self.error {
            ("Error".to_string(), "-".to_string(), "-".to_string(), 500)
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

                    (status_text, duration_text, size_text, resp.status)
                }
                None => (
                    "Sin respuesta".to_string(),
                    "-".to_string(),
                    "-".to_string(),
                    0,
                ),
            }
        };

        v_flex()
            .size_full()
            .p_4()
            .gap_3()
            // Header Bar
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
                                        .text_color(colors.accent)
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
                                    .text_color(status_color(status_num, colors))
                                    .child(status_str),
                            )
                            .child(div().text_color(colors.fg_muted).child("•"))
                            .child(div().text_color(colors.fg_muted).child(time_str))
                            .child(div().text_color(colors.fg_muted).child("•"))
                            .child(div().text_color(colors.fg_muted).child(size_str)),
                    ),
            )
            // Tab Buttons
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
                        "tab-insights",
                        &format!("Intelligence ({})", insights_count),
                        active_tab == ResponseTab::Insights,
                        cx.listener(|this, _, _, cx| {
                            this.active_tab = ResponseTab::Insights;
                            cx.notify();
                        }),
                    )),
            )
            // Body View Canvas
            .child(
                div()
                    .id("response-body-scroll")
                    .flex_1()
                    .rounded_md()
                    .border_1()
                    .border_color(colors.border)
                    .bg(colors.bg)
                    .p_3()
                    .overflow_y_scroll()
                    .child(self.render_content_canvas(cx)),
            )
    }
}

impl ResponsePanel {
    fn render_content_canvas(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;

        if self.is_loading {
            return div()
                .font_family("monospace")
                .text_xs()
                .text_color(colors.fg_muted)
                .child("Ejecutando solicitud en segundo plano...")
                .into_any_element();
        }

        if let Some(err) = &self.error {
            return div()
                .font_family("monospace")
                .text_xs()
                .text_color(colors.danger)
                .child(format!("Error al ejecutar la solicitud:\n\n{}", err))
                .into_any_element();
        }

        let Some(resp) = &self.response else {
            return div()
                .font_family("monospace")
                .text_xs()
                .text_color(colors.fg_muted)
                .child("Envía una solicitud para ver la respuesta aquí...")
                .into_any_element();
        };

        match self.active_tab {
            ResponseTab::Pretty => {
                let text =
                    if let Ok(json_val) = serde_json::from_slice::<serde_json::Value>(&resp.body) {
                        serde_json::to_string_pretty(&json_val)
                            .unwrap_or_else(|_| String::from_utf8_lossy(&resp.body).to_string())
                    } else {
                        String::from_utf8_lossy(&resp.body).to_string()
                    };
                div()
                    .font_family("monospace")
                    .text_xs()
                    .text_color(colors.fg)
                    .child(text)
                    .into_any_element()
            }
            ResponseTab::Raw => div()
                .font_family("monospace")
                .text_xs()
                .text_color(colors.fg)
                .child(String::from_utf8_lossy(&resp.body).to_string())
                .into_any_element(),
            ResponseTab::Headers => v_flex()
                .gap_1()
                .children(resp.headers.iter().map(|(k, v)| {
                    h_flex()
                        .gap_2()
                        .text_xs()
                        .font_family("monospace")
                        .child(
                            div()
                                .font_weight(FontWeight::BOLD)
                                .text_color(colors.fg)
                                .child(format!("{}:", k)),
                        )
                        .child(div().text_color(colors.fg_muted).child(v.clone()))
                }))
                .into_any_element(),
            ResponseTab::Insights => {
                if self.insights.is_empty() {
                    return div()
                        .text_xs()
                        .text_color(colors.fg_muted)
                        .child("No se detectaron hallazgos (JWT o Timestamps) en el cuerpo de la respuesta.")
                        .into_any_element();
                }

                v_flex()
                    .gap_3()
                    .children(self.insights.iter().enumerate().map(|(ix, insight)| {
                        match &insight.kind {
                            InsightKind::Jwt(jwt) => {
                                let (status_label, status_col) = match jwt.status {
                                    JwtStatus::Valid => ("Válido", colors.success),
                                    JwtStatus::Expired => ("Expirado", colors.danger),
                                    JwtStatus::NotYetValid => ("Aún no válido", colors.warning),
                                    JwtStatus::NoExpiration => ("Sin expiración", colors.fg_muted),
                                    JwtStatus::AlgNone => {
                                        ("Algoritmo none (Inseguro)", colors.danger)
                                    }
                                };

                                v_flex()
                                    .p_3()
                                    .rounded_md()
                                    .border_1()
                                    .border_color(colors.border)
                                    .bg(colors.sunken)
                                    .gap_2()
                                    .child(
                                        h_flex().justify_between().items_center().child(
                                            h_flex()
                                                .gap_2()
                                                .child(
                                                    div()
                                                        .font_weight(FontWeight::BOLD)
                                                        .text_xs()
                                                        .text_color(colors.info)
                                                        .child(format!("#{} Token JWT", ix + 1)),
                                                )
                                                .child(
                                                    div()
                                                        .px_1p5()
                                                        .py_0p5()
                                                        .rounded_xs()
                                                        .text_xs()
                                                        .font_weight(FontWeight::BOLD)
                                                        .bg(status_col)
                                                        .text_color(rgb(0xffffff))
                                                        .child(status_label),
                                                ),
                                        ),
                                    )
                                    .child(
                                        v_flex()
                                            .gap_1()
                                            .font_family("monospace")
                                            .text_xs()
                                            .child(div().text_color(colors.fg_muted).child(
                                                format!(
                                                        "Header: {}",
                                                        serde_json::to_string(&jwt.header)
                                                            .unwrap_or_default()
                                                    ),
                                            ))
                                            .child(div().text_color(colors.fg).child(format!(
                                                    "Payload: {}",
                                                    serde_json::to_string(&jwt.payload)
                                                        .unwrap_or_default()
                                                )))
                                            .when_some(jwt.exp, |this, exp| {
                                                this.child(div().text_color(colors.fg_muted).child(
                                                    format!(
                                                            "Exp (UTC): {}",
                                                            chrono::DateTime::from_timestamp(
                                                                exp, 0
                                                             )
                                                             .map(|dt| dt.to_string())
                                                             .unwrap_or_default()
                                                         ),
                                                ))
                                            }),
                                    )
                                    .into_any_element()
                            }
                            InsightKind::Timestamp(ts) => {
                                let unit_str = match ts.unit {
                                    TimestampUnit::Seconds => "segundos",
                                    TimestampUnit::Milliseconds => "milisegundos",
                                    TimestampUnit::Microseconds => "microsegundos",
                                    TimestampUnit::Nanoseconds => "nanosegundos",
                                };

                                v_flex()
                                    .p_3()
                                    .rounded_md()
                                    .border_1()
                                    .border_color(colors.border)
                                    .bg(colors.sunken)
                                    .gap_1()
                                    .child(
                                        h_flex()
                                            .gap_2()
                                            .child(
                                                div()
                                                    .font_weight(FontWeight::BOLD)
                                                    .text_xs()
                                                    .text_color(colors.accent)
                                                    .child(format!(
                                                        "#{} Timestamp Unix ({})",
                                                        ix + 1,
                                                        unit_str
                                                    )),
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_family("monospace")
                                                    .text_color(colors.fg)
                                                    .child(ts.raw.clone()),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .font_family("monospace")
                                            .text_xs()
                                            .text_color(colors.fg_muted)
                                            .child(format!("Fecha legible: {}", ts.formatted_utc)),
                                    )
                                    .into_any_element()
                            }
                        }
                    }))
                    .into_any_element()
            }
        }
    }
}

fn render_tab_button(
    id: impl Into<gpui::ElementId>,
    label: &str,
    is_active: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> Button {
    let mut btn = Button::new(id, label.to_string()).on_click(on_click);
    if is_active {
        btn = btn.variant(ButtonVariant::Outline);
    } else {
        btn = btn.variant(ButtonVariant::Ghost);
    }
    btn
}
