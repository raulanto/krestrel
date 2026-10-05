use gpui::{
    Context, EventEmitter, FontWeight, IntoElement, ParentElement as _, Render, Styled, Window,
    div, prelude::FluentBuilder as _, px, rgb,
};
use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName,
    button::{Button, ButtonVariants as _},
    h_flex,
};
use kestrel_core::HttpMethod;

#[derive(Debug, Clone)]
pub struct TabItem {
    pub id: String,
    pub title: String,
    pub method: HttpMethod,
    pub is_dirty: bool,
}

pub enum TabBarEvent {
    Select(usize),
    Close(usize),
}

pub struct WorkspaceTabBar {
    tabs: Vec<TabItem>,
    active_index: usize,
}

impl WorkspaceTabBar {
    pub fn new() -> Self {
        Self {
            tabs: Vec::new(),
            active_index: 0,
        }
    }

    pub fn set_tabs(&mut self, tabs: Vec<TabItem>, active_index: usize, cx: &mut Context<Self>) {
        self.tabs = tabs;
        self.active_index = active_index;
        cx.notify();
    }

    pub fn active_index(&self) -> usize {
        self.active_index
    }

    pub fn tabs(&self) -> &[TabItem] {
        &self.tabs
    }

    pub fn set_dirty(&mut self, index: usize, dirty: bool, cx: &mut Context<Self>) {
        if let Some(tab) = self.tabs.get_mut(index) {
            tab.is_dirty = dirty;
            cx.notify();
        }
    }
}

impl Default for WorkspaceTabBar {
    fn default() -> Self {
        Self::new()
    }
}

impl EventEmitter<TabBarEvent> for WorkspaceTabBar {}

impl Render for WorkspaceTabBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let active_ix = self.active_index;

        h_flex()
            .h(px(38.))
            .w_full()
            .border_b_1()
            .border_color(theme.border)
            .bg(theme.muted.opacity(0.18))
            .items_center()
            .children(self.tabs.iter().enumerate().map(|(ix, tab)| {
                let is_active = ix == active_ix;
                let tab_title = tab.title.clone();
                let method = tab.method;
                let is_dirty = tab.is_dirty;

                h_flex()
                    .h_full()
                    .px_2()
                    .items_center()
                    .gap_1p5()
                    .border_r_1()
                    .border_color(theme.border)
                    .text_xs()
                    .cursor_pointer()
                    .when(is_active, |this| {
                        this.bg(theme.background)
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(0xe06c1b)) // Warm highlight
                            .border_t_2()
                            .border_color(rgb(0xe06c1b))
                    })
                    .when(!is_active, |this| this.text_color(theme.muted_foreground))
                    .child(tab_method_label(method))
                    .child(
                        Button::new(format!("tab-btn-{}", ix))
                            .ghost()
                            .label(tab_title)
                            .on_click(cx.listener(move |this, _, _window, cx| {
                                this.active_index = ix;
                                cx.emit(TabBarEvent::Select(ix));
                                cx.notify();
                            })),
                    )
                    .when(is_dirty, |this| {
                        this.child(div().size(px(6.)).rounded_full().bg(rgb(0xe06c1b)))
                    })
                    .child(
                        Button::new(format!("tab-close-{}", ix))
                            .ghost()
                            .icon(Icon::new(IconName::Close))
                            .on_click(cx.listener(move |_this, _, _window, cx| {
                                cx.emit(TabBarEvent::Close(ix));
                            })),
                    )
            }))
    }
}

fn tab_method_label(method: HttpMethod) -> impl IntoElement {
    let (text, color) = match method {
        HttpMethod::GET => ("GET", rgb(0x10b981)),
        HttpMethod::POST => ("POST", rgb(0x3b82f6)),
        HttpMethod::PUT => ("PUT", rgb(0xf59e0b)),
        HttpMethod::DELETE => ("DEL", rgb(0xef4444)),
        HttpMethod::PATCH => ("PAT", rgb(0x8b5cf6)),
        HttpMethod::HEAD => ("HEAD", rgb(0x6b7280)),
        HttpMethod::OPTIONS => ("OPT", rgb(0x6b7280)),
    };

    div()
        .font_weight(FontWeight::BOLD)
        .text_xs()
        .text_color(color)
        .child(text)
}
