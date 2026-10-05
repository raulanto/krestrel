use std::collections::HashSet;

use gpui::{
    AppContext, Context, Entity, FontWeight, IntoElement, ParentElement as _, Render, SharedString,
    Styled, Subscription, Window, div, px, rgb,
};
use gpui_kit::component::{
    Icon, IconName, h_flex,
    input::{Input, InputEvent, InputState},
    sidebar::{
        Sidebar as KitSidebar, SidebarCollapsible, SidebarHeader, SidebarMenu, SidebarMenuItem,
    },
    v_flex,
};
use kestrel_core::{Collection, CollectionItem, Folder, HttpMethod, Request};

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

    fn render_request_item(
        &self,
        req: &Request,
        is_selected: bool,
        cx: &mut Context<Self>,
    ) -> SidebarMenuItem {
        let req_id = req.id.clone();
        let method = req.method;

        SidebarMenuItem::new(req.name.clone())
            .active(is_selected)
            .suffix(move |_, _| method_badge(method))
            .on_click(cx.listener(move |this, _, _window, cx| {
                this.selected_request_id = Some(req_id.clone());
                cx.emit(SidebarEvent::SelectRequest(req_id.clone()));
                cx.notify();
            }))
    }

    fn render_folder_item(&self, folder: &Folder, cx: &mut Context<Self>) -> SidebarMenuItem {
        let is_collapsed = self.collapsed_folders.contains(&folder.id);
        let folder_id = folder.id.clone();

        let icon_name = if is_collapsed {
            IconName::Folder
        } else {
            IconName::FolderOpen
        };

        let mut item = SidebarMenuItem::new(folder.name.clone())
            .icon(Icon::new(icon_name))
            .click_to_toggle(true)
            .on_click(cx.listener(move |this, _, _window, cx| {
                this.toggle_folder(&folder_id, cx);
            }));

        if !is_collapsed {
            let mut children = Vec::new();
            for sub_item in &folder.items {
                match sub_item {
                    CollectionItem::Folder(sub_folder) => {
                        children.push(self.render_folder_item(sub_folder, cx));
                    }
                    CollectionItem::Request(sub_req) => {
                        let is_selected = self.selected_request_id.as_deref() == Some(&sub_req.id);
                        children.push(self.render_request_item(sub_req, is_selected, cx));
                    }
                }
            }
            item = item.children(children);
        }

        item
    }
}

impl gpui::EventEmitter<SidebarEvent> for Sidebar {}

impl Render for Sidebar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let query = self.search_query.clone();

        let collection_name = self
            .collection
            .as_ref()
            .map(|c| c.name.clone())
            .unwrap_or_else(|| "Sin colección".to_string());

        let mut menu = SidebarMenu::new();

        if let Some(col) = &self.collection {
            if query.is_empty() {
                // Render tree hierarchy
                for item in &col.items {
                    match item {
                        CollectionItem::Folder(folder) => {
                            menu = menu.child(self.render_folder_item(folder, cx));
                        }
                        CollectionItem::Request(req) => {
                            let is_selected = self.selected_request_id.as_deref() == Some(&req.id);
                            menu = menu.child(self.render_request_item(req, is_selected, cx));
                        }
                    }
                }
            } else {
                // Render filtered list matching search query
                let filtered = filter_collection_items(&col.items, &query, &[]);
                for f_item in filtered {
                    match f_item.kind {
                        super::search::FilteredItemKind::Folder => {
                            menu = menu.child(
                                SidebarMenuItem::new(f_item.name)
                                    .icon(Icon::new(IconName::Folder))
                                    .disable(true),
                            );
                        }
                        super::search::FilteredItemKind::Request(method) => {
                            let req_id = f_item.id.clone();
                            let is_selected =
                                self.selected_request_id.as_deref() == Some(&f_item.id);
                            menu = menu.child(
                                SidebarMenuItem::new(f_item.name)
                                    .active(is_selected)
                                    .suffix(move |_, _| method_badge(method))
                                    .on_click(cx.listener(move |this, _, _window, cx| {
                                        this.selected_request_id = Some(req_id.clone());
                                        cx.emit(SidebarEvent::SelectRequest(req_id.clone()));
                                        cx.notify();
                                    })),
                            );
                        }
                    }
                }
            }
        }

        KitSidebar::new("app-sidebar")
            .collapsible(SidebarCollapsible::None)
            .w(px(280.))
            .header(
                SidebarHeader::new().child(
                    v_flex()
                        .gap_2()
                        .w_full()
                        .child(
                            h_flex()
                                .items_center()
                                .gap_2()
                                .child(Icon::new(IconName::FolderOpen))
                                .child(
                                    div()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_ellipsis()
                                        .child(collection_name),
                                ),
                        )
                        .child(
                            Input::new(&self.search_input)
                                .prefix(Icon::new(IconName::Search))
                                .cleanable(true),
                        ),
                ),
            )
            .child(menu)
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
        .px_1()
        .py_0p5()
        .rounded_xs()
        .text_xs()
        .font_weight(FontWeight::BOLD)
        .bg(color_bg)
        .text_color(color_fg)
        .child(SharedString::from(text))
}
