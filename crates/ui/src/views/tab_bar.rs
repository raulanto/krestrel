use gpui::{
    Context, EventEmitter, IntoElement, ParentElement as _, Render, Styled, Window,
    prelude::FluentBuilder as _, px, rgb,
};
use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName,
    button::{Button, ButtonVariants as _},
    h_flex,
};

pub struct TabItem {
    pub id: String,
    pub title: String,
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
            .bg(theme.muted.opacity(0.2))
            .items_center()
            .children(self.tabs.iter().enumerate().map(|(ix, tab)| {
                let is_active = ix == active_ix;
                let tab_title = tab.title.clone();

                h_flex()
                    .h_full()
                    .px_2()
                    .items_center()
                    .gap_1()
                    .border_r_1()
                    .border_color(theme.border)
                    .text_xs()
                    .cursor_pointer()
                    .when(is_active, |this| {
                        this.bg(theme.background)
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(rgb(0xe06c1b)) // Distinct warm highlight
                            .border_t_2()
                            .border_color(rgb(0xe06c1b))
                    })
                    .when(!is_active, |this| this.text_color(theme.muted_foreground))
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
