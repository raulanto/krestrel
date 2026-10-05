use gpui::{
    AppContext, Context, Entity, EventEmitter, FontWeight, IntoElement, ParentElement as _, Render,
    Styled, Subscription, Window, div, prelude::FluentBuilder as _, px, rgb,
};
use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Input, InputState},
    scroll::ScrollableElement as _,
    v_flex,
};
use kestrel_core::{EnvVariable, Environment};

#[derive(Debug, Clone)]
pub enum EnvironmentEvent {
    EnvironmentSelected(String),
    EnvironmentModified(Environment),
    CloseRequested,
}

pub struct EnvironmentModal {
    environments: Vec<Environment>,
    active_env_id: Option<String>,
    selected_edit_index: usize,
    var_name_input: Entity<InputState>,
    var_value_input: Entity<InputState>,
    is_open: bool,
    _subscriptions: Vec<Subscription>,
}

impl EnvironmentModal {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let var_name_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("Nombre de la variable (ej. baseUrl)")
        });

        let var_value_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Valor de la variable"));

        Self {
            environments: Vec::new(),
            active_env_id: None,
            selected_edit_index: 0,
            var_name_input,
            var_value_input,
            is_open: false,
            _subscriptions: Vec::new(),
        }
    }

    pub fn set_environments(
        &mut self,
        environments: Vec<Environment>,
        active_id: Option<String>,
        cx: &mut Context<Self>,
    ) {
        self.environments = environments;
        self.active_env_id = active_id;
        self.selected_edit_index = 0;
        cx.notify();
    }

    pub fn open(&mut self, cx: &mut Context<Self>) {
        self.is_open = true;
        cx.notify();
    }

    pub fn close(&mut self, cx: &mut Context<Self>) {
        self.is_open = false;
        cx.emit(EnvironmentEvent::CloseRequested);
        cx.notify();
    }

    pub fn is_open(&self) -> bool {
        self.is_open
    }

    pub fn active_environment(&self) -> Option<&Environment> {
        self.environments
            .iter()
            .find(|e| Some(&e.id) == self.active_env_id.as_ref())
    }

    pub fn all_environments(&self) -> &[Environment] {
        &self.environments
    }

    fn add_variable(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = self.var_name_input.read(cx).value().trim().to_string();
        let value = self.var_value_input.read(cx).value().trim().to_string();

        if !name.is_empty() {
            if let Some(env) = self.environments.get_mut(self.selected_edit_index) {
                env.variables.insert(
                    name,
                    EnvVariable {
                        value,
                        enabled: true,
                        secret: false,
                    },
                );
                let env_clone = env.clone();
                cx.emit(EnvironmentEvent::EnvironmentModified(env_clone));
            }

            self.var_name_input.update(cx, |input, cx| {
                input.set_value(String::new(), window, cx);
            });
            self.var_value_input.update(cx, |input, cx| {
                input.set_value(String::new(), window, cx);
            });
            cx.notify();
        }
    }

    fn toggle_var_enabled(&mut self, key: &str, cx: &mut Context<Self>) {
        if let Some(var) = self
            .environments
            .get_mut(self.selected_edit_index)
            .and_then(|env| env.variables.get_mut(key))
        {
            var.enabled = !var.enabled;
            if let Some(env) = self.environments.get(self.selected_edit_index) {
                cx.emit(EnvironmentEvent::EnvironmentModified(env.clone()));
            }
            cx.notify();
        }
    }

    fn toggle_var_secret(&mut self, key: &str, cx: &mut Context<Self>) {
        if let Some(var) = self
            .environments
            .get_mut(self.selected_edit_index)
            .and_then(|env| env.variables.get_mut(key))
        {
            var.secret = !var.secret;
            if let Some(env) = self.environments.get(self.selected_edit_index) {
                cx.emit(EnvironmentEvent::EnvironmentModified(env.clone()));
            }
            cx.notify();
        }
    }

    fn delete_variable(&mut self, key: &str, cx: &mut Context<Self>) {
        if let Some(env) = self.environments.get_mut(self.selected_edit_index) {
            env.variables.swap_remove(key);
            let env_clone = env.clone();
            cx.emit(EnvironmentEvent::EnvironmentModified(env_clone));
            cx.notify();
        }
    }

    fn add_environment(&mut self, name: &str, cx: &mut Context<Self>) {
        let new_env = Environment::new(uuid::Uuid::new_v4().to_string(), name);
        self.environments.push(new_env);
        self.selected_edit_index = self.environments.len() - 1;
        cx.notify();
    }
}

impl EventEmitter<EnvironmentEvent> for EnvironmentModal {}

impl Render for EnvironmentModal {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        if !self.is_open {
            return div().into_any_element();
        }

        let selected_ix = self.selected_edit_index;
        let selected_env = self.environments.get(selected_ix).cloned();
        let active_id = self.active_env_id.clone();

