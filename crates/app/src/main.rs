use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use gpui::{
    AppContext, Bounds, Context, Entity, FontWeight, IntoElement, ParentElement as _, Render,
    Styled, Subscription, Window, WindowBounds, WindowOptions, div, point, px, rgb, size,
};
use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName,
    button::{Button, ButtonVariants as _},
    h_flex, v_flex,
};
use kestrel_core::Request;
use kestrel_http::HttpClient;
use kestrel_storage::{load_collection_file, save_collection_file};
use kestrel_ui::views::{
    EditorEvent, EnvironmentEvent, EnvironmentModal, RequestEditor, ResponsePanel, Sidebar,
    SidebarEvent, TabBarEvent, TabItem, WorkspaceTabBar,
};

struct KestrelWorkspace {
    sidebar: Entity<Sidebar>,
    tab_bar: Entity<WorkspaceTabBar>,
    editor: Entity<RequestEditor>,
    response: Entity<ResponsePanel>,
    env_modal: Entity<EnvironmentModal>,
    http_client: Arc<HttpClient>,
    open_requests: Vec<Request>,
    active_environment_name: String,
    collection_path: Option<PathBuf>,
    _subscriptions: Vec<Subscription>,
}

impl KestrelWorkspace {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let sidebar = cx.new(|cx| Sidebar::new(window, cx));
        let tab_bar = cx.new(|_cx| WorkspaceTabBar::new());
        let editor = cx.new(|cx| RequestEditor::new(window, cx));
        let response = cx.new(|_cx| ResponsePanel::new());
        let env_modal = cx.new(|cx| EnvironmentModal::new(window, cx));

        // Subscriptions
        let sub_sidebar = cx.subscribe_in(
            &sidebar,
            window,
            |this, _, ev: &SidebarEvent, window, cx| match ev {
                SidebarEvent::SelectRequest(req_id) => {
                    this.open_or_select_request(req_id, window, cx);
                }
                SidebarEvent::OpenCollection => {
                    this.open_collection_prompt(window, cx);
                }
                SidebarEvent::SaveCollection => {
                    this.save_current_collection(cx);
                }
            },
        );

        let sub_tabs = cx.subscribe_in(
            &tab_bar,
            window,
            |this, _, ev: &TabBarEvent, window, cx| match ev {
                TabBarEvent::Select(ix) => {
                    this.switch_to_tab(*ix, window, cx);
                }
                TabBarEvent::Close(ix) => {
                    this.close_tab(*ix, window, cx);
                }
            },
        );

        let sub_editor = cx.subscribe_in(
            &editor,
            window,
            |this, _, ev: &EditorEvent, _window, cx| match ev {
                EditorEvent::RequestModified => {
                    let active_ix = this.tab_bar.read(cx).active_index();
                    this.tab_bar.update(cx, |tb, cx| {
                        tb.set_dirty(active_ix, true, cx);
                    });
                }
                EditorEvent::MethodChanged(method) => {
                    let active_ix = this.tab_bar.read(cx).active_index();
                    if let Some(req) = this.open_requests.get_mut(active_ix) {
                        req.method = *method;
                    }
                    this.sync_tabs(active_ix, cx);
                }
                EditorEvent::SaveRequest => {
                    let active_ix = this.tab_bar.read(cx).active_index();
                    this.tab_bar.update(cx, |tb, cx| {
                        tb.set_dirty(active_ix, false, cx);
                    });
                }
                EditorEvent::SendRequest => {
                    this.execute_current_request(cx);
                }
                _ => {}
            },
        );

        let sub_env = cx.subscribe_in(
            &env_modal,
            window,
            |this, _, ev: &EnvironmentEvent, _window, cx| match ev {
                EnvironmentEvent::EnvironmentSelected(env_id) => {
                    this.apply_selected_environment(env_id, cx);
                }
                EnvironmentEvent::EnvironmentModified(_env) => {
                    // Update active environment in editor if currently selected
                    if let Some(active_env) = this.env_modal.read(cx).active_environment().cloned()
                    {
                        this.active_environment_name = active_env.name.clone();
                        this.editor.update(cx, |ed, cx| {
                            ed.set_active_environment(Some(active_env), cx);
                        });
                    }
                }
                EnvironmentEvent::CloseRequested => {}
            },
        );

