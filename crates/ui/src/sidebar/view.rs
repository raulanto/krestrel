//! Sidebar view component (`SidebarView`) with search, keyboard navigation, and event emission.

use crate::sidebar::row::SidebarRow;
use crate::sidebar::tree_state::{FlatNodeKind, FlatTreeNode, TreeState};
use gpui::{
    AppContext, Context, Entity, EventEmitter, FocusHandle, Focusable, InteractiveElement as _,
    IntoElement, KeyDownEvent, ParentElement as _, Render, Styled as _, Subscription, Window, div,
};
use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName,
    button::{Button, ButtonVariants as _},
    input::{Input, InputEvent, InputState},
    scroll::ScrollableElement as _,
    v_flex,
};
use kestrel_core::Collection;

#[derive(Debug, Clone)]
pub enum SidebarEvent {
    OpenRequest(String),
    Rename { id: String, new_name: String },
    Duplicate(String),
    Delete(String),
    CopyPath(String),
}

pub struct SidebarView {
    collection: Option<Collection>,
    tree_state: TreeState,
    search_input: Entity<InputState>,
    renaming_id: Option<String>,
    renaming_input: Option<Entity<InputState>>,
    focus_handle: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

impl SidebarView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Buscar solicitudes... (Ctrl+K)"));

        let sub = cx.subscribe_in(&search_input, window, {
            let search_input = search_input.clone();
            move |this, _, ev: &InputEvent, _window, cx| {
                if let InputEvent::Change = ev {
                    let val = search_input.read(cx).value();
                    this.tree_state.set_search_query(&val);
                    cx.notify();
                }
            }
        });

        Self {
            collection: None,
            tree_state: TreeState::new(),
            search_input,
            renaming_id: None,
            renaming_input: None,
            focus_handle: cx.focus_handle(),
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

    pub fn focus_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.search_input.update(cx, |input, cx| {
            input.focus(window, cx);
        });
    }

    pub fn clear_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.search_input.update(cx, |input, cx| {
            input.set_value("", window, cx);
        });
        self.tree_state.clear_search_query();
        cx.notify();
    }

    pub fn start_rename(
        &mut self,
        id: String,
        current_name: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let input = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.set_value(current_name, window, cx);
            s
        });
        self.renaming_id = Some(id);
        self.renaming_input = Some(input.clone());
        input.update(cx, |s, cx| {
            s.focus(window, cx);
        });
        cx.notify();
    }

    pub fn confirm_rename(&mut self, cx: &mut Context<Self>) {
        if let (Some(id), Some(inp)) = (self.renaming_id.take(), self.renaming_input.take()) {
            let new_name = inp.read(cx).value();
            if !new_name.trim().is_empty() {
                cx.emit(SidebarEvent::Rename {
                    id,
                    new_name: new_name.trim().to_string(),
                });
            }
        }
        cx.notify();
    }

    pub fn cancel_rename(&mut self, cx: &mut Context<Self>) {
        self.renaming_id = None;
        self.renaming_input = None;
        cx.notify();
    }

    pub fn handle_key_down(
        &mut self,
        event: &KeyDownEvent,
        flat_nodes: &[FlatTreeNode],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if flat_nodes.is_empty() {
            return;
        }

        match event.keystroke.key.as_str() {
            "up" => {
                self.tree_state.move_up(flat_nodes.len());
                cx.notify();
            }
            "down" => {
                self.tree_state.move_down(flat_nodes.len());
                cx.notify();
            }
            "left" => {
                let folder_to_collapse = match flat_nodes.get(self.tree_state.focused_index) {
                    Some(FlatTreeNode {
                        id,
                        kind:
                            FlatNodeKind::Folder {
                                is_collapsed: false,
                                ..
                            },
                        ..
                    }) => Some(id.clone()),
                    _ => None,
                };
                if let Some(id) = folder_to_collapse {
                    self.tree_state.collapse_folder(&id);
                    cx.notify();
                }
            }
            "right" => {
                let folder_to_expand = match flat_nodes.get(self.tree_state.focused_index) {
                    Some(FlatTreeNode {
                        id,
                        kind:
                            FlatNodeKind::Folder {
                                is_collapsed: true, ..
                            },
                        ..
                    }) => Some(id.clone()),
                    _ => None,
                };
                if let Some(id) = folder_to_expand {
                    self.tree_state.expand_folder(&id);
                    cx.notify();
                }
            }
            "enter" => {
                if let Some(node) = flat_nodes.get(self.tree_state.focused_index) {
                    match &node.kind {
                        FlatNodeKind::Folder { .. } => {
                            self.tree_state.toggle_folder(&node.id);
                            cx.notify();
                        }
                        FlatNodeKind::Request { .. } => {
                            self.tree_state.selected_id = Some(node.id.clone());
                            cx.emit(SidebarEvent::OpenRequest(node.id.clone()));
                            cx.notify();
                        }
                        FlatNodeKind::ErrorNode { .. } => {}
                    }
                }
            }
            "f2" => {
                if let Some(node) = flat_nodes.get(self.tree_state.focused_index) {
                    let id = node.id.clone();
                    let name = node.name.clone();
                    self.start_rename(id, &name, window, cx);
                }
            }
            "escape" => {
                if self.renaming_id.is_some() {
                    self.cancel_rename(cx);
                } else if !self.tree_state.search_query.is_empty() {
                    self.clear_search(window, cx);
                }
            }
            _ => {}
        }
    }
}