        // Modal backdrop overlay
        div()
            .absolute()
            .inset_0()
            .bg(rgb(0x000000).opacity(0.6))
            .flex()
            .items_center()
            .justify_center()
            .child(
                // Modal Window Box
                v_flex()
                    .w(px(720.))
                    .h(px(520.))
                    .rounded_xl()
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.background)
                    .shadow_lg()
                    .overflow_hidden()
                    // Modal Header
                    .child(
                        h_flex()
                            .h(px(48.))
                            .w_full()
                            .px_4()
                            .border_b_1()
                            .border_color(theme.border)
                            .bg(theme.muted.opacity(0.2))
                            .items_center()
                            .justify_between()
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_2()
                                    .child(Icon::new(IconName::Globe).text_color(rgb(0x22c55e)))
                                    .child(
                                        div()
                                            .font_weight(FontWeight::BOLD)
                                            .text_sm()
                                            .child("Gestión de Entornos y Variables"),
                                    ),
                            )
                            .child(
                                Button::new("close-modal-btn")
                                    .ghost()
                                    .icon(Icon::new(IconName::Close))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.close(cx);
                                    })),
                            ),
                    )
                    // Modal Body: Left sidebar list of environments, Right variables editor
                    .child(
                        h_flex()
                            .flex_1()
                            .size_full()
                            // Left list: Environments
                            .child(
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
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.add_environment("Nuevo Entorno", cx);
                                                    })),
                                            ),
                                    )
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .gap_1()
                                            .children(self.environments.iter().enumerate().map(
                                                |(ix, env)| {
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
                                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                                    this.selected_edit_index = ix;
                                                                    cx.notify();
                                                                })),
                                                        )
                                                        .child(
                                                            Button::new(format!("env-activate-{}", ix))
                                                                .ghost()
                                                                .when(is_active, |b| {
                                                                    b.child(
                                                                        div()
                                                                            .size(px(8.))
                                                                            .rounded_full()
                                                                            .bg(rgb(0x22c55e)),
                                                                    )
                                                                })
                                                                .when(!is_active, |b| {
                                                                    b.label("Activar").on_click(
                                                                        cx.listener(move |this, _, _, cx| {
                                                                            this.active_env_id =
                                                                                Some(env_id.clone());
                                                                            cx.emit(
                                                                                EnvironmentEvent::EnvironmentSelected(
                                                                                    env_id.clone(),
                                                                                ),
                                                                            );
                                                                            cx.notify();
                                                                        }),
                                                                    )
                                                                }),
                                                        )
                                                },
                                            )),
                                    ),
                            )
                            // Right side: Variables Editor
                            .child(match selected_env {
                                Some(env) => self.render_variables_editor(&env, cx).into_any_element(),
                                None => div()
                                    .flex_1()
                                    .items_center()
                                    .justify_center()
                                    .p_8()
                                    .text_sm()
                                    .text_color(theme.muted_foreground)
                                    .child("Selecciona o crea un entorno para gestionar variables")
                                    .into_any_element(),
                            }),
                    ),
            )
            .into_any_element()
    }
}

impl EnvironmentModal {
    fn render_variables_editor(
        &self,
        env: &Environment,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme();
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
            // Top: Environment title and explanation
            .child(
                h_flex().items_center().justify_between().w_full().child(
                    v_flex()
                        .gap_0p5()
                        .child(
                            div()
                                .font_weight(FontWeight::BOLD)
                                .text_base()
                                .child(env_name),
                        )
                        .child(div().text_xs().text_color(theme.muted_foreground).child(
                            "Usa estas variables en URLs, Headers y Body como {{variable}}.",
                        )),
                ),
            )
            // Variable creation bar: Key + Value + Add button
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .w(px(180.))
                            .child(Input::new(&self.var_name_input).bordered(true)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .child(Input::new(&self.var_value_input).bordered(true)),
                    )
                    .child(
                        Button::new("add-var-btn")
                            .primary()
                            .label("Agregar")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.add_variable(window, cx);
                            })),
                    ),
            )
            // Variables Table / List
            .child(
                div()
                    .flex_1()
                    .rounded_md()
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.background)
                    .p_3()
                    .overflow_y_scrollbar()
                    .child(if vars.is_empty() {
                        div()
                            .p_4()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child("No hay variables definidas en este entorno.")
                    } else {
                        v_flex()
                            .gap_2()
                            .children(vars.into_iter().map(|(key, var)| {
                                let key_for_enable = key.clone();
                                let key_for_secret = key.clone();
                                let key_for_del = key.clone();
                                let is_enabled = var.enabled;
                                let is_secret = var.secret;

                                let display_val = if is_secret {
                                    "••••••••".to_string()
                                } else {
                                    var.value
                                };

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
                                                Button::new(format!("toggle-var-{}", key))
                                                    .ghost()
                                                    .label(if is_enabled { "✓" } else { "✗" })
                                                    .on_click(cx.listener(
                                                        move |this, _, _, cx| {
                                                            this.toggle_var_enabled(
                                                                &key_for_enable,
                                                                cx,
                                                            );
                                                        },
                                                    )),
                                            )
                                            .child(
                                                div()
                                                    .w(px(140.))
                                                    .font_weight(FontWeight::BOLD)
                                                    .text_xs()
                                                    .child(key),
                                            )
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .font_family("monospace")
                                                    .text_xs()
                                                    .text_color(if is_enabled {
                                                        theme.foreground
                                                    } else {
                                                        theme.muted_foreground
                                                    })
                                                    .child(display_val),
                                            ),
                                    )
                                    .child(
                                        h_flex()
                                            .items_center()
                                            .gap_1()
                                            .child(
                                                Button::new(format!(
                                                    "secret-var-{}",
                                                    key_for_secret
                                                ))
                                                .ghost()
                                                .icon(Icon::new(if is_secret {
                                                    IconName::EyeOff
                                                } else {
                                                    IconName::Eye
                                                }))
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    this.toggle_var_secret(&key_for_secret, cx);
                                                })),
                                            )
                                            .child(
                                                Button::new(format!("del-var-{}", key_for_del))
                                                    .ghost()
                                                    .icon(Icon::new(IconName::Close))
                                                    .on_click(cx.listener(
                                                        move |this, _, _, cx| {
                                                            this.delete_variable(&key_for_del, cx);
                                                        },
                                                    )),
                                            ),
                                    )
                            }))
                    }),
            )
    }
}
