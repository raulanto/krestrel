use gpui::{
    AppContext, Context, Entity, EventEmitter, FontWeight, IntoElement, ParentElement as _, Render,
    Styled, Subscription, Window, div, px, rgb,
};
use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName,
    button::{Button, ButtonVariants as _},
    h_flex, v_flex,
};
use kestrel_core::{EnvVariable, Environment};

use super::env_list::{EnvList, EnvListEvent};
use super::variable_editor::{VariableEditorEvent, VariablesEditor};

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
    env_list: Entity<EnvList>,
    variables_editor: Entity<VariablesEditor>,
    is_open: bool,
    _subscriptions: Vec<Subscription>,
}

impl EnvironmentModal {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let env_list = cx.new(|_cx| EnvList::new(Vec::new(), 0, None));
        let variables_editor = cx.new(|cx| VariablesEditor::new(window, cx));

        let sub1 = cx.subscribe_in(
            &env_list,
            window,
            |this, _, ev: &EnvListEvent, _window, cx| match ev {
                EnvListEvent::SelectEnvironment(ix) => {
                    this.selected_edit_index = *ix;
                    cx.notify();
                }
                EnvListEvent::ActivateEnvironment(id) => {
                    this.active_env_id = Some(id.clone());
                    cx.emit(EnvironmentEvent::EnvironmentSelected(id.clone()));
                    cx.notify();
                }
                EnvListEvent::AddEnvironment => {
                    this.add_environment("Nuevo Entorno", cx);
                }
            },
        );

        let sub2 = cx.subscribe_in(
            &variables_editor,
            window,
            |this, _, ev: &VariableEditorEvent, _window, cx| match ev {
                VariableEditorEvent::AddVariable { name, value } => {
                    if let Some(env) = this.environments.get_mut(this.selected_edit_index) {
                        env.variables.insert(
                            name.clone(),
                            EnvVariable {
                                value: value.clone(),
                                enabled: true,
                                secret: false,
                            },
                        );
                        let env_clone = env.clone();
                        cx.emit(EnvironmentEvent::EnvironmentModified(env_clone));
                    }
                    cx.notify();
                }
                VariableEditorEvent::ToggleVariableEnabled(key) => {
                    this.toggle_var_enabled(key, cx);
                }
                VariableEditorEvent::ToggleVariableSecret(key) => {
                    this.toggle_var_secret(key, cx);
                }
                VariableEditorEvent::DeleteVariable(key) => {
                    this.delete_variable(key, cx);
                }
            },
        );

        Self {
            environments: Vec::new(),
            active_env_id: None,
            selected_edit_index: 0,
            env_list,
            variables_editor,
            is_open: false,
            _subscriptions: vec![sub1, sub2],
        }
    }

    pub fn set_environments(
        &mut self,
        environments: Vec<Environment>,
        active_id: Option<String>,
        cx: &mut Context<Self>,
    ) {
        self.environments = environments.clone();
        self.active_env_id = active_id.clone();
        self.selected_edit_index = 0;
        let selected_ix = self.selected_edit_index;
        let active_id_clone = self.active_env_id.clone();
        self.env_list.update(cx, |list, cx| {
            list.set_data(environments, selected_ix, active_id_clone, cx);
        });
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

    fn toggle_var_enabled(&mut self, key: &str, cx: &mut Context<Self>) {
        if let Some(env) = self.environments.get_mut(self.selected_edit_index)
            && let Some(var) = env.variables.get_mut(key)
        {
            var.enabled = !var.enabled;
            let env_clone = env.clone();
            cx.emit(EnvironmentEvent::EnvironmentModified(env_clone));
        }
        cx.notify();
    }

    fn toggle_var_secret(&mut self, key: &str, cx: &mut Context<Self>) {
        if let Some(env) = self.environments.get_mut(self.selected_edit_index)
            && let Some(var) = env.variables.get_mut(key)
        {
            var.secret = !var.secret;
            let env_clone = env.clone();
            cx.emit(EnvironmentEvent::EnvironmentModified(env_clone));
        }
        cx.notify();
    }

    fn delete_variable(&mut self, key: &str, cx: &mut Context<Self>) {
        if let Some(env) = self.environments.get_mut(self.selected_edit_index) {
            env.variables.swap_remove(key);
            let env_clone = env.clone();
            cx.emit(EnvironmentEvent::EnvironmentModified(env_clone));
        }
    }

    fn add_environment(&mut self, name: &str, cx: &mut Context<Self>) {
        let new_env = Environment::new(uuid::Uuid::new_v4().to_string(), name);
        self.environments.push(new_env);
        self.selected_edit_index = self.environments.len() - 1;
        let environments = self.environments.clone();
        let selected_ix = self.selected_edit_index;
        let active_id = self.active_env_id.clone();
        self.env_list.update(cx, |list, cx| {
            list.set_data(environments, selected_ix, active_id, cx);
        });
        cx.notify();
    }
}

impl EventEmitter<EnvironmentEvent> for EnvironmentModal {}

impl Render for EnvironmentModal {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.is_open {
            return div().into_any_element();
        }

        let selected_ix = self.selected_edit_index;
        let selected_env = self.environments.get(selected_ix).cloned();
        let active_id = self.active_env_id.clone();
        let environments = self.environments.clone();

        self.env_list.update(cx, |list, cx| {
            list.set_data(environments, selected_ix, active_id, cx);
        });

        let border_color = cx.theme().border;
        let bg_color = cx.theme().background;
        let muted_color = cx.theme().muted;
        let muted_fg_color = cx.theme().muted_foreground;

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
                    .border_color(border_color)
                    .bg(bg_color)
                    .shadow_lg()
                    .overflow_hidden()
                    // Modal Header
                    .child(
                        h_flex()
                            .h(px(48.))
                            .w_full()
                            .px_4()
                            .border_b_1()
                            .border_color(border_color)
                            .bg(muted_color.opacity(0.2))
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
                            .child(self.env_list.clone())
                            // Right side: Variables Editor
                            .child(match selected_env {
                                Some(env) => self.variables_editor.update(cx, |editor, cx| {
                                    editor.render_editor(&env, cx).into_any_element()
                                }),
                                None => div()
                                    .flex_1()
                                    .items_center()
                                    .justify_center()
                                    .p_8()
                                    .text_sm()
                                    .text_color(muted_fg_color)
                                    .child("Selecciona o crea un entorno para gestionar variables")
                                    .into_any_element(),
                            }),
                    ),
            )
            .into_any_element()
    }
}
