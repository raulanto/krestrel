use std::sync::Arc;
use std::time::Instant;

use ely_gpui_component::buttons::{Button, ButtonVariant};
use ely_gpui_component::editor::{CodeEditor, FindOptions, FindWidget, LineNumbers, find_all};
use ely_gpui_component::forms::{InputEvent, TextInput};
use ely_gpui_component::primitives::{Icon, IconName};
use ely_gpui_component::theme::ControlSize;
use gpui::{
    AppContext, Context, Entity, EventEmitter, Focusable as _, FontWeight, InteractiveElement as _,
    IntoElement, ParentElement as _, Render, StatefulInteractiveElement as _, Styled, Subscription,
    Window, div, px,
};
use kestrel_http::{
    ContentTypeCategory, HttpResponse, Insight, RequestError, ResponseData, ResponseIntelligence,
    ResponseState, decode_body, detect_content_type, format_byte_size, format_pretty_json,
    format_pretty_xml,
};

use super::headers_view::HeadersView;
use super::raw_view::RawView;
use super::states::{EmptyBodyState, FailedState, IdleState, LoadingState};
use super::status_bar::StatusBar;
use crate::theme::{ThemeExt as _, h_flex, v_flex};

const LARGE_BODY_THRESHOLD_BYTES: usize = 5 * 1024 * 1024; // 5 MB

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseTab {
    Pretty,
    Raw,
    Headers,
    Insights,
}

#[derive(Debug, Clone)]
pub enum ResponseEvent {
    Retry,
    Cancel,
    SaveToFile,
    CopyBody,
}

pub struct ResponsePanel {
    state: ResponseState,
    active_tab: ResponseTab,
    is_word_wrap: bool,
    force_format_large: bool,
    code_editor: Entity<CodeEditor>,
    find_input: Entity<TextInput>,
    find_options: FindOptions,
    find_matches: Vec<std::ops::Range<usize>>,
    find_error: Option<String>,
    current_match_index: Option<usize>,
    is_search_open: bool,
    insights: Vec<Insight>,
    intelligence: ResponseIntelligence,
    _subscriptions: Vec<Subscription>,
}

