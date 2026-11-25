use std::time::{Duration, Instant};

use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy};
use tao::keyboard::ModifiersState;
#[cfg(target_os = "windows")]
use tao::platform::windows::WindowExtWindows;
use tao::window::Window;

use crate::adblock::AdblockService;
use crate::engine::webview2_runtime_version;
use crate::error::BrowserError;
use crate::ipc::{
    AppEvent, INTERNAL_NEWTAB_URL, INTERNAL_WELCOME_URL, format_origin_display,
    is_internal_blog, is_internal_newtab, is_internal_settings, is_internal_welcome,
    is_recordable_search_query, map_window_shortcut, resolve_navigation_input_with_engine,
    sanitize_profile_name,
};
use crate::layout::ShellLayout;
use crate::onboarding;
use crate::settings::{self, BrowserSettings, StartupMode, normalize_zoom};
use crate::shell::{ShellSyncPayload, ShellView};
use crate::tabs::TabManager;
use crate::window::create_main_window;

const SHORTCUT_DEDUP_WINDOW: Duration = Duration::from_millis(45);

fn is_internal_page_url(url: &str) -> bool {
    !url.is_empty()
        && (is_internal_newtab(url)
            || is_internal_blog(url)
            || is_internal_welcome(url)
            || is_internal_settings(url))
}