impl EventEmitter<SidebarEvent> for SidebarView {}

impl Focusable for SidebarView {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SidebarView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        let items = self
            .collection
            .as_ref()
            .map(|c| c.items.as_slice())
            .unwrap_or(&[]);
        let flat_nodes = self.tree_state.flatten(items);

        let mut sidebar_container = v_flex()
            .id("sidebar-view")
            .key_context("SidebarView")
            .track_focus(&self.focus_handle)
            .h_full()
            .w_full()
            .bg(theme.background)
            .border_r_1()
            .border_color(theme.border);

        // Header with search input
        let search_bar = v_flex()
            .p_2()
            .border_b_1()
            .border_color(theme.border)
            .child(
                Input::new(&self.search_input)
                    .prefix(Icon::new(IconName::Search))
                    .cleanable(true),
            );

        sidebar_container = sidebar_container.child(search_bar);

        // Empty state when search query yields no results
        if !self.tree_state.search_query.is_empty() && flat_nodes.is_empty() {
            let empty_search = v_flex()
                .items_center()
                .justify_center()
                .p_4()
                .gap_2()
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child("No se encontraron solicitudes"),
                )
                .child(
                    Button::new("btn-clear-search")
                        .ghost()
                        .label("Limpiar búsqueda")
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.clear_search(window, cx);
                        })),
                );

            return sidebar_container.child(empty_search);
        }

        // Empty state when no collection is open
        if self.collection.is_none() {
            let empty_collection = v_flex()
                .items_center()
                .justify_center()
                .p_4()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child("Sin colección abierta");

            return sidebar_container.child(empty_collection);
        }

        // Render flat nodes list
        let mut list_container = v_flex().flex_1().overflow_y_scrollbar().p_1();

        for (idx, node) in flat_nodes.iter().enumerate() {
            let is_selected = self.tree_state.selected_id.as_deref() == Some(&node.id);
            let is_focused = self.tree_state.focused_index == idx;

            let node_id = node.id.clone();
            let node_kind = node.kind.clone();

            let row = SidebarRow::new(node.clone())
                .selected(is_selected)
                .focused(is_focused)
                .on_click(cx.listener({
                    let node_id = node_id.clone();
                    let node_kind = node_kind.clone();
                    move |this, _, _, cx| {
                        this.tree_state.focused_index = idx;
                        match &node_kind {
                            FlatNodeKind::Folder { .. } => {
                                this.tree_state.toggle_folder(&node_id);
                            }
                            FlatNodeKind::Request { .. } => {
                                this.tree_state.selected_id = Some(node_id.clone());
                                cx.emit(SidebarEvent::OpenRequest(node_id.clone()));
                            }
                            FlatNodeKind::ErrorNode { .. } => {}
                        }
                        cx.notify();
                    }
                }))
                .on_toggle(cx.listener({
                    let node_id = node_id.clone();
                    move |this, _, _, cx| {
                        this.tree_state.toggle_folder(&node_id);
                        cx.notify();
                    }
                }));

            list_container = list_container.child(row);
        }

        sidebar_container.child(list_container)
    }
}
