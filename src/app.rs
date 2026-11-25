use std::time::{Duration, Instant};

use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::keyboard::ModifiersState;
use tao::window::Window;

use crate::error::BrowserError;
use crate::ipc::{
    AppEvent, BrowserTheme, SearchEngine, format_origin_display, is_internal_blog,
    is_internal_newtab, map_window_shortcut, resolve_navigation_input_with_engine,
    sanitize_profile_name,
};
use crate::layout::ShellLayout;
use crate::shell::{ShellSyncPayload, ShellView};
use crate::tabs::TabManager;
use crate::window::create_main_window;

const SHORTCUT_DEDUP_WINDOW: Duration = Duration::from_millis(45);

struct BrowserRuntime {
    // Drop all tab WebViews first, then the chrome WebView, before destroying the parent HWND.
    tabs: TabManager,
    shell: ShellView,
    window: Window,
    layout: ShellLayout,
    modifiers: ModifiersState,
    last_shortcut_at: Option<Instant>,
    window_focused: bool,
    shell_ready: bool,
    theme: BrowserTheme,
    search_engine: SearchEngine,
    profile_name: String,
}

impl BrowserRuntime {
    fn apply_bounds(&mut self) {
        self.shell.set_bounds(self.layout.chrome_bounds());
        self.tabs.set_bounds(self.layout.content_bounds());
    }

    fn sync_shell(&self) {
        if !self.shell_ready {
            return;
        }
        let active = self.tabs.active_tab();
        let origin_display = format_origin_display(&active.url);
        let summaries = self.tabs.summaries();

        let payload = ShellSyncPayload {
            tabs: &summaries,
            origin_display: &origin_display,
            title: &active.title,
            can_go_back: active.engine.can_go_back(),
            can_go_forward: active.engine.can_go_forward(),
            is_loading: active.is_loading,
            sidebar_width: self.layout.effective_sidebar_width(),
            is_compact: self.layout.is_compact(),
            is_maximized: self.window.is_maximized(),
            window_focused: self.window_focused,
            theme: self.theme.as_wire(),
            profile_name: &self.profile_name,
            search_engine: self.search_engine.as_wire(),
        };
        self.shell.sync_state(&payload);
    }

    fn sync_window_title(&self) {
        let active = self.tabs.active_tab();
        let title = if is_internal_newtab(&active.url) || active.title.is_empty() {
            "Ruthenium".to_string()
        } else {
            format!("{} - Ruthenium", active.title)
        };
        self.window.set_title(&title);
    }

    fn focus_active_surface(&self) {
        let active = self.tabs.active_tab();
        if is_internal_newtab(&active.url) {
            self.shell.focus_address_bar();
        } else {
            active.engine.focus();
        }
    }

    fn accept_shortcut(&mut self) -> bool {
        let now = Instant::now();
        if let Some(prev) = self.last_shortcut_at
            && now.duration_since(prev) < SHORTCUT_DEDUP_WINDOW
        {
            return false;
        }
        self.last_shortcut_at = Some(now);
        true
    }

