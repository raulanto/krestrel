use std::collections::HashSet;

use gpui::{
    AppContext, Context, Entity, FontWeight, IntoElement, ParentElement as _, Render, SharedString,
    Styled, Subscription, Window, div, prelude::FluentBuilder as _, px, rgb,
};
use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Input, InputEvent, InputState},
    scroll::ScrollableElement as _,
    v_flex,
};
use kestrel_core::{Collection, CollectionItem, Folder, HttpMethod};

use super::search::filter_collection_items;

pub enum SidebarEvent {
    SelectRequest(String),
}

pub struct Sidebar {
    collection: Option<Collection>,
    search_input: Entity<InputState>,
    search_query: String,
    selected_request_id: Option<String>,
    collapsed_folders: HashSet<String>,
    _subscriptions: Vec<Subscription>,
}

impl Sidebar {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search_input = cx
            .new(|cx| InputState::new(window, cx).placeholder("Buscar solicitudes o carpetas..."));

        let sub = cx.subscribe_in(&search_input, window, {
            let search_input = search_input.clone();
            move |this, _, ev: &InputEvent, _window, cx| {
                if let InputEvent::Change = ev {
                    let val = search_input.read(cx).value();
                    this.search_query = val.to_string();
                    cx.notify();
                }
            }
        });

        Self {
            collection: None,
            search_input,
            search_query: String::new(),
            selected_request_id: None,
            collapsed_folders: HashSet::new(),
            _subscriptions: vec![sub],
        }
    }

    pub fn set_collection(&mut self, collection: Collection, cx: &mut Context<Self>) {
        self.collection = Some(collection);
        cx.notify();
    }

    pub fn collection(&self) -> Option<&Collection> {
        self.collection.as_ref()
    }

    pub fn select_request(&mut self, request_id: String, cx: &mut Context<Self>) {
        self.selected_request_id = Some(request_id);
        cx.notify();
    }

    fn toggle_folder(&mut self, folder_id: &str, cx: &mut Context<Self>) {
        if self.collapsed_folders.contains(folder_id) {
            self.collapsed_folders.remove(folder_id);
        } else {
            self.collapsed_folders.insert(folder_id.to_string());
        }
        cx.notify();
    }
}

impl gpui::EventEmitter<SidebarEvent> for Sidebar {}

impl Render for Sidebar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let query = self.search_query.clone();

        let collection_name = self
            .collection
            .as_ref()
            .map(|c| c.name.clone())
            .unwrap_or_else(|| "Sin colección".to_string());

        v_flex()
            .w(px(280.))
            .h_full()
            .border_r_1()
            .border_color(theme.border)
            .bg(theme.background)
            // 1. Sidebar Header: Collection title & Quick Search input
            .child(
                v_flex()
                    .p_3()
                    .gap_2p5()
                    .border_b_1()
                    .border_color(theme.border)
                    .bg(theme.muted.opacity(0.12))
                    .child(
                        h_flex().items_center().justify_between().w_full().child(
                            h_flex()
                                .items_center()
                                .gap_2()
                                .child(Icon::new(IconName::FolderOpen).text_color(rgb(0xe06c1b)))
                                .child(
                                    div()
                                        .font_weight(FontWeight::BOLD)
                                        .text_sm()
                                        .text_color(theme.foreground)
                                        .text_ellipsis()
                                        .child(collection_name),
                                ),
                        ),
                    )
                    .child(
                        Input::new(&self.search_input)
                            .prefix(Icon::new(IconName::Search))
                            .cleanable(true),
                    ),
            )
            // 2. Sidebar Tree Content Canvas
            .child(
                div()
                    .flex_1()
                    .p_2()
                    .overflow_y_scrollbar()
                    .child(self.render_tree_content(&query, cx)),
            )
    }
}

impl Sidebar {
    fn render_tree_content(&self, query: &str, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(col) = &self.collection else {
            return v_flex()
                .p_4()
                .items_center()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child("No hay colección cargada")
                .into_any_element();
        };

        if query.is_empty() {
            // Render hierarchical folder/request tree
            v_flex()
                .gap_1()
                .w_full()
                .children(
                    col.items
                        .iter()
                        .map(|item| self.render_collection_item(item, 0, cx).into_any_element()),
                )
                .into_any_element()
        } else {
            // Render filtered search results
            let filtered = filter_collection_items(&col.items, query, &[]);
            if filtered.is_empty() {
                v_flex()
                    .p_4()
                    .items_center()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(format!("No hay resultados para \"{}\"", query))
                    .into_any_element()
            } else {
                v_flex()
                    .gap_1()
                    .w_full()
                    .children(filtered.into_iter().map(|f_item| {
                        match f_item.kind {
                            super::search::FilteredItemKind::Folder => h_flex()
                                .px_2()
                                .py_1p5()
                                .gap_2()
                                .items_center()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(Icon::new(IconName::Folder))
                                .child(f_item.name)
                                .into_any_element(),
                            super::search::FilteredItemKind::Request(method) => {
                                let req_id = f_item.id.clone();
                                let is_selected =
                                    self.selected_request_id.as_deref() == Some(&f_item.id);

                                self.render_request_row(
                                    &f_item.name,
                                    &req_id,
                                    method,
                                    is_selected,
                                    0,
                                    cx,
                                )
                                .into_any_element()
                            }
                        }
                    }))
                    .into_any_element()
            }
        }
    }

