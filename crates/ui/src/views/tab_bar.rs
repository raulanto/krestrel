use crate::theme::{ThemeExt as _, h_flex, http_method_badge};
use ely_gpui_component::buttons::IconButton;
use ely_gpui_component::primitives::IconName;
use ely_gpui_component::theme::ControlSize;
use gpui::{
    Context, EventEmitter, FontWeight, InteractiveElement as _, IntoElement, ParentElement as _,
    Render, StatefulInteractiveElement as _, Styled, Window, div, prelude::FluentBuilder as _, px,
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
        let colors = &theme.colors;
        let active_ix = self.active_index;

        h_flex()
            .h(px(38.))
            .w_full()
            .border_b_1()
            .border_color(colors.border)
            .bg(colors.sunken)
            .items_center()
            .children(self.tabs.iter().enumerate().map(|(ix, tab)| {
                let is_active = ix == active_ix;
                let tab_title = tab.title.clone();
                let method = tab.method;
                let is_dirty = tab.is_dirty;

                let (method_bg, method_fg, method_label) = http_method_badge(method, colors);

                h_flex()
                    .id(format!("tab-item-{}", ix))
                    .h_full()
                    .px_2()
                    .items_center()
                    .gap_1p5()
                    .border_r_1()
                    .border_color(colors.border)
                    .text_xs()
                    .cursor_pointer()
                    .when(is_active, |this| {
                        this.bg(colors.bg)
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(colors.accent)
                            .border_t_2()
                            .border_color(colors.accent)
                    })
                    .when(!is_active, |this| {
                        this.text_color(colors.fg_muted)
                            .hover(|s| s.bg(colors.hover))
                    })
                    .on_click(cx.listener(move |this, _, _window, cx| {
                        this.active_index = ix;
                        cx.emit(TabBarEvent::Select(ix));
                        cx.notify();
                    }))
                    .child(
                        div()
                            .px_1()
                            .py_0p5()
                            .rounded_sm()
                            .text_xs()
                            .font_weight(FontWeight::BOLD)
                            .bg(method_bg)
                            .text_color(method_fg)
                            .child(method_label),
                    )
                    .child(
                        div()
                            .text_color(if is_active {
                                colors.fg
                            } else {
                                colors.fg_muted
                            })
                            .child(tab_title),
                    )
                    .when(is_dirty, |this| {
                        this.child(div().size(px(6.)).rounded_full().bg(colors.accent))
                    })
                    .child(
                        IconButton::new(format!("tab-close-{}", ix), IconName::X)
                            .size(ControlSize::Sm)
                            .on_click(cx.listener(move |_this, _, _window, cx| {
                                cx.emit(TabBarEvent::Close(ix));
                            })),
                    )
            }))
    }
}
