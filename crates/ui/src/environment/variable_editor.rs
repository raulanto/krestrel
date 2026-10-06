use std::collections::HashSet;

use crate::theme::{ThemeExt as _, h_flex, v_flex};
use ely_gpui_component::buttons::Button;
use ely_gpui_component::forms::{Input, TextInput};
use gpui::{AppContext, Context, Entity, FontWeight, Window, div, prelude::*, px};
use kestrel_core::{EnvVariable, Environment};

use super::variable_row::VariableRow;

pub enum VariableEditorEvent {
    AddVariable { name: String, value: String },
    ToggleVariableEnabled(String),
    ToggleVariableSecret(String),
    DeleteVariable(String),
}

pub struct VariablesEditor {
    var_name_input: Entity<TextInput>,
    var_value_input: Entity<TextInput>,
    revealed_secrets: HashSet<String>,
}

impl VariablesEditor {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let var_name_input = cx.new(|cx| {
            TextInput::new(window, cx).placeholder("Nombre de la variable (ej. baseUrl)")
        });
        let var_value_input =
            cx.new(|cx| TextInput::new(window, cx).placeholder("Valor de la variable"));

        Self {
            var_name_input,
            var_value_input,
            revealed_secrets: HashSet::new(),
        }
    }

    pub fn render_editor(&mut self, env: &Environment, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let env_name = env.name.clone();
        let vars: Vec<(String, EnvVariable)> = env
            .variables
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        v_flex()
            .flex_1()
            .h_full()
            .p_4()
            .gap_3()
            .child(
                h_flex().items_center().justify_between().w_full().child(
                    v_flex()
                        .gap_0p5()
                        .child(
                            div()
                                .font_weight(FontWeight::BOLD)
                                .text_base()
                                .text_color(colors.fg)
                                .child(env_name),
                        )
                        .child(div().text_xs().text_color(colors.fg_muted).child(
                            "Usa estas variables en URLs, Headers y Body como {{variable}}.",
                        )),
                ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(div().w(px(180.)).child(Input::new(&self.var_name_input)))
                    .child(div().flex_1().child(Input::new(&self.var_value_input)))
                    .child(
                        Button::new("add-var-btn", "Agregar")
                            .primary()
                            .on_click(cx.listener(|this, _, _window, cx| {
                                let name = this.var_name_input.read(cx).text().trim().to_string();
                                let value = this.var_value_input.read(cx).text().trim().to_string();
                                if !name.is_empty() {
                                    cx.emit(VariableEditorEvent::AddVariable { name, value });
                                    this.var_name_input.update(cx, |input, cx| {
                                        input.set_text("", cx);
                                    });
                                    this.var_value_input.update(cx, |input, cx| {
                                        input.set_text("", cx);
                                    });
                                }
                            })),
                    ),
            )
            .child(
                div()
                    .id("var-editor-scroll")
                    .flex_1()
                    .rounded_md()
                    .border_1()
                    .border_color(colors.border)
                    .bg(colors.bg)
                    .p_3()
                    .overflow_y_scroll()
                    .child(if vars.is_empty() {
                        div()
                            .p_4()
                            .text_sm()
                            .text_color(colors.fg_muted)
                            .child("No hay variables definidas en este entorno.")
                    } else {
                        v_flex()
                            .gap_2()
                            .children(vars.into_iter().map(|(key, var)| {
                                let is_revealed = self.revealed_secrets.contains(&key);

                                let key_toggle_enable = key.clone();
                                let key_toggle_secret = key.clone();
                                let key_delete = key.clone();

                                VariableRow::new(
                                    key,
                                    var,
                                    is_revealed,
                                    cx.listener(move |_this, _, _window, cx| {
                                        cx.emit(VariableEditorEvent::ToggleVariableEnabled(
                                            key_toggle_enable.clone(),
                                        ));
                                    }),
                                    cx.listener(move |_this, _, _window, cx| {
                                        cx.emit(VariableEditorEvent::ToggleVariableSecret(
                                            key_toggle_secret.clone(),
                                        ));
                                    }),
                                    cx.listener(move |_this, _, _window, cx| {
                                        cx.emit(VariableEditorEvent::DeleteVariable(
                                            key_delete.clone(),
                                        ));
                                    }),
                                )
                            }))
                    }),
            )
    }
}

impl gpui::EventEmitter<VariableEditorEvent> for VariablesEditor {}

impl Render for VariablesEditor {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}
