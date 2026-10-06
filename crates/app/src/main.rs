use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use ely_gpui_component::buttons::IconButton;
use ely_gpui_component::primitives::{Icon, IconName};
use ely_gpui_component::theme::ControlSize;
use gpui::{
    AppContext, Bounds, Context, Entity, FontWeight, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, StatefulInteractiveElement as _, Styled, Subscription, Window,
    WindowBounds, WindowOptions, div, point, px, size,
};
use kestrel_core::Request;
use kestrel_http::HttpClient;
use kestrel_storage::{load_collection_file, save_collection_file};
use kestrel_ui::theme::{ThemeExt as _, h_flex, v_flex};
use kestrel_ui::views::{
    EditorEvent, EnvironmentEvent, EnvironmentModal, RequestEditor, ResponseEvent, ResponsePanel,
    Sidebar, SidebarEvent, TabBarEvent, TabItem, WorkspaceTabBar,
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
        let response = cx.new(|cx| ResponsePanel::new(window, cx));
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
                    if let Some(env) = this
                        .env_modal
                        .read(cx)
                        .all_environments()
                        .iter()
                        .find(|e| e.id == *env_id)
                    {
                        this.active_environment_name = env.name.clone();
                        let env_clone = env.clone();
                        this.editor.update(cx, |ed, cx| {
                            ed.set_active_environment(Some(env_clone), cx);
                        });
                    }
                    cx.notify();
                }
                EnvironmentEvent::EnvironmentModified(_) => {
                    if let Some(env) = this.env_modal.read(cx).active_environment() {
                        let env_clone = env.clone();
                        this.editor.update(cx, |ed, cx| {
                            ed.set_active_environment(Some(env_clone), cx);
                        });
                    }
                    cx.notify();
                }
                EnvironmentEvent::CloseRequested => {
                    cx.notify();
                }
            },
        );

        // Load example collection on startup if present
        let example_path = PathBuf::from("examples/basic_opencollection.yaml");
        let initial_collection = if example_path.exists() {
            match load_collection_file(&example_path) {
                Ok(col) => {
                    sidebar.update(cx, |s, cx| {
                        s.set_collection(col.clone(), cx);
                    });
                    Some(col)
                }
                Err(err) => {
                    tracing::error!("Error al cargar colección inicial: {}", err);
                    None
                }
            }
        } else {
            None
        };

        // Initialize with default environments if collection has environments
        let initial_envs = initial_collection
            .as_ref()
            .map(|c| c.environments.clone())
            .unwrap_or_default();

        let active_env = initial_envs.first().cloned();
        let active_env_name = active_env
            .as_ref()
            .map(|e| e.name.clone())
            .unwrap_or_else(|| "Sin Entorno".to_string());

        let active_env_id = active_env.as_ref().map(|e| e.id.clone());

        env_modal.update(cx, |em, cx| {
            em.set_environments(initial_envs, active_env_id, cx);
        });

        let sub_response = cx.subscribe_in(
            &response,
            window,
            |this, _, ev: &ResponseEvent, _window, cx| match ev {
                ResponseEvent::Retry => {
                    this.execute_current_request(cx);
                }
                ResponseEvent::Cancel => {
                    // Handled inside panel state
                }
                ResponseEvent::SaveToFile => {
                    this.save_response_body_to_disk(cx);
                }
                ResponseEvent::CopyBody => {}
            },
        );

        let mut workspace = Self {
            sidebar,
            tab_bar,
            editor,
            response,
            env_modal,
            http_client: Arc::new(HttpClient::new()),
            open_requests: Vec::new(),
            active_environment_name: active_env_name,
            collection_path: if example_path.exists() {
                Some(example_path)
            } else {
                None
            },
            _subscriptions: vec![sub_sidebar, sub_tabs, sub_editor, sub_env, sub_response],
        };

        let first_req = workspace.sidebar.read(cx).collection().and_then(|col| {
            fn find_first_req(items: &[kestrel_core::CollectionItem]) -> Option<Request> {
                for item in items {
                    match item {
                        kestrel_core::CollectionItem::Request(req) => return Some(req.clone()),
                        kestrel_core::CollectionItem::Folder(folder) => {
                            if let Some(r) = find_first_req(&folder.items) {
                                return Some(r);
                            }
                        }
                        kestrel_core::CollectionItem::ErrorNode(_) => {}
                    }
                }
                None
            }
            find_first_req(&col.items)
        });

        if let Some(req) = first_req {
            workspace.open_requests.push(req.clone());
            workspace.sync_tabs(0, cx);
            workspace.editor.update(cx, |ed, cx| {
                ed.set_request(req, window, cx);
                if let Some(env) = active_env {
                    ed.set_active_environment(Some(env), cx);
                }
            });
        }

        workspace
    }

    fn sync_tabs(&mut self, active_ix: usize, cx: &mut Context<Self>) {
        let tab_items: Vec<TabItem> = self
            .open_requests
            .iter()
            .map(|r| TabItem {
                id: r.id.clone(),
                title: r.name.clone(),
                method: r.method,
                is_dirty: false,
            })
            .collect();

        self.tab_bar.update(cx, |tb, cx| {
            tb.set_tabs(tab_items, active_ix, cx);
        });
    }

    fn open_or_select_request(
        &mut self,
        req_id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // If already open in tabs, switch to it
        if let Some(existing_ix) = self.open_requests.iter().position(|r| r.id == req_id) {
            self.switch_to_tab(existing_ix, window, cx);
            return;
        }

        // Find in sidebar collection
        let req = self.sidebar.read(cx).collection().and_then(|col| {
            fn find_req(
                items: &[kestrel_core::CollectionItem],
                target_id: &str,
            ) -> Option<Request> {
                for item in items {
                    match item {
                        kestrel_core::CollectionItem::Request(r) if r.id == target_id => {
                            return Some(r.clone());
                        }
                        kestrel_core::CollectionItem::Folder(f) => {
                            if let Some(r) = find_req(&f.items, target_id) {
                                return Some(r);
                            }
                        }
                        _ => {}
                    }
                }
                None
            }
            find_req(&col.items, req_id)
        });

        if let Some(r) = req {
            self.open_requests.push(r.clone());
            let new_ix = self.open_requests.len() - 1;
            self.sync_tabs(new_ix, cx);
            self.editor.update(cx, |ed, cx| {
                ed.set_request(r, window, cx);
            });
            self.sidebar.update(cx, |s, cx| {
                s.select_request(req_id.to_string(), cx);
            });
            cx.notify();
        }
    }

    fn switch_to_tab(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if index < self.open_requests.len() {
            let req = self.open_requests[index].clone();
            self.sync_tabs(index, cx);
            self.editor.update(cx, |ed, cx| {
                ed.set_request(req.clone(), window, cx);
            });
            self.sidebar.update(cx, |s, cx| {
                s.select_request(req.id, cx);
            });
            cx.notify();
        }
    }

    fn close_tab(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if index < self.open_requests.len() {
            self.open_requests.remove(index);

            if self.open_requests.is_empty() {
                self.sync_tabs(0, cx);
                self.editor.update(cx, |ed, cx| {
                    ed.set_request(
                        Request::new(uuid::Uuid::new_v4().to_string(), "Nueva Solicitud"),
                        window,
                        cx,
                    );
                });
            } else {
                let next_ix = if index >= self.open_requests.len() {
                    self.open_requests.len() - 1
                } else {
                    index
                };
                self.switch_to_tab(next_ix, window, cx);
            }
            cx.notify();
        }
    }

    fn execute_current_request(&mut self, cx: &mut Context<Self>) {
        // Fetch fresh Request object from editor
        let Some(req) = self.editor.read(cx).build_current_request(cx) else {
            return;
        };

        let active_env = self.env_modal.read(cx).active_environment().cloned();
        let client = Arc::clone(&self.http_client);
        let response_entity = self.response.clone();

        response_entity.update(cx, |res, cx| {
            res.set_loading(true, cx);
        });

        cx.spawn(async move |_this, cx| {
            let env_refs: Vec<&kestrel_core::Environment> =
                active_env.as_ref().into_iter().collect();
            match client.execute(&req, &env_refs).await {
                Ok(resp) => {
                    cx.update(|cx| {
                        response_entity.update(cx, |res, cx| {
                            res.set_response(resp, cx);
                        });
                    });
                }
                Err(err) => {
                    cx.update(|cx| {
                        response_entity.update(cx, |res, cx| {
                            res.set_error(err.into(), cx);
                        });
                    });
                }
            }
        })
        .detach();
    }

    fn save_response_body_to_disk(&mut self, cx: &mut Context<Self>) {
        if let Some(resp) = self.response.read(cx).current_response() {
            let file_name = format!("response_{}.bin", resp.status);
            let target_path = PathBuf::from(&file_name);
            let _ = std::fs::write(&target_path, &resp.body);
        }
    }

    fn open_collection_prompt(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let example_path = PathBuf::from("examples/basic_opencollection.yaml");
        if example_path.exists() {
            match load_collection_file(&example_path) {
                Ok(col) => {
                    self.sidebar.update(cx, |s, cx| {
                        s.set_collection(col.clone(), cx);
                    });
                    self.env_modal.update(cx, |em, cx| {
                        let active_id = col.environments.first().map(|e| e.id.clone());
                        em.set_environments(col.environments.clone(), active_id, cx);
                    });
                    self.collection_path = Some(example_path);

                    fn find_first(items: &[kestrel_core::CollectionItem]) -> Option<Request> {
                        for item in items {
                            match item {
                                kestrel_core::CollectionItem::Request(r) => return Some(r.clone()),
                                kestrel_core::CollectionItem::Folder(f) => {
                                    if let Some(r) = find_first(&f.items) {
                                        return Some(r);
                                    }
                                }
                                _ => {}
                            }
                        }
                        None
                    }

                    if let Some(r) = find_first(&col.items) {
                        self.open_requests = vec![r.clone()];
                        self.sync_tabs(0, cx);
                        self.editor.update(cx, |ed, cx| {
                            ed.set_request(r, window, cx);
                        });
                    }
                    cx.notify();
                }
                Err(err) => {
                    tracing::error!("Error al cargar colección: {}", err);
                }
            }
        }
    }

    fn save_current_collection(&mut self, cx: &mut Context<Self>) {
        let active_ix = self.tab_bar.read(cx).active_index();
        let current_editor_req = self.editor.read(cx).build_current_request(cx);

        // Update active request in open_requests and in the sidebar collection
        if let Some(req) = current_editor_req {
            if let Some(r) = self.open_requests.get_mut(active_ix) {
                *r = req.clone();
            }

            self.sidebar.update(cx, |s, cx| {
                if let Some(col) = s.collection_mut() {
                    fn update_req(
                        items: &mut [kestrel_core::CollectionItem],
                        updated: &Request,
                    ) -> bool {
                        for item in items.iter_mut() {
                            match item {
                                kestrel_core::CollectionItem::Request(r) if r.id == updated.id => {
                                    *r = updated.clone();
                                    return true;
                                }
                                kestrel_core::CollectionItem::Folder(f) => {
                                    if update_req(&mut f.items, updated) {
                                        return true;
                                    }
                                }
                                _ => {}
                            }
                        }
                        false
                    }
                    update_req(&mut col.items, &req);
                    cx.notify();
                }
            });

            self.editor.update(cx, |ed, cx| {
                ed.mark_saved(cx);
            });
            self.tab_bar.update(cx, |tb, cx| {
                tb.set_dirty(active_ix, false, cx);
            });
        }

        // Sync environments from modal to collection before saving
        let envs = self.env_modal.read(cx).all_environments().to_vec();
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
        let colors = &theme.colors;
        let env_name = self.active_environment_name.clone();

        div()
            .size_full()
            .relative()
            .child(
                v_flex()
                    .size_full()
                    .bg(colors.bg)
                    .text_color(colors.fg)
                    // 1. Top Window Application Bar (Collection, Quick Switcher, Environment Selector)
                    .child(
                        h_flex()
                            .h(px(42.))
                            .w_full()
                            .px_3()
                            .border_b_1()
                            .border_color(colors.border)
                            .bg(colors.sunken)
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
                                            .border_color(colors.border)
                                            .bg(colors.bg)
                                            .text_xs()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child(Icon::new(IconName::Globe).color(colors.accent))
                                            .child(div().child("Kestrel API Client")),
                                    )
                                    .child(
                                        IconButton::new("quick-add-btn", IconName::Plus)
                                            .size(ControlSize::Sm),
                                    ),
                            )
                            // Right: Environment Selector Pill + View controls
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .id("env-selector-pill-btn")
                                            .cursor_pointer()
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
                                                    .border_color(colors.border)
                                                    .bg(colors.bg)
                                                    .hover(|s| s.bg(colors.hover))
                                                    .text_xs()
                                                    .child(
                                                        div()
                                                            .size(px(7.))
                                                            .rounded_full()
                                                            .bg(colors.success), // Active green dot
                                                    )
                                                    .child(
                                                        div()
                                                            .font_weight(FontWeight::MEDIUM)
                                                            .text_color(colors.fg)
                                                            .child(env_name),
                                                    )
                                                    .child(
                                                        Icon::new(IconName::ChevronDown)
                                                            .color(colors.fg_muted),
                                                    ),
                                            ),
                                    )
                                    .child(
                                        IconButton::new("toggle-panels-btn", IconName::PanelRight)
                                            .size(ControlSize::Sm),
                                    ),
                            ),
                    )
                    // 2. Main Workspace Layout (Sidebar + Request Tabs + Side-by-side Editor & Response)
                    .child(
                        h_flex()
                            .flex_1()
                            .min_h_0()
                            .size_full()
                            .overflow_hidden()
                            // Left Column: Native Tree Sidebar
                            .child(
                                div()
                                    .w(px(260.))
                                    .min_w(px(220.))
                                    .h_full()
                                    .flex_shrink_0()
                                    .child(self.sidebar.clone()),
                            )
                            // Right Area: Tab Bar + Editor & Response Panels
                            .child(
                                v_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .h_full()
                                    .overflow_hidden()
                                    .child(self.tab_bar.clone())
                                    .child(
                                        h_flex()
                                            .flex_1()
                                            .min_h_0()
                                            .size_full()
                                            .overflow_hidden()
                                            // Left Pane: Request Editor
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .min_w_0()
                                                    .h_full()
                                                    .overflow_hidden()
                                                    .child(self.editor.clone()),
                                            )
                                            // Right Pane: Response Panel
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .min_w_0()
                                                    .h_full()
                                                    .overflow_hidden()
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

    gpui_platform::application()
        .with_assets(ely_gpui_component::Assets)
        .run(|cx| {
            ely_gpui_component::init(cx);

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

            cx.open_window(options, |window, cx| {
                cx.new(|cx| KestrelWorkspace::new(window, cx))
            })
            .expect("Error al abrir ventana principal");
        });

    Ok(())
}