        // Load example collection by default
        let example_path = PathBuf::from("examples/basic_opencollection.yaml");
        let mut initial_envs = Vec::new();
        if let Ok(col) = load_collection_file(&example_path) {
            initial_envs = col.environments.clone();
            sidebar.update(cx, |s, cx| {
                s.set_collection(col, cx);
            });
        }

        let first_env_id = initial_envs.first().map(|e| e.id.clone());
        let first_env_name = initial_envs
            .first()
            .map(|e| e.name.clone())
            .unwrap_or_else(|| "Sin entorno".to_string());

        let active_env_clone = initial_envs.first().cloned();
        editor.update(cx, |ed, cx| {
            ed.set_active_environment(active_env_clone, cx);
        });

        env_modal.update(cx, |em, cx| {
            em.set_environments(initial_envs, first_env_id, cx);
        });

        let http_client = Arc::new(HttpClient::new());

        let mut workspace = Self {
            sidebar,
            tab_bar,
            editor,
            response,
            env_modal,
            http_client,
            open_requests: Vec::new(),
            active_environment_name: first_env_name,
            collection_path: Some(example_path),
            _subscriptions: vec![sub_sidebar, sub_tabs, sub_editor, sub_env],
        };

        let first_req = workspace.sidebar.read(cx).collection().and_then(|col| {
            fn find_first(items: &[kestrel_core::CollectionItem]) -> Option<String> {
                for item in items {
                    match item {
                        kestrel_core::CollectionItem::Request(r) => return Some(r.id.clone()),
                        kestrel_core::CollectionItem::Folder(f) => {
                            if let Some(id) = find_first(&f.items) {
                                return Some(id);
                            }
                        }
                        kestrel_core::CollectionItem::ErrorNode(_) => {}
                    }
                }
                None
            }
            find_first(&col.items)
        });

        if let Some(req_id) = first_req {
            workspace.open_or_select_request(&req_id, window, cx);
        }