fn escape_json_string(value: &str) -> String {
    use std::fmt::Write;
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

struct BrowserRuntime {
    // Drop all tab WebViews first, then the chrome WebView/popup, before destroying the parent HWND.
    tabs: TabManager,
    shell: ShellView,
    window: Window,
    layout: ShellLayout,
    adblock: AdblockService,
    proxy: EventLoopProxy<AppEvent>,
    modifiers: ModifiersState,
    last_shortcut_at: Option<Instant>,
    main_focused: bool,
    popup_focused: bool,
    shell_ready: bool,
    settings: BrowserSettings,
    profile_name: String,
    is_default_browser: bool,
    webview2_version: String,
}

impl BrowserRuntime {
    fn apply_bounds(&mut self) {
        self.shell.set_bounds(self.layout.chrome_bounds());
        self.tabs.set_bounds(self.layout.content_bounds());
    }

    fn close_omnibox(&mut self, blur_input: bool) {
        self.shell.hide_omnibox_popup();
        self.shell.close_omnibox_ui(blur_input);
    }

    fn hwnd_value(&self) -> isize {
        #[cfg(target_os = "windows")]
        {
            self.window.hwnd()
        }
        #[cfg(not(target_os = "windows"))]
        {
            0
        }
    }

    fn build_settings_ui_json(&self) -> String {
        let whitelist_items: Vec<String> = self
            .settings
            .adblock_whitelist
            .iter()
            .map(|domain| escape_json_string(domain))
            .collect();
        let download_dir = self.settings.effective_download_dir();
        let adblock_status = self.adblock.status_text();

        format!(
            concat!(
                "{{",
                "\"theme\":{},",
                "\"searchEngine\":{},",
                "\"startupMode\":{},",
                "\"startupUrl\":{},",
                "\"defaultZoom\":{},",
                "\"showHomeClock\":{},",
                "\"showHomeShortcuts\":{},",
                "\"suggestEnabled\":{},",
                "\"adblockEnabled\":{},",
                "\"adblockWhitelist\":[{}],",
                "\"downloadDir\":{},",
                "\"isDefaultBrowser\":{},",
                "\"adblockStatus\":{},",
                "\"adblockBlockedTotal\":{},",
                "\"adblockRefreshing\":{},",
                "\"webview2Version\":{}",
                "}}"
            ),
            escape_json_string(self.settings.theme.as_wire()),
            escape_json_string(self.settings.search_engine.as_wire()),
            escape_json_string(self.settings.startup_mode.as_wire()),
            escape_json_string(&self.settings.startup_url),
            self.settings.default_zoom,
            self.settings.show_home_clock,
            self.settings.show_home_shortcuts,
            self.settings.suggest_enabled,
            self.settings.adblock_enabled,
            whitelist_items.join(","),
            escape_json_string(&download_dir),
            self.is_default_browser,
            escape_json_string(&adblock_status),
            self.adblock.total_blocked(),
            self.adblock.is_refreshing(),
            escape_json_string(&self.webview2_version),
        )
    }

    fn sync_settings_pages(&self) {
        let json = self.build_settings_ui_json();
        self.tabs.sync_settings_all(&json);
    }

    fn persist_and_sync_all(&self) {
        settings::save_settings(&self.settings);
        self.sync_shell();
        self.sync_settings_pages();
    }

    fn sync_shell(&self) {
        if !self.shell_ready {
            return;
        }
        let active = self.tabs.active_tab();
        let origin_display = format_origin_display(&active.url);
        let summaries = self.tabs.summaries();
        let adblock_status = self.adblock.status_text();
        let adblock_site_whitelisted = self.adblock.is_host_whitelisted(&active.url);

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
            window_focused: self.main_focused || self.popup_focused,
            theme: self.settings.theme.as_wire(),
            profile_name: &self.profile_name,
            search_engine: self.settings.search_engine.as_wire(),
            suggest_enabled: self.settings.suggest_enabled,
            adblock_enabled: self.adblock.is_enabled(),
            adblock_site_whitelisted,
            adblock_blocked_tab: active.blocked_count,
            adblock_blocked_total: self.adblock.total_blocked(),
            adblock_status: &adblock_status,
            adblock_refreshing: self.adblock.is_refreshing(),
            is_default_browser: self.is_default_browser,
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
            self.shell.focus_address_bar(false);
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
                | AppEvent::OpenSettings(_)
        );
        if is_shortcut_action && !self.accept_shortcut() {
            return false;
        }

        match app_event {
            AppEvent::ShellReady => {
                self.shell_ready = true;
                self.shell.set_popup_theme(self.settings.theme.as_wire());
                self.tabs
                    .sync_internal_prefs_all(self.settings.search_engine.as_wire(), self.settings.theme);
                self.tabs.sync_home_prefs_all(
                    self.settings.show_home_clock,
                    self.settings.show_home_shortcuts,
                    self.settings.suggest_enabled,
                );
                self.tabs
                    .sync_default_browser_all(self.is_default_browser);
                self.tabs.set_zoom_all(self.settings.default_zoom);
                self.sync_settings_pages();
                self.sync_window_title();
                self.sync_shell();
            }
            AppEvent::Navigate(raw) => {
                self.close_omnibox(true);
                if is_recordable_search_query(&raw) {
                    let trimmed = raw.trim();
                    self.shell.sync_history_add(trimmed);
                    self.tabs.sync_history_add_all(trimmed);
                }
                let target = resolve_navigation_input_with_engine(&raw, self.settings.search_engine);
                self.tabs.navigate_active(&target);
                self.sync_window_title();
                self.sync_shell();
            }
            AppEvent::Back => {
                self.close_omnibox(true);
                self.tabs.active_tab().engine.go_back();
                self.sync_shell();
            }
            AppEvent::Forward => {
                self.close_omnibox(true);
                self.tabs.active_tab().engine.go_forward();
                self.sync_shell();
            }
            AppEvent::Reload => {
                self.close_omnibox(true);
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
                self.close_omnibox(true);
                if let Err(err) = self.tabs.create_tab(&self.window, true) {
                    eprintln!("error creating tab: {err}");
                } else {
                    self.sync_window_title();
                    self.sync_shell();
                    self.shell.focus_address_bar(false);
                }
            }
            AppEvent::OpenUrlInNewTab(url) => {
                self.close_omnibox(true);
                if let Err(err) = self.tabs.open_url_in_new_tab(&self.window, &url) {
                    eprintln!("error opening url in new tab: {err}");
                } else {
                    self.sync_window_title();
                    self.sync_shell();
                }
            }
            AppEvent::SelectTab(id) => {
                self.close_omnibox(true);
                if self.tabs.select_tab(id) {
                    self.sync_window_title();
                    self.sync_shell();
                    self.focus_active_surface();
                }
            }
            AppEvent::CloseTab(id) => {
                self.close_omnibox(true);
                if self.tabs.close_tab(id) {
                    settings::save_session(&settings::SessionState::default());
                    return true;
                }
                self.sync_window_title();
                self.sync_shell();
                self.focus_active_surface();
            }
            AppEvent::CloseActiveTab => {
                self.close_omnibox(true);
                if self.tabs.close_active_tab() {
                    settings::save_session(&settings::SessionState::default());
                    return true;
                }
                self.sync_window_title();
                self.sync_shell();
                self.focus_active_surface();
            }
            AppEvent::NextTab => {
                self.close_omnibox(true);
                if self.tabs.select_next() {
                    self.sync_window_title();
                    self.sync_shell();
                    self.focus_active_surface();
                }
            }
            AppEvent::PrevTab => {
                self.close_omnibox(true);
                if self.tabs.select_prev() {
                    self.sync_window_title();
                    self.sync_shell();
                    self.focus_active_surface();
                }
            }
            AppEvent::SelectTabSlot(slot) => {
                self.close_omnibox(true);
                if self.tabs.select_slot(slot) {
                    self.sync_window_title();
                    self.sync_shell();
                    self.focus_active_surface();
                }
            }
            AppEvent::FocusContent => {
                self.close_omnibox(true);
                self.tabs.active_tab().engine.focus();
            }
            AppEvent::FocusAddressBar => {
                self.shell.focus_address_bar(true);
            }
            AppEvent::OmniboxOpen { bounds, payload } => {
                self.shell
                    .show_omnibox_popup(&self.window, &bounds, &payload);
            }
            AppEvent::OmniboxSelect(idx) => {
                self.shell.select_omnibox_popup(idx);
            }
            AppEvent::OmniboxClose => {
                self.shell.hide_omnibox_popup();
            }
            AppEvent::OmniboxBlur => {
                if !self.shell.is_pointer_or_focus_in_popup() {
                    self.close_omnibox(true);
                }
            }
            AppEvent::OmniboxPopupGo(raw) => {
                self.close_omnibox(true);
                if is_recordable_search_query(&raw) {
                    let trimmed = raw.trim();
                    self.shell.sync_history_add(trimmed);
                    self.tabs.sync_history_add_all(trimmed);
                }
                let target = resolve_navigation_input_with_engine(&raw, self.settings.search_engine);
                self.tabs.navigate_active(&target);
                self.sync_window_title();
                self.sync_shell();
            }
            AppEvent::OmniboxPopupFill(text) => {
                self.shell.fill_address_bar(&text);
            }
            AppEvent::OmniboxPopupDeleteHistory(text) => {
                self.shell.sync_history_remove(&text);
                self.tabs.sync_history_remove_all(&text);
                self.shell.refresh_omnibox_after_history_change(true);
            }
            AppEvent::OmniboxPopupClearHistory => {
                self.shell.hide_omnibox_popup();
                self.shell.sync_history_clear();
                self.tabs.sync_history_clear_all();
                self.shell.refresh_omnibox_after_history_change(true);
            }
            AppEvent::OmniboxPopupDismiss => {
                self.close_omnibox(true);
            }
            AppEvent::RemoveSearchHistory(text) => {
                self.shell.sync_history_remove(&text);
                self.tabs.sync_history_remove_all(&text);
            }
            AppEvent::ClearSearchHistory => {
                self.shell.sync_history_clear();
                self.tabs.sync_history_clear_all();
                self.tabs
                    .sync_settings_feedback_all("Riwayat pencarian berhasil dihapus.");
            }
            AppEvent::ToggleSidebar => {
                self.close_omnibox(true);
                self.layout.toggle_sidebar();
                self.apply_bounds();
                self.sync_shell();
            }
            AppEvent::SetSidebarOpen(open) => {
                self.close_omnibox(true);
                self.layout.set_sidebar_open(open);
                self.apply_bounds();
                self.sync_shell();
            }
            AppEvent::SetSidebarWidth(width) => {
                self.close_omnibox(true);
                self.layout.drag_sidebar_to(width);
                self.apply_bounds();
                self.sync_shell();
            }
            AppEvent::ResetSidebarWidth => {
                self.close_omnibox(true);
                self.layout.reset_sidebar_width();
                self.apply_bounds();
                self.sync_shell();
            }
            AppEvent::SetTheme(theme) => {
                self.settings.theme = theme;
                self.tabs.set_theme(theme);
                self.shell.set_popup_theme(theme.as_wire());
                self.persist_and_sync_all();
            }
            AppEvent::SetSearchEngine(engine) => {
                if self.settings.search_engine != engine {
                    self.settings.search_engine = engine;
                    self.tabs.sync_internal_prefs_all(
                        self.settings.search_engine.as_wire(),
                        self.settings.theme,
                    );
                    self.persist_and_sync_all();
                }
            }
            AppEvent::SetProfileName(name) => {
                let clean = sanitize_profile_name(&name);
                if self.profile_name != clean {
                    self.profile_name = clean;
                    self.sync_shell();
                }
            }
            AppEvent::SetAdblockEnabled(enabled) => {
                self.settings.adblock_enabled = enabled;
                self.adblock.set_enabled(enabled);
                self.tabs.sync_adblock_state_all();
                self.persist_and_sync_all();
            }
            AppEvent::RefreshAdblockLists => {
                self.adblock.start_background_load(self.proxy.clone(), true);
                self.sync_shell();
                self.sync_settings_pages();
            }
            AppEvent::AdblockUpdated => {
                if self.adblock.is_enabled() {
                    self.tabs.sync_adblock_state_all();
                }
                self.sync_shell();
                self.sync_settings_pages();
            }
            AppEvent::OpenSettings(section) => {
                self.close_omnibox(true);
                match self
                    .tabs
                    .open_or_select_settings(&self.window, section.as_deref().filter(|s| !s.is_empty()))
                {
                    Ok(_) => {
                        self.sync_settings_pages();
                        self.sync_window_title();
                        self.sync_shell();
                        self.focus_active_surface();
                    }
                    Err(err) => eprintln!("error opening settings: {err}"),
                }
            }
            AppEvent::ToggleActiveSiteAdblock => {
                self.close_omnibox(true);
                let active_url = self.tabs.active_tab().url.clone();
                if is_internal_page_url(&active_url)
                    || self.adblock.toggle_site_whitelist(&active_url).is_none()
                {
                    match self
                        .tabs
                        .open_or_select_settings(&self.window, Some("privacy"))
                    {
                        Ok(_) => {
                            self.sync_settings_pages();
                            self.sync_window_title();
                            self.sync_shell();
                            self.focus_active_surface();
                        }
                        Err(err) => eprintln!("error opening settings: {err}"),
                    }
                } else {
                    self.settings.adblock_whitelist = self.adblock.whitelist();
                    self.tabs.sync_adblock_state_all();
                    self.persist_and_sync_all();
                }
            }
            AppEvent::SetStartupMode(mode) => {
                if let Some(parsed) = StartupMode::from_wire(&mode)
                    && self.settings.startup_mode != parsed
                {
                    self.settings.startup_mode = parsed;
                    self.persist_and_sync_all();
                }
            }
            AppEvent::SetStartupUrl(url) => {
                let trimmed = url.trim().to_string();
                if self.settings.startup_url != trimmed {
                    self.settings.startup_url = trimmed;
                    self.persist_and_sync_all();
                }
            }
            AppEvent::SetDefaultZoom(zoom) => {
                let normalized = normalize_zoom(zoom);
                if self.settings.default_zoom != normalized {
                    self.settings.default_zoom = normalized;
                    self.tabs.set_zoom_all(normalized);
                    self.persist_and_sync_all();
                }
            }
            AppEvent::SetShowHomeClock(show) => {
                if self.settings.show_home_clock != show {
                    self.settings.show_home_clock = show;
                    self.tabs.sync_home_prefs_all(
                        self.settings.show_home_clock,
                        self.settings.show_home_shortcuts,
                        self.settings.suggest_enabled,
                    );
                    self.persist_and_sync_all();
                }
            }
            AppEvent::SetShowHomeShortcuts(show) => {
                if self.settings.show_home_shortcuts != show {
                    self.settings.show_home_shortcuts = show;
                    self.tabs.sync_home_prefs_all(
                        self.settings.show_home_clock,
                        self.settings.show_home_shortcuts,
                        self.settings.suggest_enabled,
                    );
                    self.persist_and_sync_all();
                }
            }
            AppEvent::SetSuggestEnabled(enabled) => {
                if self.settings.suggest_enabled != enabled {
                    self.settings.suggest_enabled = enabled;
                    self.tabs.sync_home_prefs_all(
                        self.settings.show_home_clock,
                        self.settings.show_home_shortcuts,
                        self.settings.suggest_enabled,
                    );
                    self.persist_and_sync_all();
                }
            }
            AppEvent::AddAdblockWhitelist(domain) => {
                if self.adblock.add_whitelist_domain(&domain).is_some() {
                    self.settings.adblock_whitelist = self.adblock.whitelist();
                    self.tabs.sync_adblock_state_all();
                    self.persist_and_sync_all();
                } else {
                    self.tabs
                        .sync_settings_feedback_all("Domain tidak valid atau sudah ada di daftar.");
                }
            }
            AppEvent::RemoveAdblockWhitelist(domain) => {
                self.adblock.remove_whitelist_domain(&domain);
                self.settings.adblock_whitelist = self.adblock.whitelist();
                self.tabs.sync_adblock_state_all();
                self.persist_and_sync_all();
            }
            AppEvent::ClearBrowsingData {
                cache,
                cookies,
                search_history,
            } => {
                if search_history {
                    self.shell.hide_omnibox_popup();
                    self.shell.sync_history_clear();
                    self.tabs.sync_history_clear_all();
                }
                if cache || cookies {
                    self.tabs.clear_browsing_data(cache, cookies);
                } else {
                    let _ = self.proxy.send_event(AppEvent::BrowsingDataCleared(true));
                }
            }
            AppEvent::BrowsingDataCleared(ok) => {
                let msg = if ok {
                    "Data penjelajahan berhasil dihapus."
                } else {
                    "Sebagian data penjelajahan gagal dihapus."
                };
                self.tabs.sync_settings_feedback_all(msg);
            }
            AppEvent::PickDownloadFolder => {
                if let Some(folder) = settings::pick_download_folder(
                    self.hwnd_value(),
                    "Pilih folder default unduhan Ruthenium",
                ) {
                    self.settings.download_dir = folder;
                    self.tabs
                        .apply_download_folder(&self.settings.effective_download_dir());
                    self.persist_and_sync_all();
                    self.tabs
                        .sync_settings_feedback_all("Folder unduhan berhasil diperbarui.");
                }
            }
            AppEvent::OpenDownloadFolder => {
                settings::open_folder_in_explorer(&self.settings.effective_download_dir());
            }
            AppEvent::ResetAllSettings => {
                self.settings = BrowserSettings::default();
                self.adblock.set_enabled(self.settings.adblock_enabled);
                self.adblock.set_whitelist(&self.settings.adblock_whitelist);
                self.tabs.set_theme(self.settings.theme);
                self.shell.set_popup_theme(self.settings.theme.as_wire());
                self.tabs.sync_internal_prefs_all(
                    self.settings.search_engine.as_wire(),
                    self.settings.theme,
                );
                self.tabs.sync_home_prefs_all(
                    self.settings.show_home_clock,
                    self.settings.show_home_shortcuts,
                    self.settings.suggest_enabled,
                );
                self.tabs.set_zoom_all(self.settings.default_zoom);
                self.tabs
                    .apply_download_folder(&self.settings.effective_download_dir());
                self.tabs.sync_adblock_state_all();
                self.persist_and_sync_all();
                self.tabs
                    .sync_settings_feedback_all("Pengaturan dikembalikan ke bawaan");
            }
            AppEvent::WelcomeComplete => {
                if let Err(err) = onboarding::mark_first_run_completed() {
                    eprintln!("error saving first-run marker: {err}");
                }
                self.close_omnibox(true);
                self.tabs.navigate_active(INTERNAL_NEWTAB_URL);
                self.sync_window_title();
                self.sync_shell();
                self.shell.focus_address_bar(false);
            }
            AppEvent::OpenDefaultBrowserSettings => {
                onboarding::open_windows_default_apps_settings();
            }
            AppEvent::OpenWelcomeGuide => {
                self.close_omnibox(true);
                self.tabs.navigate_active(INTERNAL_WELCOME_URL);
                self.sync_window_title();
                self.sync_shell();
            }
            AppEvent::TabAdBlocked { tab_id, count } => {
                if self.tabs.update_tab_blocked_count(tab_id, count)
                    && tab_id == self.tabs.active_tab().id
                {
                    self.sync_shell();
                }
            }
            AppEvent::CosmeticProbe {
                tab_id,
                classes,
                ids,
            } => {
                if self.adblock.is_enabled() {
                    self.tabs
                        .apply_generic_cosmetic_for_tab(tab_id, &classes, &ids);
                }
            }
            AppEvent::Minimize => {
                self.close_omnibox(true);
                self.window.set_minimized(true);
            }
            AppEvent::ToggleMaximize => {
                self.close_omnibox(true);
                self.window.set_maximized(!self.window.is_maximized());
                self.sync_shell();
            }
            AppEvent::Close => {
                if self.settings.startup_mode == StartupMode::RestoreSession {
                    settings::save_session(&self.tabs.snapshot_session());
                }
                return true;
            }
            AppEvent::DragWindow => {
                self.close_omnibox(true);
                let _ = self.window.drag_window();
            }
            AppEvent::DragResize(dir) => {
                self.close_omnibox(true);
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
                let is_welcome_url = !url.is_empty() && is_internal_welcome(&url);
                let is_settings_url = !url.is_empty() && is_internal_settings(&url);
                let is_internal_url = is_internal_page_url(&url);
                if self.tabs.handle_load_state(tab_id, loading, url) {
                    if !loading && is_internal_url {
                        self.tabs.apply_zoom_for_tab(tab_id, 100);
                        self.tabs.sync_internal_prefs_tab(
                            tab_id,
                            self.settings.search_engine.as_wire(),
                            self.settings.theme,
                        );
                        self.tabs.sync_home_prefs_tab(
                            tab_id,
                            self.settings.show_home_clock,
                            self.settings.show_home_shortcuts,
                            self.settings.suggest_enabled,
                        );
                        if is_welcome_url {
                            self.tabs
                                .sync_default_browser_tab(tab_id, self.is_default_browser);
                        }
                        if is_settings_url {
                            let json = self.build_settings_ui_json();
                            self.tabs.sync_settings_tab(tab_id, &json);
                        }
                    } else if !is_internal_url {
                        self.tabs
                            .apply_zoom_for_tab(tab_id, self.settings.default_zoom);
                        self.tabs.sync_adblock_state_tab(tab_id);
                        if !loading {
                            self.tabs.sync_tab_blocked_count(tab_id);
                        }
                        if self.adblock.is_enabled() {
                            self.tabs.apply_cosmetic_for_tab(tab_id);
                        }
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
    #[cfg(target_os = "windows")]
    set_windows_app_user_model_id("Ruthenium.Browser");

    onboarding::register_default_browser_hkcu();
    let is_default_browser = onboarding::is_default_browser_windows();
    let settings = settings::load_settings();
    let webview2_version = webview2_runtime_version();

    let mut restore_session: Option<settings::SessionState> = None;
    let startup_url =
        if let Some(cli_url) = onboarding::parse_startup_url_from_args(std::env::args()) {
            cli_url
        } else if !onboarding::is_first_run_completed() {
            INTERNAL_WELCOME_URL.to_string()
        } else {
            match settings.startup_mode {
                StartupMode::RestoreSession => {
                    let session = settings::load_session();
                    if let Some(first_url) = session.urls.first().cloned() {
                        restore_session = Some(session);
                        first_url
                    } else {
                        INTERNAL_NEWTAB_URL.to_string()
                    }
                }
                StartupMode::NewTab => INTERNAL_NEWTAB_URL.to_string(),
                StartupMode::CustomUrl => {
                    let trimmed = settings.startup_url.trim();
                    if trimmed.is_empty() {
                        INTERNAL_NEWTAB_URL.to_string()
                    } else {
                        resolve_navigation_input_with_engine(trimmed, settings.search_engine)
                    }
                }
            }
        };

    let event_loop = EventLoopBuilder::<AppEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    let window = create_main_window(&event_loop)?;
    let logical = window.inner_size().to_logical::<f64>(window.scale_factor());
    let layout = ShellLayout::new(logical.width, logical.height);

    let adblock = AdblockService::new();
    adblock.set_enabled(settings.adblock_enabled);
    adblock.set_whitelist(&settings.adblock_whitelist);
    adblock.start_background_load(proxy.clone(), false);

    let shell = ShellView::attach(&event_loop, &window, layout.chrome_bounds(), proxy.clone())?;
    #[cfg(target_os = "windows")]
    let mut tabs = TabManager::new(
        &window,
        layout.content_bounds(),
        &startup_url,
        shell.shared_environment(),
        adblock.clone(),
        proxy.clone(),
    )?;
    #[cfg(not(target_os = "windows"))]
    let mut tabs = TabManager::new(
        &window,
        layout.content_bounds(),
        &startup_url,
        adblock.clone(),
        proxy.clone(),
    )?;

    tabs.set_theme(settings.theme);
    tabs.apply_download_folder(&settings.effective_download_dir());
    if let Some(session) = restore_session
        && (session.urls.len() > 1 || session.active_index > 0)
        && let Err(err) = tabs.restore_session_tabs(&window, &session.urls, session.active_index)
    {
        eprintln!("error restoring session tabs: {err}");
    }

    let mut runtime = Some(BrowserRuntime {
        tabs,
        shell,
        window,
        layout,
        adblock,
        proxy,
        modifiers: ModifiersState::empty(),
        last_shortcut_at: None,
        main_focused: true,
        popup_focused: false,
        shell_ready: false,
        settings,
        profile_name: String::new(),
        is_default_browser,
        webview2_version,
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
                let is_popup = runtime
                    .as_ref()
                    .is_some_and(|rt| rt.shell.popup_window_id() == window_id);

                match win_event {
                    WindowEvent::CloseRequested if is_main => {
                        if let Some(rt) = runtime.as_ref()
                            && rt.settings.startup_mode == StartupMode::RestoreSession
                        {
                            settings::save_session(&rt.tabs.snapshot_session());
                        }
                        runtime = None;
                    }
                    WindowEvent::CloseRequested if is_popup => {
                        if let Some(rt) = runtime.as_mut() {
                            rt.close_omnibox(true);
                        }
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
                    WindowEvent::Moved(_) if is_main => {
                        if let Some(rt) = runtime.as_mut() {
                            rt.close_omnibox(true);
                        }
                    }
                    WindowEvent::Resized(physical) if is_main => {
                        if let Some(rt) = runtime.as_mut() {
                            rt.close_omnibox(true);
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
                            rt.close_omnibox(true);
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
                            rt.main_focused = focused;
                            if focused {
                                let now_default = onboarding::is_default_browser_windows();
                                if rt.is_default_browser != now_default {
                                    rt.is_default_browser = now_default;
                                    rt.tabs.sync_default_browser_all(now_default);
                                    rt.sync_settings_pages();
                                }
                            } else {
                                rt.modifiers = ModifiersState::empty();
                                if !rt.shell.is_popup_foreground() {
                                    rt.close_omnibox(true);
                                }
                            }
                            rt.sync_shell();
                        }
                    }
                    WindowEvent::Focused(focused) if is_popup => {
                        if let Some(rt) = runtime.as_mut() {
                            rt.popup_focused = focused;
                            if !focused && !rt.main_focused {
                                rt.close_omnibox(true);
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

#[cfg(target_os = "windows")]
fn set_windows_app_user_model_id(app_id: &str) {
    #[link(name = "shell32")]
    unsafe extern "system" {
        fn SetCurrentProcessExplicitAppUserModelID(app_id: *const u16) -> i32;
    }

    let wide: Vec<u16> = app_id.encode_utf16().chain(std::iter::once(0)).collect();
    // SAFETY: `wide` is a valid NUL-terminated UTF-16 string that outlives the call.
    unsafe {
        let _ = SetCurrentProcessExplicitAppUserModelID(wide.as_ptr());
    }
}
