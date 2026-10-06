use gpui::{
    Context, FontWeight, IntoElement, ParentElement as _, Render, Styled, Window, div,
    prelude::FluentBuilder as _, px, rgb,
};
use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName,
    button::{Button, ButtonVariants as _},
    h_flex, v_flex,
};
use kestrel_core::Environment;

pub enum EnvListEvent {
    SelectEnvironment(usize),
    ActivateEnvironment(String),
    AddEnvironment,
}

pub struct EnvList {
    environments: Vec<Environment>,
    selected_index: usize,
    active_id: Option<String>,
}

impl EnvList {
    pub fn new(
        environments: Vec<Environment>,
        selected_index: usize,
        active_id: Option<String>,
    ) -> Self {
        Self {
            environments,
            selected_index,
            active_id,
        }
    }

    pub fn set_data(
        &mut self,
        environments: Vec<Environment>,
        selected_index: usize,
        active_id: Option<String>,
        cx: &mut Context<Self>,
    ) {
        self.environments = environments;
        self.selected_index = selected_index;
        self.active_id = active_id;
        cx.notify();
    }
}

impl gpui::EventEmitter<EnvListEvent> for EnvList {}

impl Render for EnvList {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let selected_ix = self.selected_index;
        let active_id = self.active_id.clone();

        v_flex()
            .w(px(220.))
            .h_full()
            .border_r_1()
            .border_color(theme.border)
            .bg(theme.muted.opacity(0.08))
            .p_3()
            .gap_2()
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .font_weight(FontWeight::BOLD)
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child("ENTORNOS"),
                    )
                    .child(
                        Button::new("add-env-btn")
                            .ghost()
                            .icon(Icon::new(IconName::Plus))
                            .on_click(cx.listener(|_this, _, _, cx| {
                                cx.emit(EnvListEvent::AddEnvironment);
                            })),
                    ),
            )
            .child(
                v_flex()
                    .flex_1()
                    .gap_1()
                    .children(self.environments.iter().enumerate().map(|(ix, env)| {
                        let is_editing = ix == selected_ix;
                        let is_active = Some(&env.id) == active_id.as_ref();
                        let env_id = env.id.clone();
                        let env_name = env.name.clone();

                        h_flex()
                            .w_full()
                            .items_center()
                            .justify_between()
                            .px_2()
                            .py_1p5()
                            .rounded_md()
                            .when(is_editing, |this| {
                                this.bg(theme.muted.opacity(0.8))
                                    .font_weight(FontWeight::BOLD)
                            })
                            .child(
                                Button::new(format!("env-select-{}", ix))
                                    .ghost()
                                    .label(env_name)
                                    .on_click(cx.listener(move |_this, _, _, cx| {
                                        cx.emit(EnvListEvent::SelectEnvironment(ix));
                                    })),
                            )
                            .child(
                                Button::new(format!("env-activate-{}", ix))
                                    .ghost()
                                    .when(is_active, |b| {
                                        b.child(div().size(px(8.)).rounded_full().bg(rgb(0x22c55e)))
                                    })
                                    .when(!is_active, |b| {
                                        b.label("Activar").on_click(cx.listener({
                                            let id = env_id.clone();
                                            move |_this, _, _, cx| {
                                                cx.emit(EnvListEvent::ActivateEnvironment(
                                                    id.clone(),
                                                ));
                                            }
                                        }))
                                    }),
                            )
                    })),
            )
    }
}