    fn render_collection_item(
        &self,
        item: &CollectionItem,
        depth: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        match item {
            CollectionItem::Folder(folder) => {
                self.render_folder_row(folder, depth, cx).into_any_element()
            }
            CollectionItem::Request(req) => {
                let is_selected = self.selected_request_id.as_deref() == Some(&req.id);
                self.render_request_row(&req.name, &req.id, req.method, is_selected, depth, cx)
                    .into_any_element()
            }
        }
    }

    fn render_folder_row(
        &self,
        folder: &Folder,
        depth: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme();
        let is_collapsed = self.collapsed_folders.contains(&folder.id);
        let folder_id = folder.id.clone();
        let indent = (depth as f32) * 12.0;

        let icon_name = if is_collapsed {
            IconName::Folder
        } else {
            IconName::FolderOpen
        };

        let chevron_name = if is_collapsed {
            IconName::ChevronRight
        } else {
            IconName::ChevronDown
        };

        v_flex()
            .w_full()
            .child(
                h_flex().w_full().pl(px(indent + 8.)).pr_2().child(
                    Button::new(format!("folder-btn-{}", folder_id))
                        .ghost()
                        .w_full()
                        .on_click(cx.listener({
                            let f_id = folder_id.clone();
                            move |this, _, _, cx| {
                                this.toggle_folder(&f_id, cx);
                            }
                        }))
                        .child(
                            h_flex()
                                .w_full()
                                .items_center()
                                .justify_between()
                                .gap_2()
                                .py_0p5()
                                .child(
                                    h_flex()
                                        .items_center()
                                        .gap_2()
                                        .flex_1()
                                        .child(
                                            Icon::new(chevron_name)
                                                .text_color(theme.muted_foreground),
                                        )
                                        .child(Icon::new(icon_name).text_color(rgb(0xe06c1b)))
                                        .child(
                                            div()
                                                .text_xs()
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(theme.foreground)
                                                .text_ellipsis()
                                                .child(folder.name.clone()),
                                        ),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(theme.muted_foreground)
                                        .child(folder.items.len().to_string()),
                                ),
                        ),
                ),
            )
            .when(!is_collapsed, |this| {
                this.children(folder.items.iter().map(|child_item| {
                    self.render_collection_item(child_item, depth + 1, cx)
                        .into_any_element()
                }))
            })
    }

    fn render_request_row(
        &self,
        name: &str,
        id: &str,
        method: HttpMethod,
        is_selected: bool,
        depth: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let req_id = id.to_string();
        let req_name = name.to_string();
        let indent = (depth as f32) * 12.0;

        h_flex().w_full().pl(px(indent + 16.)).pr_2().child(
            Button::new(format!("open-req-{}", req_id))
                .ghost()
                .w_full()
                .on_click(cx.listener({
                    let r_id = req_id.clone();
                    move |this, _, _, cx| {
                        this.selected_request_id = Some(r_id.clone());
                        cx.emit(SidebarEvent::SelectRequest(r_id.clone()));
                        cx.notify();
                    }
                }))
                .child(
                    h_flex()
                        .w_full()
                        .items_center()
                        .justify_between()
                        .gap_2()
                        .py_0p5()
                        .child(
                            h_flex()
                                .items_center()
                                .gap_2()
                                .flex_1()
                                .child(method_badge(method))
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(if is_selected {
                                            FontWeight::SEMIBOLD
                                        } else {
                                            FontWeight::NORMAL
                                        })
                                        .text_color(cx.theme().foreground)
                                        .when(is_selected, |this| this.text_color(rgb(0xe06c1b)))
                                        .text_ellipsis()
                                        .child(req_name),
                                ),
                        )
                        .when(is_selected, |this| {
                            this.child(div().size(px(6.)).rounded_full().bg(rgb(0xe06c1b)))
                        }),
                ),
        )
    }
}

fn method_badge(method: HttpMethod) -> impl IntoElement {
    let (text, color_bg, color_fg) = match method {
        HttpMethod::GET => ("GET", rgb(0x10b981), rgb(0xffffff)),
        HttpMethod::POST => ("POST", rgb(0x3b82f6), rgb(0xffffff)),
        HttpMethod::PUT => ("PUT", rgb(0xf59e0b), rgb(0xffffff)),
        HttpMethod::DELETE => ("DEL", rgb(0xef4444), rgb(0xffffff)),
        HttpMethod::PATCH => ("PAT", rgb(0x8b5cf6), rgb(0xffffff)),
        HttpMethod::HEAD => ("HEAD", rgb(0x6b7280), rgb(0xffffff)),
        HttpMethod::OPTIONS => ("OPT", rgb(0x6b7280), rgb(0xffffff)),
    };

    div()
        .px_1p5()
        .py_0p5()
        .rounded_xs()
        .text_xs()
        .font_weight(FontWeight::BOLD)
        .bg(color_bg)
        .text_color(color_fg)
        .child(SharedString::from(text))
}