impl ResponsePanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let code_editor = cx.new(|cx| {
            CodeEditor::new("", window, cx)
                .language("JSON")
                .line_numbers(LineNumbers::Absolute)
                .read_only()
        });

        let find_input = cx.new(|cx| TextInput::new(window, cx).placeholder("Buscar..."));

        let sub_find = cx.subscribe_in(&find_input, window, {
            move |this, _, ev: &InputEvent, _window, cx| match ev {
                InputEvent::Changed => {
                    this.update_search(cx);
                }
                InputEvent::Submit => {
                    this.step_match(true, cx);
                }
                _ => {}
            }
        });

        Self {
            state: ResponseState::Idle,
            active_tab: ResponseTab::Pretty,
            is_word_wrap: false,
            force_format_large: false,
            code_editor,
            find_input,
            find_options: FindOptions::default(),
            find_matches: Vec::new(),
            find_error: None,
            current_match_index: None,
            is_search_open: false,
            insights: Vec::new(),
            intelligence: ResponseIntelligence::new(),
            _subscriptions: vec![sub_find],
        }
    }

    pub fn set_loading(&mut self, is_loading: bool, cx: &mut Context<Self>) {
        if is_loading {
            self.state = ResponseState::Loading {
                started: Instant::now(),
            };
        } else if matches!(self.state, ResponseState::Loading { .. }) {
            self.state = ResponseState::Idle;
        }
        cx.notify();
    }

    pub fn set_response(&mut self, resp: HttpResponse, cx: &mut Context<Self>) {
        let body_str = String::from_utf8_lossy(&resp.body);
        let insights = self.intelligence.analyze(&body_str);

        self.insights = insights;
        self.state = ResponseState::Done(Arc::new(resp));
        self.force_format_large = false;
        self.update_editor_text(cx);
        cx.notify();
    }

    pub fn set_error(&mut self, error: RequestError, cx: &mut Context<Self>) {
        self.state = ResponseState::Failed(error);
        cx.notify();
    }

    pub fn set_cancelled(&mut self, cx: &mut Context<Self>) {
        self.state = ResponseState::Cancelled;
        cx.notify();
    }

    pub fn current_response(&self) -> Option<ResponseData> {
        if let ResponseState::Done(ref arc_resp) = self.state {
            Some((**arc_resp).clone())
        } else {
            None
        }
    }

    pub fn toggle_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.is_search_open = !self.is_search_open;
        if self.is_search_open {
            let focus = self.find_input.read(cx).focus_handle(cx);
            window.focus(&focus, cx);
            self.update_search(cx);
        } else {
            self.find_matches.clear();
            self.find_error = None;
            self.current_match_index = None;
            self.code_editor.update(cx, |ed, cx| {
                ed.set_backgrounds(Vec::new(), cx);
            });
            cx.notify();
        }
    }

    pub fn toggle_word_wrap(&mut self, cx: &mut Context<Self>) {
        self.is_word_wrap = !self.is_word_wrap;
        cx.notify();
    }

    fn update_editor_text(&mut self, cx: &mut Context<Self>) {
        if let ResponseState::Done(ref resp) = self.state {
            let (decoded, _lossy) = decode_body(&resp.body, resp.content_type.as_deref());
            let category = detect_content_type(resp.content_type.as_deref(), &resp.body);

            let formatted_text = match (self.active_tab, category) {
                (ResponseTab::Pretty, ContentTypeCategory::Json) => {
                    if let Ok(pretty) = format_pretty_json(&decoded) {
                        pretty
                    } else {
                        decoded.clone()
                    }
                }
                (ResponseTab::Pretty, ContentTypeCategory::Xml | ContentTypeCategory::Html) => {
                    format_pretty_xml(&decoded)
                }
                _ => decoded.clone(),
            };

            self.code_editor.update(cx, |ed, cx| {
                ed.set_text(formatted_text, cx);
                ed.set_read_only(true, cx);
            });

            if self.is_search_open {
                self.update_search(cx);
            }
        }
    }

    pub fn update_search(&mut self, cx: &mut Context<Self>) {
        let query = self.find_input.read(cx).text().to_string();
        if query.is_empty() {
            self.find_matches.clear();
            self.find_error = None;
            self.current_match_index = None;
            self.code_editor.update(cx, |ed, cx| {
                ed.set_backgrounds(Vec::new(), cx);
            });
            cx.notify();
            return;
        }

        let text = self.code_editor.read(cx).text().to_string();
        match find_all(&text, &query, self.find_options) {
            Ok(ranges) => {
                self.find_error = None;
                let count = ranges.len();
                if count > 0 {
                    let curr = self.current_match_index.unwrap_or(0).min(count - 1);
                    self.current_match_index = Some(curr);
                    let target = ranges[curr].clone();
                    let colors = cx.theme().colors.clone();
                    let bg_ranges: Vec<_> = ranges
                        .iter()
                        .enumerate()
                        .map(|(ix, r)| {
                            let wash = if ix == curr {
                                colors.warning.opacity(0.45)
                            } else {
                                colors.warning.opacity(0.2)
                            };
                            (r.clone(), wash)
                        })
                        .collect();
                    self.code_editor.update(cx, |ed, cx| {
                        ed.set_backgrounds(bg_ranges, cx);
                        ed.select([target], cx);
                    });
                } else {
                    self.current_match_index = None;
                    self.code_editor.update(cx, |ed, cx| {
                        ed.set_backgrounds(Vec::new(), cx);
                    });
                }
                self.find_matches = ranges;
            }
            Err(err) => {
                self.find_error = Some(err);
                self.find_matches.clear();
                self.current_match_index = None;
                self.code_editor.update(cx, |ed, cx| {
                    ed.set_backgrounds(Vec::new(), cx);
                });
            }
        }
        cx.notify();
    }

    pub fn step_match(&mut self, is_next: bool, cx: &mut Context<Self>) {
        let total = self.find_matches.len();
        if total == 0 {
            return;
        }
        let curr = self.current_match_index.unwrap_or(0);
        let next_ix = if is_next {
            (curr + 1) % total
        } else {
            (curr + total - 1) % total
        };
        self.current_match_index = Some(next_ix);
        let target = self.find_matches[next_ix].clone();
        let colors = cx.theme().colors.clone();
        let bg_ranges: Vec<_> = self
            .find_matches
            .iter()
            .enumerate()
            .map(|(ix, r)| {
                let wash = if ix == next_ix {
                    colors.warning.opacity(0.45)
                } else {
                    colors.warning.opacity(0.2)
                };
                (r.clone(), wash)
            })
            .collect();
        self.code_editor.update(cx, |ed, cx| {
            ed.set_backgrounds(bg_ranges, cx);
            ed.select([target], cx);
        });
        cx.notify();
    }
}