    fn handle_event(&mut self, app_event: AppEvent) -> bool {
        let is_shortcut_action = matches!(
            app_event,
            AppEvent::NewTab
                | AppEvent::CloseActiveTab
                | AppEvent::NextTab
                | AppEvent::PrevTab
                | AppEvent::SelectTabSlot(_)
        );
        if is_shortcut_action && !self.accept_shortcut() {
            return false;
        }

        match app_event {
            AppEvent::ShellReady => {
                self.shell_ready = true;
                self.tabs
                    .sync_internal_prefs_all(self.search_engine.as_wire(), self.theme);
                self.sync_window_title();
                self.sync_shell();
            }
            AppEvent::Navigate(raw) => {
                let target = resolve_navigation_input_with_engine(&raw, self.search_engine);
                self.tabs.navigate_active(&target);
                self.sync_window_title();
                self.sync_shell();
            }
            AppEvent::Back => {
                self.tabs.active_tab().engine.go_back();
                self.sync_shell();
            }
            AppEvent::Forward => {
                self.tabs.active_tab().engine.go_forward();
                self.sync_shell();
            }
            AppEvent::Reload => {
                self.tabs.active_tab().engine.reload();
                self.sync_shell();
            }
            AppEvent::Stop => {
                if self.tabs.active_tab().is_loading {
                    self.tabs.stop_active();
                    self.sync_shell();
                }
            }
            AppEvent::NewTab => {
                if let Err(err) = self.tabs.create_tab(&self.window, true) {
                    eprintln!("error creating tab: {err}");
                } else {
                    self.sync_window_title();
                    self.sync_shell();
                    self.shell.focus_address_bar();
                }
            }
            AppEvent::SelectTab(id) => {
                if self.tabs.select_tab(id) {
                    self.sync_window_title();
                    self.sync_shell();
                    self.focus_active_surface();
                }
            }
            AppEvent::CloseTab(id) => match self.tabs.close_tab(&self.window, id) {
                Ok(reset_to_newtab) => {
                    self.sync_window_title();
                    self.sync_shell();
                    if reset_to_newtab {
                        self.shell.focus_address_bar();
                    } else {
                        self.focus_active_surface();
                    }
                }
                Err(err) => eprintln!("error closing tab: {err}"),
            },
            AppEvent::CloseActiveTab => match self.tabs.close_active_tab(&self.window) {
                Ok(reset_to_newtab) => {
                    self.sync_window_title();
                    self.sync_shell();
                    if reset_to_newtab {
                        self.shell.focus_address_bar();
                    } else {
                        self.focus_active_surface();
                    }
                }
                Err(err) => eprintln!("error closing active tab: {err}"),
            },
            AppEvent::NextTab => {
                if self.tabs.select_next() {
                    self.sync_window_title();
                    self.sync_shell();
                    self.focus_active_surface();
                }
            }
            AppEvent::PrevTab => {
                if self.tabs.select_prev() {
                    self.sync_window_title();
                    self.sync_shell();
                    self.focus_active_surface();
                }
            }
            AppEvent::SelectTabSlot(slot) => {
                if self.tabs.select_slot(slot) {
                    self.sync_window_title();
                    self.sync_shell();
                    self.focus_active_surface();
                }
            }
            AppEvent::FocusContent => {
                self.tabs.active_tab().engine.focus();
            }
            AppEvent::FocusAddressBar => {
                self.shell.focus_address_bar();
            }
            AppEvent::ToggleSidebar => {
                self.layout.toggle_sidebar();
                self.apply_bounds();
                self.sync_shell();
            }
            AppEvent::SetSidebarOpen(open) => {
                self.layout.set_sidebar_open(open);
                self.apply_bounds();
                self.sync_shell();
            }
            AppEvent::SetSidebarWidth(width) => {
                self.layout.drag_sidebar_to(width);
                self.apply_bounds();
                self.sync_shell();
            }
            AppEvent::ResetSidebarWidth => {
                self.layout.reset_sidebar_width();
                self.apply_bounds();
                self.sync_shell();
            }
            AppEvent::SetTheme(theme) => {
                self.theme = theme;
                self.tabs.set_theme(theme);
                self.sync_shell();
            }
            AppEvent::SetSearchEngine(engine) => {
                if self.search_engine != engine {
                    self.search_engine = engine;
                    self.tabs
                        .sync_internal_prefs_all(self.search_engine.as_wire(), self.theme);
                    self.sync_shell();
                }
            }
            AppEvent::SetProfileName(name) => {
                let clean = sanitize_profile_name(&name);
                if self.profile_name != clean {
                    self.profile_name = clean;
                    self.sync_shell();
                }
            }
            AppEvent::Minimize => {
                self.window.set_minimized(true);
            }
            AppEvent::ToggleMaximize => {
                self.window.set_maximized(!self.window.is_maximized());
                self.sync_shell();
            }
            AppEvent::Close => {
                return true;
            }
            AppEvent::DragWindow => {
                let _ = self.window.drag_window();
            }
            AppEvent::DragResize(dir) => {
                let _ = self.window.drag_resize_window(dir);
            }
            AppEvent::TabNavigated { tab_id, url } => {
                if self.tabs.handle_navigated(tab_id, url) {
                    self.sync_window_title();
                    self.sync_shell();
                }
            }
            AppEvent::TabTitleChanged { tab_id, title } => {
                if self.tabs.handle_title_changed(tab_id, title) {
                    self.sync_window_title();
                    self.sync_shell();
                }
            }
            AppEvent::TabLoadState {
                tab_id,
                loading,
                url,
            } => {
                let is_internal_done = !loading && (is_internal_newtab(&url) || is_internal_blog(&url));
                if self.tabs.handle_load_state(tab_id, loading, url) {
                    if is_internal_done {
                        self.tabs.sync_internal_prefs_tab(
                            tab_id,
                            self.search_engine.as_wire(),
                            self.theme,
                        );
                    }
                    self.sync_window_title();
                    self.sync_shell();
                }
            }
            AppEvent::TabFaviconChanged {
                tab_id,
                favicon_url,
            } => {
                if self.tabs.handle_favicon_changed(tab_id, favicon_url) {
                    self.sync_shell();
                }
            }
        }

        false
    }
}

