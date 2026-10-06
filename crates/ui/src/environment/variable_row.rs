use gpui::{
    ClickEvent, FontWeight, IntoElement, ParentElement as _, RenderOnce, SharedString, Styled, div,
    prelude::FluentBuilder as _, rgb,
};
use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName,
    button::{Button, ButtonVariants as _},
    h_flex,
};
use kestrel_core::EnvVariable;

#[derive(IntoElement)]
pub struct VariableRow<F1, F2, F3>
where
    F1: Fn(&ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    F2: Fn(&ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    F3: Fn(&ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
{
    key: String,
    variable: EnvVariable,
    is_revealed: bool,
    on_toggle_enable: F1,
    on_toggle_secret: F2,
    on_delete: F3,
}

impl<F1, F2, F3> VariableRow<F1, F2, F3>
where
    F1: Fn(&ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    F2: Fn(&ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    F3: Fn(&ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
{
    pub fn new(
        key: impl Into<String>,
        variable: EnvVariable,
        is_revealed: bool,
        on_toggle_enable: F1,
        on_toggle_secret: F2,
        on_delete: F3,
    ) -> Self {
        Self {
            key: key.into(),
            variable,
            is_revealed,
            on_toggle_enable,
            on_toggle_secret,
            on_delete,
        }
    }
}

impl<F1, F2, F3> RenderOnce for VariableRow<F1, F2, F3>
where
    F1: Fn(&ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    F2: Fn(&ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    F3: Fn(&ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
{
    fn render(self, _window: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        let theme = cx.theme();
        let key = self.key;
        let is_enabled = self.variable.enabled;
        let is_secret = self.variable.secret;

        let display_val = if is_secret && !self.is_revealed {
            "••••••••".to_string()
        } else {
            self.variable.value.clone()
        };

        let on_toggle_enable = self.on_toggle_enable;
        let on_toggle_secret = self.on_toggle_secret;
        let on_delete = self.on_delete;

        h_flex()
            .items_center()
            .justify_between()
            .px_2()
            .py_1p5()
            .rounded_md()
            .bg(theme.muted.opacity(0.3))
            .child(
                h_flex()
                    .items_center()
                    .gap_2()
                    .child(
                        Button::new(SharedString::from(format!("toggle-var-{}", key)))
                            .ghost()
                            .label(if is_enabled { "✓" } else { "✗" })
                            .on_click(on_toggle_enable),
                    )
                    .child(
                        div()
                            .w(gpui::px(140.))
                            .font_weight(FontWeight::BOLD)
                            .text_xs()
                            .child(key.clone()),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(display_val),
                    ),
            )
            .child(
                h_flex()
                    .items_center()
                    .gap_1()
                    .child(
                        Button::new(SharedString::from(format!("secret-var-{}", key)))
                            .ghost()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(theme.muted_foreground)
                                    .when(is_secret, |this| this.text_color(rgb(0xe06c1b)))
                                    .child(if is_secret { "Secreto" } else { "Público" }),
                            )
                            .on_click(on_toggle_secret),
                    )
                    .child(
                        Button::new(SharedString::from(format!("del-var-{}", key)))
                            .ghost()
                            .icon(Icon::new(IconName::Close))
                            .on_click(on_delete),
                    ),
            )
    }
}