impl EventEmitter<ResponseEvent> for ResponsePanel {}

impl Render for ResponsePanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let active_tab = self.active_tab;
        let insights_count = self.insights.len();
        let resp_opt = self.current_response();

        v_flex()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .size_full()
            .bg(colors.bg)
            .border_l_1()
            .border_color(colors.border)
            .overflow_hidden()
            // 1. Top Bar: View Selector (Pretty | Raw | Headers | Insights)
            .child(
                h_flex()
                    .h(px(40.))
                    .min_h(px(40.))
                    .flex_shrink_0()
                    .px_3()
                    .items_center()
                    .justify_between()
                    .border_b_1()
                    .border_color(colors.border)
                    .bg(colors.surface)
                    .child(
                        h_flex()
                            .items_center()
                            .gap_1()
                            // Pretty Tab
                            .child(
                                Button::new("tab-pretty", "Pretty")
                                    .variant(if active_tab == ResponseTab::Pretty {
                                        ButtonVariant::Secondary
                                    } else {
                                        ButtonVariant::Ghost
                                    })
                                    .size(ControlSize::Sm)
                                    .on_click(cx.listener(|this, _, _window, cx| {
                                        this.active_tab = ResponseTab::Pretty;
                                        this.update_editor_text(cx);
                                        cx.notify();
                                    })),
                            )
                            // Raw Tab
                            .child(
                                Button::new("tab-raw", "Raw")
                                    .variant(if active_tab == ResponseTab::Raw {
                                        ButtonVariant::Secondary
                                    } else {
                                        ButtonVariant::Ghost
                                    })
                                    .size(ControlSize::Sm)
                                    .on_click(cx.listener(|this, _, _window, cx| {
                                        this.active_tab = ResponseTab::Raw;
                                        this.update_editor_text(cx);
                                        cx.notify();
                                    })),
                            )
                            // Headers Tab
                            .child(
                                Button::new("tab-headers", "Headers")
                                    .variant(if active_tab == ResponseTab::Headers {
                                        ButtonVariant::Secondary
                                    } else {
                                        ButtonVariant::Ghost
                                    })
                                    .size(ControlSize::Sm)
                                    .on_click(cx.listener(|this, _, _window, cx| {
                                        this.active_tab = ResponseTab::Headers;
                                        cx.notify();
                                    })),
                            )
                            // Insights Tab (if any)
                            .children(if insights_count > 0 {
                                Some(
                                    Button::new(
                                        "tab-insights",
                                        format!("Insights ({})", insights_count),
                                    )
                                    .variant(if active_tab == ResponseTab::Insights {
                                        ButtonVariant::Secondary
                                    } else {
                                        ButtonVariant::Ghost
                                    })
                                    .size(ControlSize::Sm)
                                    .on_click(cx.listener(
                                        |this, _, _window, cx| {
                                            this.active_tab = ResponseTab::Insights;
                                            cx.notify();
                                        },
                                    )),
                                )
                            } else {
                                None
                            }),
                    )
                    // Optional point of extension for Diagram/Node Visualizer
                    .child(
                        h_flex().items_center().gap_2().child(
                            div()
                                .text_xs()
                                .font_family("monospace")
                                .text_color(colors.fg_muted)
                                .child("Respuesta"),
                        ),
                    ),
            )
            // 2. Main Content Canvas
            .child(
                v_flex()
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .p_3()
                    .overflow_hidden()
                    .child(self.render_main_content(cx)),
            )
            // 3. Bottom Status Bar with Metrics & Controls
            .child(StatusBar::new(
                resp_opt.clone(),
                self.is_word_wrap,
                self.is_search_open,
                cx.listener(|this, _, _window, cx| {
                    if let Some(resp) = this.current_response() {
                        let text = String::from_utf8_lossy(&resp.body).to_string();
                        cx.write_to_clipboard(gpui::ClipboardItem::new_string(text));
                    }
                    cx.emit(ResponseEvent::CopyBody);
                }),
                cx.listener(|_this, _, _window, cx| {
                    cx.emit(ResponseEvent::SaveToFile);
                }),
                cx.listener(|this, _, _window, cx| {
                    this.toggle_word_wrap(cx);
                }),
                cx.listener(|this, _, window, cx| {
                    this.toggle_search(window, cx);
                }),
            ))
    }
}