pub fn run() -> Result<(), BrowserError> {
    let event_loop = EventLoopBuilder::<AppEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    let window = create_main_window(&event_loop)?;
    let logical = window.inner_size().to_logical::<f64>(window.scale_factor());
    let layout = ShellLayout::new(logical.width, logical.height);

    let shell = ShellView::attach(&window, layout.chrome_bounds(), proxy.clone())?;
    let tabs = TabManager::new(&window, layout.content_bounds(), proxy)?;

    let mut runtime = Some(BrowserRuntime {
        tabs,
        shell,
        window,
        layout,
        modifiers: ModifiersState::empty(),
        last_shortcut_at: None,
        window_focused: true,
        shell_ready: false,
        theme: BrowserTheme::default(),
        search_engine: SearchEngine::default(),
        profile_name: String::new(),
    });

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent {
                window_id,
                event: win_event,
                ..
            } => {
                let is_main = runtime
                    .as_ref()
                    .is_some_and(|rt| rt.window.id() == window_id);

                match win_event {
                    WindowEvent::CloseRequested if is_main => {
                        runtime = None;
                    }
                    WindowEvent::Destroyed if runtime.is_none() => {
                        *control_flow = ControlFlow::Exit;
                    }
                    WindowEvent::ModifiersChanged(modifiers) if is_main => {
                        if let Some(rt) = runtime.as_mut() {
                            rt.modifiers = modifiers;
                        }
                    }
                    WindowEvent::KeyboardInput {
                        event: key_event,
                        is_synthetic,
                        ..
                    } if is_main => {
                        let mut request_close = false;
                        if let Some(rt) = runtime.as_mut()
                            && let Some(app_event) =
                                map_window_shortcut(&key_event, rt.modifiers, is_synthetic)
                        {
                            request_close = rt.handle_event(app_event);
                        }
                        if request_close {
                            runtime = None;
                        }
                    }
                    WindowEvent::Resized(physical) if is_main => {
                        if let Some(rt) = runtime.as_mut() {
                            let scale = rt.window.scale_factor();
                            let logical = physical.to_logical::<f64>(scale);
                            if logical.width > 0.0 && logical.height > 0.0 {
                                rt.layout.set_window_size(logical.width, logical.height);
                                rt.apply_bounds();
                                rt.sync_shell();
                            }
                        }
                    }
                    WindowEvent::ScaleFactorChanged {
                        scale_factor,
                        new_inner_size,
                    } if is_main => {
                        if let Some(rt) = runtime.as_mut() {
                            let logical = new_inner_size.to_logical::<f64>(scale_factor);
                            if logical.width > 0.0 && logical.height > 0.0 {
                                rt.layout.set_window_size(logical.width, logical.height);
                                rt.apply_bounds();
                                rt.sync_shell();
                            }
                        }
                    }
                    WindowEvent::Focused(focused) if is_main => {
                        if let Some(rt) = runtime.as_mut() {
                            rt.window_focused = focused;
                            if !focused {
                                rt.modifiers = ModifiersState::empty();
                            }
                            rt.sync_shell();
                        }
                    }
                    _ => {}
                }
            }
            Event::UserEvent(app_event) => {
                let mut request_close = false;
                if let Some(rt) = runtime.as_mut() {
                    request_close = rt.handle_event(app_event);
                }
                if request_close {
                    runtime = None;
                }
            }
            _ => {}
        }
    });
}
