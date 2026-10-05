use gpui::{
    App, AppContext, Context, Entity, EventEmitter, FontWeight, IntoElement, ParentElement as _,
    Render, Styled, Subscription, Window, div, prelude::FluentBuilder as _, px, rgb,
};
use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Input, InputEvent, InputState},
    v_flex,
};
use kestrel_core::{Body, HttpMethod, Request};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorTab {
    Path,
    Query,
    Headers,
    Body,
    Authentication,
}

pub enum EditorEvent {
    SendRequest,
    SaveRequest,
    UrlChanged(String),
    MethodChanged(HttpMethod),
}

pub struct RequestEditor {
    request: Option<Request>,
    active_tab: EditorTab,
    url_input: Entity<InputState>,
    body_input: Entity<InputState>,
    _subscriptions: Vec<Subscription>,
}

impl RequestEditor {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let url_input = cx
            .new(|cx| InputState::new(window, cx).placeholder("https://api.example.com/endpoint"));

        let body_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("{\n  \"key\": \"value\"\n}"));

        let sub_url = cx.subscribe_in(&url_input, window, |_, _, ev: &InputEvent, _window, cx| {
            if let InputEvent::Change = ev {
                cx.notify();
            }
        });

        Self {
            request: None,
            active_tab: EditorTab::Body,
            url_input,
            body_input,
            _subscriptions: vec![sub_url],
        }
    }

    pub fn set_request(&mut self, req: Request, window: &mut Window, cx: &mut Context<Self>) {
        let url_str = req.url.clone();
        self.url_input.update(cx, |input, cx| {
            input.set_value(url_str, window, cx);
        });

        if let Body::Json { content } = &req.body {
            let body_str = content.clone();
            self.body_input.update(cx, |input, cx| {
                input.set_value(body_str, window, cx);
            });
        }

        self.request = Some(req);
        cx.notify();
    }
}

impl EventEmitter<EditorEvent> for RequestEditor {}

impl Render for RequestEditor {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let method = self
            .request
            .as_ref()
            .map(|r| r.method)
            .unwrap_or(HttpMethod::GET);
        let active_tab = self.active_tab;

        let req_name = self
            .request
            .as_ref()
            .map(|r| r.name.clone())
            .unwrap_or_else(|| "Nueva Solicitud".to_string());

        v_flex()
            .size_full()
            .p_4()
            .gap_4()
            // Breadcrumbs / Title bar
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .w_full()
                    .child(
                        h_flex()
                            .items_center()
                            .gap_1p5()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(div().child("Colección"))
                            .child(div().child(">"))
                            .child(
                                div()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(theme.foreground)
                                    .child(req_name),
                            ),
                    )
                    .child(
                        Button::new("save-btn")
                            .ghost()
                            .icon(Icon::new(IconName::FileText))
                            .on_click(cx.listener(|_this, _, _window, cx| {
                                cx.emit(EditorEvent::SaveRequest);
                            })),
                    ),
            )
            // URL Bar: Method Selector + URL input + Send Button
            .child(
                h_flex()
                    .items_center()
                    .gap_2()
                    .w_full()
                    .child(
                        // Method Selector Pill
                        h_flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .px_3()
                            .h(px(36.))
                            .rounded_md()
                            .border_1()
                            .border_color(theme.border)
                            .bg(theme.background)
                            .child(
                                div()
                                    .font_weight(FontWeight::BOLD)
                                    .text_xs()
                                    .text_color(method_color(method))
                                    .child(format!("{:?}", method)),
                            )
                            .child(Icon::new(IconName::ChevronDown)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .child(Input::new(&self.url_input).bordered(true).cleanable(true)),
                    )
                    .child(
                        Button::new("send-req-btn")
                            .primary()
                            .label("Enviar")
                            .on_click(cx.listener(|_this, _, _window, cx| {
                                cx.emit(EditorEvent::SendRequest);
                            })),
                    ),
            )
            // Sub-tabs: Path, Query, Headers, Body, Authentication
            .child(
                h_flex()
                    .items_center()
                    .gap_2()
                    .border_b_1()
                    .border_color(theme.border)
                    .pb_2()
                    .child(render_subtab_button(
                        "subtab-path",
                        "Path 0",
                        active_tab == EditorTab::Path,
                        cx.listener(|this, _, _, cx| {
                            this.active_tab = EditorTab::Path;
                            cx.notify();
                        }),
                    ))
                    .child(render_subtab_button(
                        "subtab-query",
                        "Query 0",
                        active_tab == EditorTab::Query,
                        cx.listener(|this, _, _, cx| {
                            this.active_tab = EditorTab::Query;
                            cx.notify();
                        }),
                    ))
                    .child(render_subtab_button(
                        "subtab-headers",
                        "Headers 1",
                        active_tab == EditorTab::Headers,
                        cx.listener(|this, _, _, cx| {
                            this.active_tab = EditorTab::Headers;
                            cx.notify();
                        }),
                    ))
                    .child(render_subtab_button(
                        "subtab-body",
                        "Body",
                        active_tab == EditorTab::Body,
                        cx.listener(|this, _, _, cx| {
                            this.active_tab = EditorTab::Body;
                            cx.notify();
                        }),
                    ))
                    .child(render_subtab_button(
                        "subtab-auth",
                        "Autenticación",
                        active_tab == EditorTab::Authentication,
                        cx.listener(|this, _, _, cx| {
                            this.active_tab = EditorTab::Authentication;
                            cx.notify();
                        }),
                    )),
            )
            // Format Selector for Body (None, JSON, Text, XML, Form, Multipart, File)
            .when(active_tab == EditorTab::Body, |this| {
                this.child(
                    h_flex()
                        .gap_3()
                        .text_xs()
                        .font_weight(FontWeight::MEDIUM)
                        .child(format_pill("JSON", true))
                        .child(format_pill("Text", false))
                        .child(format_pill("XML", false))
                        .child(format_pill("Form", false))
                        .child(format_pill("Multipart", false))
                        .child(format_pill("Archivo", false)),
                )
            })
            // Editor Canvas / Input area
            .child(
                div()
                    .flex_1()
                    .rounded_md()
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.background)
                    .p_3()
                    .child(Input::new(&self.body_input).h_full().cleanable(false)),
            )
    }
}

fn render_subtab_button(
    id: impl Into<gpui::ElementId>,
    label: &str,
    is_active: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> Button {
    let btn = Button::new(id).label(label.to_string()).on_click(on_click);
    if is_active {
        btn.outline()
    } else {
        btn.ghost()
    }
}

fn format_pill(label: &str, is_selected: bool) -> impl IntoElement {
    div()
        .px_1p5()
        .py_0p5()
        .rounded_xs()
        .when(is_selected, |this| {
            this.font_weight(FontWeight::BOLD)
                .border_b_2()
                .border_color(rgb(0xe06c1b))
                .text_color(rgb(0xe06c1b))
        })
        .when(!is_selected, |this| this.opacity(0.6))
        .child(label.to_string())
}

fn method_color(method: HttpMethod) -> gpui::Rgba {
    match method {
        HttpMethod::GET => rgb(0x10b981),
        HttpMethod::POST => rgb(0x3b82f6),
        HttpMethod::PUT => rgb(0xf59e0b),
        HttpMethod::DELETE => rgb(0xef4444),
        HttpMethod::PATCH => rgb(0x8b5cf6),
        HttpMethod::HEAD => rgb(0x6b7280),
        HttpMethod::OPTIONS => rgb(0x6b7280),
    }
}