impl ResponsePanel {
    fn render_main_content(&self, cx: &mut Context<Self>) -> impl IntoElement {
        match &self.state {
            ResponseState::Idle => IdleState.into_any_element(),
            ResponseState::Loading { started } => LoadingState::new(
                *started,
                cx.listener(|this, _, _window, cx| {
                    this.set_cancelled(cx);
                    cx.emit(ResponseEvent::Cancel);
                }),
            )
            .into_any_element(),
            ResponseState::Failed(err) => FailedState::new(
                err.clone(),
                cx.listener(|_this, _, _window, cx| {
                    cx.emit(ResponseEvent::Retry);
                }),
            )
            .into_any_element(),
            ResponseState::Cancelled => {
                let theme = cx.theme();
                let colors = &theme.colors;
                v_flex()
                    .flex_1()
                    .items_center()
                    .justify_center()
                    .gap_2()
                    .p_8()
                    .child(Icon::new(IconName::X).color(colors.fg_muted))
                    .child(
                        div()
                            .text_sm()
                            .text_color(colors.fg_muted)
                            .child("Solicitud cancelada por el usuario"),
                    )
                    .into_any_element()
            }
            ResponseState::Done(resp) => {
                if resp.is_empty() {
                    return EmptyBodyState.into_any_element();
                }

                let category = detect_content_type(resp.content_type.as_deref(), &resp.body);

                // Handle binary or image content in Pretty/Raw tab
                if (self.active_tab == ResponseTab::Pretty || self.active_tab == ResponseTab::Raw)
                    && matches!(
                        category,
                        ContentTypeCategory::Image | ContentTypeCategory::Binary
                    )
                {
                    return RawView::new(
                        resp.body.clone(),
                        resp.content_type.clone(),
                        self.is_word_wrap,
                        None,
                        cx.listener(|_this, _, _window, cx| {
                            cx.emit(ResponseEvent::SaveToFile);
                        }),
                    )
                    .into_any_element();
                }

                // Check large body threshold
                if resp.body.len() > LARGE_BODY_THRESHOLD_BYTES
                    && !self.force_format_large
                    && self.active_tab == ResponseTab::Pretty
                {
                    let theme = cx.theme();
                    let colors = &theme.colors;
                    let size_str = format_byte_size(resp.body.len());
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
                                        .on_click(cx.listener(|this, _, _window, cx| {
                                            this.force_format_large = true;
                                            this.update_editor_text(cx);
                                            cx.notify();
                                        })),
                                )
                                .child(
                                    Button::new("save-large-file-btn", "Guardar a archivo")
                                        .variant(ButtonVariant::Ghost)
                                        .on_click(cx.listener(|_this, _, _window, cx| {
                                            cx.emit(ResponseEvent::SaveToFile);
                                        })),
                                ),
                        )
                        .into_any_element();
                }

                match self.active_tab {
                    ResponseTab::Pretty | ResponseTab::Raw => {
                        let find_widget = if self.is_search_open {
                            let panel = cx.entity().clone();
                            let current_idx = self.current_match_index;
                            let total_matches = self.find_matches.len();
                            let mut widget = FindWidget::new(
                                "response-find-widget",
                                &self.find_input,
                                current_idx,
                                total_matches,
                            )
                            .options(self.find_options)
                            .on_options({
                                let panel = panel.clone();
                                move |opts, _window, cx| {
                                    panel.update(cx, |this, cx| {
                                        this.find_options = opts;
                                        this.update_search(cx);
                                    });
                                }
                            })
                            .on_step({
                                let panel = panel.clone();
                                move |is_next, _window, cx| {
                                    panel.update(cx, |this, cx| {
                                        this.step_match(is_next, cx);
                                    });
                                }
                            })
                            .on_close({
                                let panel = panel.clone();
                                move |_window, cx| {
                                    panel.update(cx, |this, cx| {
                                        this.is_search_open = false;
                                        this.find_matches.clear();
                                        this.find_error = None;
                                        this.current_match_index = None;
                                        this.code_editor.update(cx, |ed, cx| {
                                            ed.set_backgrounds(Vec::new(), cx);
                                        });
                                        cx.notify();
                                    });
                                }
                            });

                            if let Some(ref err) = self.find_error {
                                widget = widget.error(err.clone());
                            }

                            Some(div().absolute().top(px(8.)).right(px(16.)).child(widget))
                        } else {
                            None
                        };

                        div()
                            .relative()
                            .flex_1()
                            .min_h_0()
                            .w_full()
                            .overflow_hidden()
                            .child(self.code_editor.clone())
                            .children(find_widget)
                            .into_any_element()
                    }
                    ResponseTab::Headers => HeadersView::new(
                        resp.headers.clone(),
                        resp.url_final.clone(),
                        resp.http_version.clone(),
                    )
                    .into_any_element(),
                    ResponseTab::Insights => {
                        let theme = cx.theme();
                        let colors = &theme.colors;
                        v_flex()
                            .id("insights-panel-scroll")
                            .flex_1()
                            .min_h_0()
                            .w_full()
                            .overflow_y_scroll()
                            .child(
                                v_flex()
                                    .gap_3()
                                    .children(self.insights.iter().enumerate().map(
                                        |(idx, insight)| {
                                            div()
                                                .id(format!("insight-item-{}", idx))
                                                .p_3()
                                                .rounded_md()
                                                .bg(colors.surface)
                                                .border_1()
                                                .border_color(colors.border)
                                                .child(
                                                    h_flex()
                                                        .gap_2()
                                                        .items_center()
                                                        .child(
                                                            Icon::new(IconName::Globe)
                                                                .color(colors.accent),
                                                        )
                                                        .child(
                                                            div()
                                                                .font_weight(FontWeight::MEDIUM)
                                                                .text_xs()
                                                                .text_color(colors.fg)
                                                                .child(format!(
                                                                    "{:?}",
                                                                    insight.kind
                                                                )),
                                                        ),
                                                )
                                        },
                                    )),
                            )
                            .into_any_element()
                    }
                }
            }
        }
    }
}