        workspace
    }

    fn apply_selected_environment(&mut self, env_id: &str, cx: &mut Context<Self>) {
        if let Some(env) = self
            .env_modal
            .read(cx)
            .all_environments()
            .iter()
            .find(|e| e.id == env_id)
            .cloned()
        {
            self.active_environment_name = env.name.clone();
            self.editor.update(cx, |ed, cx| {
                ed.set_active_environment(Some(env), cx);
            });
            cx.notify();
        }
    }

    fn open_or_select_request(
        &mut self,
        req_id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // If already open in tabs, select it
        if let Some(pos) = self.open_requests.iter().position(|r| r.id == req_id) {
            self.switch_to_tab(pos, window, cx);
            return;
        }

        // Find in sidebar collection
        let maybe_req = self
            .sidebar
            .read(cx)
            .collection()
            .and_then(|col| col.find_request(req_id))
            .cloned();

        if let Some(req) = maybe_req {
            self.open_requests.push(req.clone());
            let new_ix = self.open_requests.len() - 1;

            self.sync_tabs(new_ix, cx);

            self.editor.update(cx, |ed, cx| {
                ed.set_request(req, window, cx);
            });
        }
    }

    fn switch_to_tab(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(req) = self.open_requests.get(index).cloned() {
            self.editor.update(cx, |ed, cx| {
                ed.set_request(req, window, cx);
            });
            self.sync_tabs(index, cx);
        }
    }

    fn close_tab(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if index < self.open_requests.len() {
            self.open_requests.remove(index);
            let next_ix = if self.open_requests.is_empty() {
                0
            } else {
                index.min(self.open_requests.len() - 1)
            };

            self.sync_tabs(next_ix, cx);

            if let Some(req) = self.open_requests.get(next_ix).cloned() {
                self.editor.update(cx, |ed, cx| {
                    ed.set_request(req, window, cx);
                });
            }
        }
    }

    fn sync_tabs(&mut self, active_ix: usize, cx: &mut Context<Self>) {
        let is_editor_dirty = self.editor.read(cx).is_dirty();
        let tabs = self
            .open_requests
            .iter()
            .enumerate()
            .map(|(ix, r)| TabItem {
                id: r.id.clone(),
                title: r.name.clone(),
                method: r.method,
                is_dirty: if ix == active_ix {
                    is_editor_dirty
                } else {
                    false
                },
            })
            .collect();

        self.tab_bar.update(cx, |tb, cx| {
            tb.set_tabs(tabs, active_ix, cx);
        });
        cx.notify();
    }

    fn execute_current_request(&mut self, cx: &mut Context<Self>) {
        let maybe_req = self.editor.read(cx).build_current_request(cx);
        let Some(req) = maybe_req else {
            return;
        };

        let active_env = self.env_modal.read(cx).active_environment().cloned();
        let client = Arc::clone(&self.http_client);
        let response_entity = self.response.clone();

        // Mark loading state in response panel
        response_entity.update(cx, |resp_panel, cx| {
            resp_panel.set_loading(true, cx);
        });

        cx.spawn(async move |this, cx| {
            let env_ref = active_env.as_ref();
            let envs = if let Some(e) = env_ref {
                vec![e]
            } else {
                Vec::new()
            };

            let result = client.execute(&req, &envs).await;

            let _ = this.update(cx, |this, cx| {
                this.response.update(cx, |resp_panel, cx| match result {
                    Ok(resp) => {
                        resp_panel.set_response(resp, cx);
                    }
                    Err(err) => {
                        resp_panel.set_error(err.to_string(), cx);
                    }
                });
            });
        })
        .detach();
    }

    fn open_collection_prompt(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // As a default convenient path, open the basic opencollection example or current path
        let target_path = self
            .collection_path
            .clone()
            .unwrap_or_else(|| PathBuf::from("examples/basic_opencollection.yaml"));

        if let Ok(col) = load_collection_file(&target_path) {
            let envs = col.environments.clone();
            let first_env_id = envs.first().map(|e| e.id.clone());
            let first_env_name = envs
                .first()
                .map(|e| e.name.clone())
                .unwrap_or_else(|| "Sin entorno".to_string());
            let active_env_clone = envs.first().cloned();

            self.active_environment_name = first_env_name;
            self.collection_path = Some(target_path);

            self.sidebar.update(cx, |s, cx| {
                s.set_collection(col.clone(), cx);
            });

            self.editor.update(cx, |ed, cx| {
                ed.set_active_environment(active_env_clone, cx);
            });

            self.env_modal.update(cx, |em, cx| {
                em.set_environments(envs, first_env_id, cx);
            });

            // Clear open requests and open first
            self.open_requests.clear();
            fn find_first(items: &[kestrel_core::CollectionItem]) -> Option<String> {
                for item in items {
                    match item {
                        kestrel_core::CollectionItem::Request(r) => return Some(r.id.clone()),
                        kestrel_core::CollectionItem::Folder(f) => {
                            if let Some(id) = find_first(&f.items) {
                                return Some(id);
                            }
                        }
                        kestrel_core::CollectionItem::ErrorNode(_) => {}
                    }
                }

                None
            }

            if let Some(req_id) = find_first(&col.items) {
                self.open_or_select_request(&req_id, window, cx);
            }
            cx.notify();
        }
    }

    fn save_current_collection(&mut self, cx: &mut Context<Self>) {
        // First sync current active request from editor into the collection
        if let Some(current_req) = self.editor.read(cx).build_current_request(cx) {
            self.sidebar.update(cx, |s, _cx| {
                if let Some(col) = s.collection_mut() {
                    col.update_request(current_req);
                }
            });
        }

        // Sync environments from env_modal
        let envs: Vec<_> = self.env_modal.read(cx).all_environments().to_vec();
        self.sidebar.update(cx, |s, _cx| {
            if let Some(col) = s.collection_mut() {
                col.environments = envs;
            }
        });

        if let Some(col) = self.sidebar.read(cx).collection() {
            let save_path = self
                .collection_path
                .clone()
                .unwrap_or_else(|| PathBuf::from("examples/basic_opencollection.yaml"));

            if let Err(e) = save_collection_file(&save_path, col) {
                tracing::error!("Error al guardar la colección: {}", e);
            } else {
                tracing::info!("Colección guardada exitosamente en {:?}", save_path);
            }
        }
    }
}

