use crate::theme::{ThemeExt as _, h_flex, v_flex};
use ely_gpui_component::buttons::{Button, ButtonVariant, IconButton};
use ely_gpui_component::primitives::IconName;
use ely_gpui_component::theme::ControlSize;
use gpui::{
    Context, FontWeight, InteractiveElement as _, IntoElement, ParentElement as _, Render, Styled,
    Window, div, prelude::FluentBuilder as _, px,
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
        let colors = &theme.colors;
        let selected_ix = self.selected_index;
        let active_id = self.active_id.clone();

        v_flex()
            .w(px(220.))
            .h_full()
            .border_r_1()
            .border_color(colors.border)
            .bg(colors.sunken)
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
                            .text_color(colors.fg_muted)
                            .child("ENTORNOS"),
                    )
                    .child(
                        IconButton::new("add-env-btn", IconName::Plus)
                            .size(ControlSize::Sm)
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
                            .cursor_pointer()
                            .when(is_editing, |this| {
                                this.bg(colors.active).font_weight(FontWeight::BOLD)
                            })
                            .when(!is_editing, |this| this.hover(|s| s.bg(colors.hover)))
                            .child(
                                Button::new(format!("env-select-{}", ix), env_name)
                                    .variant(ButtonVariant::Ghost)
                                    .size(ControlSize::Sm)
                                    .on_click(cx.listener(move |_this, _, _, cx| {
                                        cx.emit(EnvListEvent::SelectEnvironment(ix));
                                    })),
                            )
                            .child(if is_active {
                                div()
                                    .size(px(8.))
                                    .rounded_full()
                                    .bg(colors.success)
                                    .into_any_element()
                            } else {
                                Button::new(format!("env-activate-{}", ix), "Activar")
                                    .variant(ButtonVariant::Ghost)
                                    .size(ControlSize::Sm)
                                    .on_click(cx.listener({
                                        let id = env_id.clone();
                                        move |_this, _, _, cx| {
                                            cx.emit(EnvListEvent::ActivateEnvironment(id.clone()));
                                        }
                                    }))
                                    .into_any_element()
                            })
                    })),
            )
    }
}
