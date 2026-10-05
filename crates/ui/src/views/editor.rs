use gpui::{
    App, AppContext, Context, Entity, EventEmitter, FontWeight, IntoElement, ParentElement as _,
    Render, Styled, Subscription, Window, div, prelude::FluentBuilder as _, px, rgb,
};
use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Input, InputEvent, InputState},
    scroll::ScrollableElement as _,
    v_flex,
};
use kestrel_core::{
    ApiKeyLocation, Auth, Body, FormEntry, FormValue, HeaderParam, HttpMethod, KeyValuePair,
    QueryParam, Request,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorTab {
    Path,
    Query,
    Headers,
    Body,
    Authentication,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyFormat {
    None,
    Json,
    Text,
    Xml,
    FormUrlEncoded,
    Multipart,
    Binary,
    GraphQL,
}

pub enum EditorEvent {
    SendRequest,
    SaveRequest,
    RequestModified,
    UrlChanged(String),
    MethodChanged(HttpMethod),
}

pub struct RequestEditor {
    request: Option<Request>,
    active_tab: EditorTab,
    selected_body_format: BodyFormat,
    url_input: Entity<InputState>,
    body_input: Entity<InputState>,
    graphql_vars_input: Entity<InputState>,
    auth_token_input: Entity<InputState>,
    auth_user_input: Entity<InputState>,
    auth_pass_input: Entity<InputState>,
    auth_key_name_input: Entity<InputState>,
    auth_key_val_input: Entity<InputState>,
    active_environment: Option<kestrel_core::Environment>,
    is_dirty: bool,
    _subscriptions: Vec<Subscription>,
}

impl RequestEditor {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let url_input = cx
            .new(|cx| InputState::new(window, cx).placeholder("https://api.example.com/endpoint"));

        let body_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("{\n  \"clave\": \"valor\"\n}"));

        let graphql_vars_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("{\n  \"variable\": 123\n}"));

        let auth_token_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("Token de autenticación (ej. Bearer JWT)")
        });

        let auth_user_input = cx.new(|cx| InputState::new(window, cx).placeholder("Usuario"));

        let auth_pass_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Contraseña")
                .masked(true)
        });

        let auth_key_name_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("Nombre de la clave (ej. X-API-Key)")
        });

        let auth_key_val_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Valor de la clave"));

        let sub_url = cx.subscribe_in(
            &url_input,
            window,
            |this, _, ev: &InputEvent, _window, cx| {
                if let InputEvent::Change = ev {
                    let val = this.url_input.read(cx).value().to_string();
                    if let Some(req) = &mut this.request {
                        req.url = val.clone();
                    }
                    this.is_dirty = true;
                    cx.emit(EditorEvent::UrlChanged(val));
                    cx.emit(EditorEvent::RequestModified);
                    cx.notify();
                }
            },
        );

        let sub_body = cx.subscribe_in(
            &body_input,
            window,
            |this, _, ev: &InputEvent, _window, cx| {
                if let InputEvent::Change = ev {
                    let val = this.body_input.read(cx).value().to_string();
                    if let Some(req) = &mut this.request {
                        match this.selected_body_format {
                            BodyFormat::Json => req.body = Body::Json { content: val },
                            BodyFormat::Text => {
                                req.body = Body::Raw {
                                    content: val,
                                    content_type: "text/plain".to_string(),
                                }
                            }
                            BodyFormat::Xml => {
                                req.body = Body::Raw {
                                    content: val,
                                    content_type: "application/xml".to_string(),
                                }
                            }
                            _ => {}
                        }
                    }
                    this.is_dirty = true;
                    cx.emit(EditorEvent::RequestModified);
                    cx.notify();
                }
            },
        );

        Self {
            request: None,
            active_tab: EditorTab::Body,
            selected_body_format: BodyFormat::Json,
            url_input,
            body_input,
            graphql_vars_input,
            auth_token_input,
            auth_user_input,
            auth_pass_input,
            auth_key_name_input,
            auth_key_val_input,
            active_environment: None,
            is_dirty: false,
            _subscriptions: vec![sub_url, sub_body],
        }
    }

    pub fn set_active_environment(
        &mut self,
        env: Option<kestrel_core::Environment>,
        cx: &mut Context<Self>,
    ) {
        self.active_environment = env;
        cx.notify();
    }

    pub fn set_request(&mut self, req: Request, window: &mut Window, cx: &mut Context<Self>) {
        let url_str = req.url.clone();
        self.url_input.update(cx, |input, cx| {
            input.set_value(url_str, window, cx);
        });

        // Determine Body format and populate body_input
        match &req.body {
            Body::None => {
                self.selected_body_format = BodyFormat::None;
                self.body_input.update(cx, |input, cx| {
                    input.set_value(String::new(), window, cx);
                });
            }
            Body::Json { content } => {
                self.selected_body_format = BodyFormat::Json;
                let c = content.clone();
                self.body_input.update(cx, |input, cx| {
                    input.set_value(c, window, cx);
                });
            }
            Body::Raw {
                content,
                content_type,
            } => {
                if content_type.to_lowercase().contains("xml") {
                    self.selected_body_format = BodyFormat::Xml;
                } else {
                    self.selected_body_format = BodyFormat::Text;
                }
                let c = content.clone();
                self.body_input.update(cx, |input, cx| {
                    input.set_value(c, window, cx);
                });
            }
            Body::UrlEncoded { .. } => {
                self.selected_body_format = BodyFormat::FormUrlEncoded;
            }
            Body::FormData { .. } => {
                self.selected_body_format = BodyFormat::Multipart;
            }
            Body::Binary { file_path } => {
                self.selected_body_format = BodyFormat::Binary;
                let path = file_path.clone();
                self.body_input.update(cx, |input, cx| {
                    input.set_value(path, window, cx);
                });
            }
            Body::GraphQL { query, variables } => {
                self.selected_body_format = BodyFormat::GraphQL;
                let q = query.clone();
                self.body_input.update(cx, |input, cx| {
                    input.set_value(q, window, cx);
                });
                if let Some(vars) = variables {
                    let v = vars.clone();
                    self.graphql_vars_input.update(cx, |input, cx| {
                        input.set_value(v, window, cx);
                    });
                }
            }
        }

        // Populate Auth inputs
        match &req.auth {
            Auth::None => {}
            Auth::Bearer { token } => {
                let t = token.clone();
                self.auth_token_input.update(cx, |input, cx| {
                    input.set_value(t, window, cx);
                });
            }
            Auth::Basic { username, password } => {
                let u = username.clone();
                let p = password.clone();
                self.auth_user_input.update(cx, |input, cx| {
                    input.set_value(u, window, cx);
                });
                self.auth_pass_input.update(cx, |input, cx| {
                    input.set_value(p, window, cx);
                });
            }
            Auth::ApiKey { key, value, .. } => {
                let k = key.clone();
                let v = value.clone();
                self.auth_key_name_input.update(cx, |input, cx| {
                    input.set_value(k, window, cx);
                });
                self.auth_key_val_input.update(cx, |input, cx| {
                    input.set_value(v, window, cx);
                });
            }
        }

        self.request = Some(req);
        self.is_dirty = false;
        cx.notify();
    }

    pub fn current_request(&self) -> Option<&Request> {
        self.request.as_ref()
    }

    /// Builds a fresh `Request` reflecting all current values in the URL input,
    /// body input, headers, query parameters, and auth inputs.
    pub fn build_current_request(&self, cx: &App) -> Option<Request> {
        let mut req = self.request.clone()?;
        req.url = self.url_input.read(cx).value().to_string();

        let body_val = self.body_input.read(cx).value().to_string();
        req.body = match self.selected_body_format {
            BodyFormat::None => Body::None,
            BodyFormat::Json => Body::Json { content: body_val },
            BodyFormat::Text => Body::Raw {
                content: body_val,
                content_type: "text/plain".to_string(),
            },
            BodyFormat::Xml => Body::Raw {
                content: body_val,
                content_type: "application/xml".to_string(),
            },
            BodyFormat::FormUrlEncoded => match &req.body {
                Body::UrlEncoded { entries } => Body::UrlEncoded {
                    entries: entries.clone(),
                },
                _ => Body::UrlEncoded {
                    entries: Vec::new(),
                },
            },
            BodyFormat::Multipart => match &req.body {
                Body::FormData { entries } => Body::FormData {
                    entries: entries.clone(),
                },
                _ => Body::FormData {
                    entries: Vec::new(),
                },
            },
            BodyFormat::Binary => Body::Binary {
                file_path: body_val,
            },
            BodyFormat::GraphQL => {
                let vars_str = self.graphql_vars_input.read(cx).value().to_string();
                let vars = if vars_str.trim().is_empty() {
                    None
                } else {
                    Some(vars_str)
                };
                Body::GraphQL {
                    query: body_val,
                    variables: vars,
                }
            }
        };

        // Sync auth fields from inputs
        req.auth = match &req.auth {
            Auth::None => Auth::None,
            Auth::Bearer { .. } => Auth::Bearer {
                token: self.auth_token_input.read(cx).value().to_string(),
            },
            Auth::Basic { .. } => Auth::Basic {
                username: self.auth_user_input.read(cx).value().to_string(),
                password: self.auth_pass_input.read(cx).value().to_string(),
            },
            Auth::ApiKey { location, .. } => Auth::ApiKey {
                key: self.auth_key_name_input.read(cx).value().to_string(),
                value: self.auth_key_val_input.read(cx).value().to_string(),
                location: *location,
            },
        };

        Some(req)
    }

    pub fn is_dirty(&self) -> bool {
        self.is_dirty
    }

    pub fn mark_saved(&mut self, cx: &mut Context<Self>) {
        self.is_dirty = false;
        cx.notify();
    }

    fn toggle_method(&mut self, cx: &mut Context<Self>) {
        if let Some(req) = &mut self.request {
            req.method = match req.method {
                HttpMethod::GET => HttpMethod::POST,
                HttpMethod::POST => HttpMethod::PUT,
                HttpMethod::PUT => HttpMethod::DELETE,
                HttpMethod::DELETE => HttpMethod::PATCH,
                HttpMethod::PATCH => HttpMethod::HEAD,
                HttpMethod::HEAD => HttpMethod::OPTIONS,
                HttpMethod::OPTIONS => HttpMethod::GET,
            };
            let m = req.method;
            self.is_dirty = true;
            cx.emit(EditorEvent::MethodChanged(m));
            cx.emit(EditorEvent::RequestModified);
            cx.notify();
        }
    }

    fn set_body_format(&mut self, format: BodyFormat, cx: &mut Context<Self>) {
        self.selected_body_format = format;
        let body_content = self.body_input.read(cx).value().to_string();

        if let Some(req) = &mut self.request {
            req.body = match format {
                BodyFormat::None => Body::None,
                BodyFormat::Json => Body::Json {
                    content: if body_content.trim().is_empty() {
                        "{\n  \n}".to_string()
                    } else {
                        body_content
                    },
                },
                BodyFormat::Text => Body::Raw {
                    content: body_content,
                    content_type: "text/plain".to_string(),
                },
                BodyFormat::Xml => Body::Raw {
                    content: body_content,
                    content_type: "application/xml".to_string(),
                },
                BodyFormat::FormUrlEncoded => Body::UrlEncoded {
                    entries: vec![KeyValuePair {
                        key: "clave".to_string(),
                        value: "valor".to_string(),
                        enabled: true,
                    }],
                },
                BodyFormat::Multipart => Body::FormData {
                    entries: vec![FormEntry {
                        key: "campo".to_string(),
                        value: FormValue::Text {
                            text: "valor".to_string(),
                        },
                        enabled: true,
                    }],
                },
                BodyFormat::Binary => Body::Binary {
                    file_path: body_content,
                },
                BodyFormat::GraphQL => Body::GraphQL {
                    query: body_content,
                    variables: None,
                },
            };
        }

        self.is_dirty = true;
        cx.emit(EditorEvent::RequestModified);
        cx.notify();
    }

    // Add query parameter
    fn add_query_param(&mut self, cx: &mut Context<Self>) {
        if let Some(req) = &mut self.request {
            req.params.push(QueryParam {
                key: String::new(),
                value: String::new(),
                enabled: true,
            });
            self.is_dirty = true;
            cx.emit(EditorEvent::RequestModified);
            cx.notify();
        }
    }

    // Add header
    fn add_header_param(&mut self, cx: &mut Context<Self>) {
        if let Some(req) = &mut self.request {
            req.headers.push(HeaderParam {
                key: String::new(),
                value: String::new(),
                enabled: true,
            });
            self.is_dirty = true;
            cx.emit(EditorEvent::RequestModified);
            cx.notify();
        }
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
        let selected_format = self.selected_body_format;

        let req_name = self
            .request
            .as_ref()
            .map(|r| r.name.clone())
            .unwrap_or_else(|| "Nueva Solicitud".to_string());

        let params_count = self.request.as_ref().map(|r| r.params.len()).unwrap_or(0);
        let headers_count = self.request.as_ref().map(|r| r.headers.len()).unwrap_or(0);

        v_flex()
            .size_full()
            .p_4()
            .gap_4()
            // 1. Breadcrumbs / Title bar with Save status
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
                            )
                            .when(self.is_dirty, |this| {
                                this.child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(rgb(0xe06c1b))
                                        .child("• Modificado"),
                                )
                            }),
                    )
                    .child(
                        Button::new("save-req-btn")
                            .ghost()
                            .icon(Icon::new(IconName::FileText))
                            .label("Guardar")
                            .on_click(cx.listener(|this, _, _window, cx| {
                                this.is_dirty = false;
                                cx.emit(EditorEvent::SaveRequest);
                                cx.notify();
                            })),
                    ),
            )
            // 2. URL Bar: Method Selector Pill + URL Input + Send Button
            .child(
                h_flex()
                    .items_center()
                    .gap_2()
                    .w_full()
                    .child(
                        // Method Selector Pill (click to cycle method)
                        Button::new("method-pill-btn")
                            .ghost()
                            .on_click(cx.listener(|this, _, _window, cx| {
                                this.toggle_method(cx);
                            }))
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_1p5()
                                    .child(
                                        div()
                                            .font_weight(FontWeight::BOLD)
                                            .text_xs()
                                            .text_color(method_color(method))
                                            .child(format!("{:?}", method)),
                                    )
                                    .child(Icon::new(IconName::ChevronDown)),
                            ),
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
            // 3. Sub-tabs Navigation Bar: Path, Query, Headers, Body, Authentication
            .child(
                h_flex()
                    .items_center()
                    .gap_2()
                    .border_b_1()
                    .border_color(theme.border)
                    .pb_2()
                    .child(render_subtab_button(
                        "subtab-path",
                        "Path",
                        active_tab == EditorTab::Path,
                        cx.listener(|this, _, _, cx| {
                            this.active_tab = EditorTab::Path;
                            cx.notify();
                        }),
                    ))
                    .child(render_subtab_button(
                        "subtab-query",
                        &format!("Query {}", params_count),
                        active_tab == EditorTab::Query,
                        cx.listener(|this, _, _, cx| {
                            this.active_tab = EditorTab::Query;
                            cx.notify();
                        }),
                    ))
                    .child(render_subtab_button(
                        "subtab-headers",
                        &format!("Headers {}", headers_count),
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
            // 4. Tab Body Content Area
            .child(match active_tab {
                EditorTab::Body => self
                    .render_body_view(selected_format, cx)
                    .into_any_element(),
                EditorTab::Headers => self.render_headers_view(cx).into_any_element(),
                EditorTab::Query => self.render_query_view(cx).into_any_element(),
                EditorTab::Path => self.render_path_view(cx).into_any_element(),
                EditorTab::Authentication => self.render_auth_view(cx).into_any_element(),
            })
    }
}

impl RequestEditor {
    fn render_body_view(
        &self,
        selected_format: BodyFormat,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme();

        v_flex()
            .flex_1()
            .size_full()
            .gap_3()
            // Format Selector Bar (None, JSON, Text, XML, Form, Multipart, File, GraphQL)
            .child(
                h_flex()
                    .gap_1p5()
                    .text_xs()
                    .font_weight(FontWeight::MEDIUM)
                    .child(format_selector_button(
                        "fmt-none",
                        "None",
                        selected_format == BodyFormat::None,
                        cx.listener(|this, _, _, cx| this.set_body_format(BodyFormat::None, cx)),
                    ))
                    .child(format_selector_button(
                        "fmt-json",
                        "JSON",
                        selected_format == BodyFormat::Json,
                        cx.listener(|this, _, _, cx| this.set_body_format(BodyFormat::Json, cx)),
                    ))
                    .child(format_selector_button(
                        "fmt-text",
                        "Text",
                        selected_format == BodyFormat::Text,
                        cx.listener(|this, _, _, cx| this.set_body_format(BodyFormat::Text, cx)),
                    ))
                    .child(format_selector_button(
                        "fmt-xml",
                        "XML",
                        selected_format == BodyFormat::Xml,
                        cx.listener(|this, _, _, cx| this.set_body_format(BodyFormat::Xml, cx)),
                    ))
                    .child(format_selector_button(
                        "fmt-form",
                        "Form URL-Encoded",
                        selected_format == BodyFormat::FormUrlEncoded,
                        cx.listener(|this, _, _, cx| {
                            this.set_body_format(BodyFormat::FormUrlEncoded, cx)
                        }),
                    ))
                    .child(format_selector_button(
                        "fmt-multipart",
                        "Multipart",
                        selected_format == BodyFormat::Multipart,
                        cx.listener(|this, _, _, cx| {
                            this.set_body_format(BodyFormat::Multipart, cx)
                        }),
                    ))
                    .child(format_selector_button(
                        "fmt-binary",
                        "Archivo",
                        selected_format == BodyFormat::Binary,
                        cx.listener(|this, _, _, cx| this.set_body_format(BodyFormat::Binary, cx)),
                    ))
                    .child(format_selector_button(
                        "fmt-graphql",
                        "GraphQL",
                        selected_format == BodyFormat::GraphQL,
                        cx.listener(|this, _, _, cx| this.set_body_format(BodyFormat::GraphQL, cx)),
                    )),
            )
            // Body Input Canvas
            .child(
                div()
                    .flex_1()
                    .rounded_md()
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.background)
                    .p_3()
                    .overflow_y_scrollbar()
                    .child(match selected_format {
                        BodyFormat::None => div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .p_4()
                            .child("Esta solicitud no tiene cuerpo (Body: None)")
                            .into_any_element(),
                        BodyFormat::Json | BodyFormat::Text | BodyFormat::Xml => {
                            Input::new(&self.body_input)
                                .h_full()
                                .cleanable(false)
                                .into_any_element()
                        }
                        BodyFormat::FormUrlEncoded => {
                            self.render_key_value_form(cx).into_any_element()
                        }
                        BodyFormat::Multipart => self.render_multipart_form(cx).into_any_element(),
                        BodyFormat::Binary => v_flex()
                            .gap_2()
                            .p_2()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::BOLD)
                                    .child("Ruta del archivo:"),
                            )
                            .child(Input::new(&self.body_input).bordered(true))
                            .into_any_element(),
                        BodyFormat::GraphQL => v_flex()
                            .size_full()
                            .gap_3()
                            .child(div().flex_1().child(Input::new(&self.body_input).h_full()))
                            .child(
                                div()
                                    .h(px(120.))
                                    .border_t_1()
                                    .border_color(theme.border)
                                    .pt_2()
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(FontWeight::BOLD)
                                            .child("Variables (JSON):"),
                                    )
                                    .child(Input::new(&self.graphql_vars_input).h_full()),
                            )
                            .into_any_element(),
                    }),
            )
    }

    fn render_headers_view(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let headers = self
            .request
            .as_ref()
            .map(|r| r.headers.clone())
            .unwrap_or_default();

        v_flex()
            .flex_1()
            .size_full()
            .gap_3()
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::BOLD)
                            .child("Encabezados HTTP (Headers)"),
                    )
                    .child(
                        Button::new("add-hdr-btn")
                            .ghost()
                            .icon(Icon::new(IconName::Plus))
                            .label("Agregar Header")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.add_header_param(cx);
                            })),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .rounded_md()
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.background)
                    .p_3()
                    .overflow_y_scrollbar()
                    .child(if headers.is_empty() {
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .p_4()
                            .child("Sin encabezados configurados")
                    } else {
                        v_flex()
                            .gap_1p5()
                            .children(headers.into_iter().enumerate().map(|(ix, h)| {
                                h_flex()
                                    .gap_3()
                                    .items_center()
                                    .px_3()
                                    .py_2()
                                    .rounded_md()
                                    .bg(theme.muted.opacity(0.18))
                                    .child(
                                        div()
                                            .w(px(180.))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_xs()
                                            .child(h.key),
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .font_family("monospace")
                                            .text_xs()
                                            .child(h.value),
                                    )
                                    .child(
                                        Button::new(format!("del-hdr-{}", ix))
                                            .ghost()
                                            .icon(Icon::new(IconName::Close))
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                if let Some(req) = this
                                                    .request
                                                    .as_mut()
                                                    .filter(|r| ix < r.headers.len())
                                                {
                                                    req.headers.remove(ix);
                                                    this.is_dirty = true;
                                                    cx.notify();
                                                }
                                            })),
                                    )
                            }))
                    }),
            )
    }

    fn render_query_view(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let params = self
            .request
            .as_ref()
            .map(|r| r.params.clone())
            .unwrap_or_default();

        v_flex()
            .flex_1()
            .size_full()
            .gap_3()
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::BOLD)
                            .child("Parámetros de consulta (Query Params)"),
                    )
                    .child(
                        Button::new("add-param-btn")
                            .ghost()
                            .icon(Icon::new(IconName::Plus))
                            .label("Agregar Parámetro")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.add_query_param(cx);
                            })),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .rounded_md()
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.background)
                    .p_3()
                    .overflow_y_scrollbar()
                    .child(if params.is_empty() {
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .p_4()
                            .child("Sin parámetros de consulta")
                    } else {
                        v_flex()
                            .gap_1p5()
                            .children(params.into_iter().enumerate().map(|(ix, p)| {
                                h_flex()
                                    .gap_3()
                                    .items_center()
                                    .px_3()
                                    .py_2()
                                    .rounded_md()
                                    .bg(theme.muted.opacity(0.18))
                                    .child(
                                        div()
                                            .w(px(180.))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_xs()
                                            .child(format!("?{}", p.key)),
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .font_family("monospace")
                                            .text_xs()
                                            .child(p.value),
                                    )
                                    .child(
                                        Button::new(format!("del-param-{}", ix))
                                            .ghost()
                                            .icon(Icon::new(IconName::Close))
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                if let Some(req) = this
                                                    .request
                                                    .as_mut()
                                                    .filter(|r| ix < r.params.len())
                                                {
                                                    req.params.remove(ix);
                                                    this.is_dirty = true;
                                                    cx.notify();
                                                }
                                            })),
                                    )
                            }))
                    }),
            )
    }

    fn render_path_view(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let raw_url = self.url_input.read(cx).value();
        let resolved_url = if let Some(env) = &self.active_environment {
            kestrel_core::resolve_variables(&raw_url, &[env])
        } else {
            raw_url.to_string()
        };

        v_flex()
            .flex_1()
            .size_full()
            .p_4()
            .gap_3()
            .child(
                v_flex()
                    .gap_1()
                    .child(div().font_weight(FontWeight::BOLD).text_xs().child("Ruta URL cruda (con variables):"))
                    .child(
                        div()
                            .font_family("monospace")
                            .text_xs()
                            .p_3()
                            .rounded_md()
                            .bg(theme.muted.opacity(0.4))
                            .child(raw_url.to_string()),
                    ),
            )
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(div().font_weight(FontWeight::BOLD).text_xs().child("Ruta URL resuelta:"))
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(rgb(0x22c55e))
                                    .child(if self.active_environment.is_some() {
                                        "✓ Entorno activo aplicado"
                                    } else {
                                        "⚠ Sin entorno activo"
                                    }),
                            ),
                    )
                    .child(
                        div()
                            .font_family("monospace")
                            .text_xs()
                            .p_3()
                            .rounded_md()
                            .border_1()
                            .border_color(theme.border)
                            .bg(theme.background)
                            .child(resolved_url),
                    ),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child("Las variables de entorno en la URL (ej. {{baseUrl}}) se resuelven dinámicamente con los valores del entorno seleccionado."),
            )
    }

    fn render_auth_view(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let auth = self
            .request
            .as_ref()
            .map(|r| r.auth.clone())
            .unwrap_or(Auth::None);

        v_flex()
            .flex_1()
            .size_full()
            .gap_3()
            .child(
                h_flex()
                    .gap_1p5()
                    .text_xs()
                    .font_weight(FontWeight::MEDIUM)
                    .child(format_selector_button(
                        "auth-none",
                        "None",
                        matches!(auth, Auth::None),
                        cx.listener(|this, _, _, cx| {
                            if let Some(req) = &mut this.request {
                                req.auth = Auth::None;
                                this.is_dirty = true;
                                cx.notify();
                            }
                        }),
                    ))
                    .child(format_selector_button(
                        "auth-bearer",
                        "Bearer Token",
                        matches!(auth, Auth::Bearer { .. }),
                        cx.listener(|this, _, _, cx| {
                            if let Some(req) = &mut this.request {
                                req.auth = Auth::Bearer {
                                    token: String::new(),
                                };
                                this.is_dirty = true;
                                cx.notify();
                            }
                        }),
                    ))
                    .child(format_selector_button(
                        "auth-basic",
                        "Basic Auth",
                        matches!(auth, Auth::Basic { .. }),
                        cx.listener(|this, _, _, cx| {
                            if let Some(req) = &mut this.request {
                                req.auth = Auth::Basic {
                                    username: String::new(),
                                    password: String::new(),
                                };
                                this.is_dirty = true;
                                cx.notify();
                            }
                        }),
                    ))
                    .child(format_selector_button(
                        "auth-apikey",
                        "API Key",
                        matches!(auth, Auth::ApiKey { .. }),
                        cx.listener(|this, _, _, cx| {
                            if let Some(req) = &mut this.request {
                                req.auth = Auth::ApiKey {
                                    key: "X-API-Key".to_string(),
                                    value: String::new(),
                                    location: ApiKeyLocation::Header,
                                };
                                this.is_dirty = true;
                                cx.notify();
                            }
                        }),
                    )),
            )
            .child(
                div()
                    .flex_1()
                    .rounded_md()
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.background)
                    .p_4()
                    .child(match auth {
                        Auth::None => div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child("Sin autenticación configurada."),
                        Auth::Bearer { .. } => v_flex()
                            .gap_2()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::BOLD)
                                    .child("Token Bearer:"),
                            )
                            .child(Input::new(&self.auth_token_input).bordered(true)),
                        Auth::Basic { .. } => v_flex()
                            .gap_3()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::BOLD)
                                    .child("Usuario:"),
                            )
                            .child(Input::new(&self.auth_user_input).bordered(true))
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::BOLD)
                                    .child("Contraseña:"),
                            )
                            .child(Input::new(&self.auth_pass_input).bordered(true)),
                        Auth::ApiKey { .. } => v_flex()
                            .gap_3()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::BOLD)
                                    .child("Nombre de Header o Parámetro:"),
                            )
                            .child(Input::new(&self.auth_key_name_input).bordered(true))
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::BOLD)
                                    .child("Valor:"),
                            )
                            .child(Input::new(&self.auth_key_val_input).bordered(true)),
                    }),
            )
    }

    fn render_key_value_form(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        let entries = if let Some(Request {
            body: Body::UrlEncoded { entries },
            ..
        }) = &self.request
        {
            entries.clone()
        } else {
            Vec::new()
        };

        v_flex().gap_2().children(entries.into_iter().map(|e| {
            h_flex()
                .gap_2()
                .items_center()
                .child(div().w(px(140.)).child(e.key))
                .child(div().flex_1().child(e.value))
        }))
    }

    fn render_multipart_form(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        let entries = if let Some(Request {
            body: Body::FormData { entries },
            ..
        }) = &self.request
        {
            entries.clone()
        } else {
            Vec::new()
        };

        v_flex().gap_2().children(entries.into_iter().map(|e| {
            let val_str = match e.value {
                FormValue::Text { text } => text,
                FormValue::File { file_path } => format!("[Archivo] {}", file_path),
            };
            h_flex()
                .gap_2()
                .items_center()
                .child(div().w(px(140.)).child(e.key))
                .child(div().flex_1().child(val_str))
        }))
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

fn format_selector_button(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_editor_tab_variants() {
        assert_eq!(EditorTab::Body, EditorTab::Body);
        assert_ne!(EditorTab::Body, EditorTab::Headers);
    }

    #[test]
    fn test_body_format_variants() {
        assert_eq!(BodyFormat::Json, BodyFormat::Json);
        assert_ne!(BodyFormat::Json, BodyFormat::GraphQL);
    }

    #[test]
    fn test_method_colors() {
        assert_eq!(method_color(HttpMethod::GET), rgb(0x10b981));
        assert_eq!(method_color(HttpMethod::POST), rgb(0x3b82f6));
    }
}