impl Render for KestrelWorkspace {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let env_name = self.active_environment_name.clone();

        div()
            .size_full()
            .relative()
            .child(
                v_flex()
                    .size_full()
                    .bg(theme.background)
                    .text_color(theme.foreground)
                    // 1. Top Window Application Bar (Collection, Quick Switcher, Environment Selector)
                    .child(
                        h_flex()
                            .h(px(42.))
                            .w_full()
                            .px_3()
                            .border_b_1()
                            .border_color(theme.border)
                            .bg(theme.muted.opacity(0.15))
                            .items_center()
                            .justify_between()
                            // Left: Collection indicator
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        h_flex()
                                            .items_center()
                                            .gap_1p5()
                                            .px_2()
                                            .py_1()
                                            .rounded_md()
                                            .border_1()
                                            .border_color(theme.border)
                                            .bg(theme.background)
                                            .text_xs()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child(Icon::new(IconName::Globe))
                                            .child(div().child("Kestrel API Client")),
                                    )
                                    .child(
                                        Button::new("quick-add-btn")
                                            .ghost()
                                            .icon(Icon::new(IconName::Plus)),
                                    ),
                            )
                            // Right: Environment Selector Pill + View controls
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        Button::new("env-selector-pill-btn")
                                            .ghost()
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.env_modal.update(cx, |em, cx| {
                                                    em.open(cx);
                                                });
                                            }))
                                            .child(
                                                h_flex()
                                                    .items_center()
                                                    .gap_1p5()
                                                    .px_2p5()
                                                    .h(px(28.))
                                                    .rounded_md()
                                                    .border_1()
                                                    .border_color(theme.border)
                                                    .bg(theme.background)
                                                    .text_xs()
                                                    .child(
                                                        div()
                                                            .size(px(7.))
                                                            .rounded_full()
                                                            .bg(rgb(0x22c55e)), // Active green dot
                                                    )
                                                    .child(
                                                        div()
                                                            .font_weight(FontWeight::MEDIUM)
                                                            .child(env_name),
                                                    )
                                                    .child(Icon::new(IconName::ChevronDown)),
                                            ),
                                    )
                                    .child(
                                        Button::new("toggle-panels-btn")
                                            .ghost()
                                            .icon(Icon::new(IconName::PanelRight)),
                                    ),
                            ),
                    )
                    // 2. Main Workspace Layout (Sidebar + Request Tabs + Split Editor/Response)
                    .child(
                        h_flex()
                            .flex_1()
                            .size_full()
                            // Left Column: Native Tree Sidebar
                            .child(self.sidebar.clone())
                            // Right Column: Tab Bar + Editor & Response Panels
                            .child(
                                v_flex()
                                    .flex_1()
                                    .size_full()
                                    .child(self.tab_bar.clone())
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .size_full()
                                            // Top Half: Request Editor
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .size_full()
                                                    .child(self.editor.clone()),
                                            )
                                            // Subtle horizontal separator
                                            .child(div().h(px(1.)).w_full().bg(theme.border))
                                            // Bottom Half: Response Panel
                                            .child(
                                                div()
                                                    .h(px(320.))
                                                    .w_full()
                                                    .child(self.response.clone()),
                                            ),
                                    ),
                            ),
                    ),
            )
            // 3. Floating Modal overlay for Environment & Variables Management
            .child(self.env_modal.clone())
    }
}

fn main() -> Result<()> {
    // reqwest and hyper require an active Tokio reactor running on threads executing network I/O
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let _guard = rt.enter();

    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(|cx| {
            gpui_kit::init(cx);

            let window_bounds = Bounds {
                origin: point(px(80.0), px(80.0)),
                size: size(px(1240.0), px(800.0)),
            };
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(window_bounds)),
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some("Kestrel".into()),
                    appears_transparent: false,
                    traffic_light_position: None,
                }),
                ..Default::default()
            };

            gpui_kit::open_window(options, cx, |window, cx| {
                cx.new(|cx| KestrelWorkspace::new(window, cx))
            })
            .expect("Error al abrir ventana principal");
        });

    Ok(())
}
