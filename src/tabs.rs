use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use tao::event_loop::EventLoopProxy;
use tao::window::Window;
#[cfg(target_os = "windows")]
use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Environment;
use wry::Rect;
#[cfg(not(target_os = "windows"))]
use wry::WebContext;

use crate::adblock::AdblockService;
#[cfg(not(target_os = "windows"))]
use crate::engine::content_data_dir;
use crate::engine::{ContentAttachOptions, ContentEngine};
use crate::error::BrowserError;
use crate::ipc::{
    AppEvent, BrowserTheme, INTERNAL_BLOG_URL, INTERNAL_NEWTAB_URL, INTERNAL_SETTINGS_URL,
    INTERNAL_WELCOME_URL, TabId, format_origin_display, is_internal_blog, is_internal_newtab,
    is_internal_settings, is_internal_welcome,
};
use crate::settings::SessionState;
use crate::shell::ShellTabSummary;

pub trait TabRuntimeOps {
    fn set_bounds(&self, bounds: Rect);
    fn set_visible(&self, visible: bool);
}

impl TabRuntimeOps for ContentEngine {
    fn set_bounds(&self, bounds: Rect) {
        self.set_bounds(bounds);
    }

    fn set_visible(&self, visible: bool) {
        self.set_visible(visible);
    }
}

pub struct TabEntry<E> {
    pub id: TabId,
    pub url: String,
    pub title: String,
    pub favicon_url: Option<String>,
    pub is_loading: bool,
    pub blocked_count: u64,
    blocked_counter: Arc<AtomicU64>,
    pub engine: E,
}

impl<E> TabEntry<E> {
    fn new(id: TabId, engine: E) -> Self {
        Self {
            id,
            url: INTERNAL_NEWTAB_URL.to_string(),
            title: "Beranda".to_string(),
            favicon_url: None,
            is_loading: false,
            blocked_count: 0,
            blocked_counter: Arc::new(AtomicU64::new(0)),
            engine,
        }
    }
}

pub struct TabCollection<E> {
    tabs: Vec<TabEntry<E>>,
    active_id: TabId,
    bounds: Rect,
}

impl<E: TabRuntimeOps> TabCollection<E> {
    pub fn new(initial_id: TabId, initial_engine: E, bounds: Rect) -> Self {
        initial_engine.set_bounds(bounds);
        initial_engine.set_visible(true);
        Self {
            tabs: vec![TabEntry::new(initial_id, initial_engine)],
            active_id: initial_id,
            bounds,
        }
    }

    pub fn tabs(&self) -> &[TabEntry<E>] {
        &self.tabs
    }

    pub fn active_id(&self) -> TabId {
        self.active_id
    }

    pub fn active_tab(&self) -> &TabEntry<E> {
        self.tabs
            .iter()
            .find(|t| t.id == self.active_id)
            .expect("TabCollection always maintains one active tab")
    }

    pub fn tab_mut(&mut self, id: TabId) -> Option<&mut TabEntry<E>> {
        self.tabs.iter_mut().find(|t| t.id == id)
    }

    pub fn bounds(&self) -> Rect {
        self.bounds
    }

    pub fn set_bounds(&mut self, bounds: Rect) {
        self.bounds = bounds;
        self.active_tab().engine.set_bounds(bounds);
    }

    pub fn insert_tab(&mut self, id: TabId, engine: E, activate: bool) {
        if activate {
            self.active_tab().engine.set_visible(false);
            engine.set_bounds(self.bounds);
            engine.set_visible(true);
            self.active_id = id;
        } else {
            engine.set_visible(false);
        }
        self.tabs.push(TabEntry::new(id, engine));
    }

    pub fn select_tab(&mut self, id: TabId) -> bool {
        if id == self.active_id || !self.tabs.iter().any(|t| t.id == id) {
            return false;
        }
        let bounds = self.bounds;
        let prev_id = self.active_id;

        if let Some(next) = self.tabs.iter().find(|t| t.id == id) {
            next.engine.set_bounds(bounds);
            next.engine.set_visible(true);
        }
        if let Some(prev) = self.tabs.iter().find(|t| t.id == prev_id) {
            prev.engine.set_visible(false);
        }
        self.active_id = id;
        true
    }

    pub fn select_next(&mut self) -> bool {
        if self.tabs.len() <= 1 {
            return false;
        }
        let idx = self.active_index();
        let next_id = self.tabs[(idx + 1) % self.tabs.len()].id;
        self.select_tab(next_id)
    }

    pub fn select_prev(&mut self) -> bool {
        if self.tabs.len() <= 1 {
            return false;
        }
        let idx = self.active_index();
        let prev_idx = if idx == 0 {
            self.tabs.len() - 1
        } else {
            idx - 1
        };
        let prev_id = self.tabs[prev_idx].id;
        self.select_tab(prev_id)
    }

    pub fn select_slot(&mut self, slot: usize) -> bool {
        if !(1..=9).contains(&slot) || self.tabs.is_empty() {
            return false;
        }
        let target_idx = if slot == 9 {
            self.tabs.len() - 1
        } else if slot - 1 < self.tabs.len() {
            slot - 1
        } else {
            return false;
        };
        let id = self.tabs[target_idx].id;
        self.select_tab(id)
    }

    pub fn remove_non_last_tab(&mut self, id: TabId) -> Option<TabEntry<E>> {
        if self.tabs.len() <= 1 {
            return None;
        }
        let idx = self.tabs.iter().position(|t| t.id == id)?;

        if id == self.active_id {
            let next_idx = if idx + 1 < self.tabs.len() {
                idx + 1
            } else {
                idx - 1
            };
            let next_id = self.tabs[next_idx].id;
            let bounds = self.bounds;
            if let Some(next) = self.tabs.iter().find(|t| t.id == next_id) {
                next.engine.set_bounds(bounds);
                next.engine.set_visible(true);
            }
            self.active_id = next_id;
        }

        let removed = self.tabs.remove(idx);
        removed.engine.set_visible(false);
        Some(removed)
    }

    #[cfg(test)]
    pub fn replace_last_tab(
        &mut self,
        replacement_id: TabId,
        replacement_engine: E,
    ) -> TabEntry<E> {
        debug_assert_eq!(self.tabs.len(), 1);
        replacement_engine.set_bounds(self.bounds);
        replacement_engine.set_visible(true);

        let old = self.tabs.remove(0);
        old.engine.set_visible(false);
        self.tabs
            .push(TabEntry::new(replacement_id, replacement_engine));
        self.active_id = replacement_id;
        old
    }

    fn active_index(&self) -> usize {
        self.tabs
            .iter()
            .position(|t| t.id == self.active_id)
            .expect("active_id must exist in tabs")
    }
}

pub struct TabManager {
    // Drop all child WebView instances before dropping the shared WebContext/environment.
    collection: TabCollection<ContentEngine>,
    next_id: u64,
    theme: BrowserTheme,
    adblock: AdblockService,
    #[cfg(target_os = "windows")]
    shared_env: ICoreWebView2Environment,
    #[cfg(not(target_os = "windows"))]
    context: WebContext,
    proxy: EventLoopProxy<AppEvent>,
}

impl TabManager {
    pub fn new(
        window: &Window,
        initial_bounds: Rect,
        initial_url: &str,
        #[cfg(target_os = "windows")] shared_env: ICoreWebView2Environment,
        adblock: AdblockService,
        proxy: EventLoopProxy<AppEvent>,
    ) -> Result<Self, BrowserError> {
        #[cfg(not(target_os = "windows"))]
        let mut context = WebContext::new(Some(content_data_dir()));
        let first_id = TabId(1);
        let theme = BrowserTheme::default();
        let first_counter = Arc::new(AtomicU64::new(0));
        let first_engine = ContentEngine::attach(
            window,
            ContentAttachOptions {
                tab_id: first_id,
                initial_url,
                #[cfg(target_os = "windows")]
                shared_env: &shared_env,
                #[cfg(not(target_os = "windows"))]
                context: &mut context,
                bounds: initial_bounds,
                visible: true,
                is_dark: theme.is_dark(),
                adblock: adblock.clone(),
                blocked_counter: Arc::clone(&first_counter),
                proxy: proxy.clone(),
            },
        )?;

        let mut collection = TabCollection::new(first_id, first_engine, initial_bounds);
        if let Some(tab) = collection.tab_mut(first_id) {
            tab.blocked_counter = first_counter;
            tab.url = canonical_url_for_state(initial_url);
            tab.title = initial_title_for_url(&tab.url);
        }

        Ok(Self {
            collection,
            next_id: 2,
            theme,
            adblock,
            #[cfg(target_os = "windows")]
            shared_env,
            #[cfg(not(target_os = "windows"))]
            context,
            proxy,
        })
    }

    pub fn set_theme(&mut self, theme: BrowserTheme) {
        self.theme = theme;
        let wire = theme.as_wire();
        for tab in self.collection.tabs() {
            if is_internal_page(&tab.url) {
                tab.engine.sync_theme(wire);
            }
        }
    }

    pub fn sync_internal_prefs_all(&self, engine: &str, theme: BrowserTheme) {
        let wire = theme.as_wire();
        for tab in self.collection.tabs() {
            if is_internal_page(&tab.url) {
                tab.engine.sync_internal_prefs(engine, wire);
            }
        }
    }

    pub fn sync_internal_prefs_tab(&self, tab_id: TabId, engine: &str, theme: BrowserTheme) {
        let wire = theme.as_wire();
        if let Some(tab) = self.collection.tabs().iter().find(|t| t.id == tab_id)
            && is_internal_page(&tab.url)
        {
            tab.engine.sync_internal_prefs(engine, wire);
        }
    }

    pub fn sync_default_browser_all(&self, is_default: bool) {
        for tab in self.collection.tabs() {
            if is_internal_welcome(&tab.url) {
                tab.engine.sync_default_browser(is_default);
            }
        }
    }

    pub fn sync_default_browser_tab(&self, tab_id: TabId, is_default: bool) {
        if let Some(tab) = self.collection.tabs().iter().find(|t| t.id == tab_id)
            && is_internal_welcome(&tab.url)
        {
            tab.engine.sync_default_browser(is_default);
        }
    }

    pub fn sync_home_prefs_all(&self, show_clock: bool, show_shortcuts: bool, suggest_enabled: bool) {
        for tab in self.collection.tabs() {
            if is_internal_newtab(&tab.url) {
                tab.engine
                    .sync_home_prefs(show_clock, show_shortcuts, suggest_enabled);
            }
        }
    }

    pub fn sync_home_prefs_tab(
        &self,
        tab_id: TabId,
        show_clock: bool,
        show_shortcuts: bool,
        suggest_enabled: bool,
    ) {
        if let Some(tab) = self.collection.tabs().iter().find(|t| t.id == tab_id)
            && is_internal_newtab(&tab.url)
        {
            tab.engine
                .sync_home_prefs(show_clock, show_shortcuts, suggest_enabled);
        }
    }

    pub fn sync_settings_all(&self, json: &str) {
        for tab in self.collection.tabs() {
            if is_internal_settings(&tab.url) {
                tab.engine.sync_settings_json(json);
            }
        }
    }

    pub fn sync_settings_tab(&self, tab_id: TabId, json: &str) {
        if let Some(tab) = self.collection.tabs().iter().find(|t| t.id == tab_id)
            && is_internal_settings(&tab.url)
        {
            tab.engine.sync_settings_json(json);
        }
    }

    pub fn sync_settings_feedback_all(&self, msg: &str) {
        for tab in self.collection.tabs() {
            if is_internal_settings(&tab.url) {
                tab.engine.sync_settings_feedback(msg);
            }
        }
    }

    pub fn set_zoom_all(&self, zoom_percent: u16) {
        for tab in self.collection.tabs() {
            let zoom = if is_internal_page(&tab.url) {
                100
            } else {
                zoom_percent
            };
            tab.engine.set_zoom(zoom);
        }
    }

    pub fn apply_zoom_for_tab(&self, tab_id: TabId, zoom_percent: u16) {
        if let Some(tab) = self.collection.tabs().iter().find(|t| t.id == tab_id) {
            let zoom = if is_internal_page(&tab.url) {
                100
            } else {
                zoom_percent
            };
            tab.engine.set_zoom(zoom);
        }
    }

    pub fn apply_download_folder(&self, path: &str) {
        self.collection.active_tab().engine.set_download_folder(path);
    }

    pub fn clear_browsing_data(&self, cache: bool, cookies: bool) {
        self.collection
            .active_tab()
            .engine
            .clear_browsing_data(cache, cookies, self.proxy.clone());
    }

    pub fn sync_history_add_all(&self, query: &str) {
        for tab in self.collection.tabs() {
            if is_internal_newtab(&tab.url) {
                tab.engine.sync_history_add(query);
            }
        }
    }

    pub fn sync_history_remove_all(&self, query: &str) {
        for tab in self.collection.tabs() {
            if is_internal_newtab(&tab.url) {
                tab.engine.sync_history_remove(query);
            }
        }
    }

    pub fn sync_history_clear_all(&self) {
        for tab in self.collection.tabs() {
            if is_internal_newtab(&tab.url) {
                tab.engine.sync_history_clear();
            }
        }
    }

    pub fn active_tab(&self) -> &TabEntry<ContentEngine> {
        self.collection.active_tab()
    }

    pub fn set_bounds(&mut self, bounds: Rect) {
        self.collection.set_bounds(bounds);
    }

    pub fn create_tab(&mut self, window: &Window, activate: bool) -> Result<TabId, BrowserError> {
        self.create_tab_with_url(window, "browser://localhost/newtab", activate)
    }

    pub fn create_tab_with_url(
        &mut self,
        window: &Window,
        url: &str,
        activate: bool,
    ) -> Result<TabId, BrowserError> {
        let id = self.alloc_id();
        let counter = Arc::new(AtomicU64::new(0));
        let engine = self.attach_tab_engine(window, id, url, activate, Arc::clone(&counter))?;
        self.collection.insert_tab(id, engine, activate);
        if let Some(tab) = self.collection.tab_mut(id) {
            tab.blocked_counter = counter;
            tab.url = canonical_url_for_state(url);
            tab.title = initial_title_for_url(&tab.url);
        }
        Ok(id)
    }

    pub fn open_url_in_new_tab(
        &mut self,
        window: &Window,
        url: &str,
    ) -> Result<TabId, BrowserError> {
        self.create_tab_with_url(window, url, true)
    }

    pub fn open_or_select_settings(
        &mut self,
        window: &Window,
        section: Option<&str>,
    ) -> Result<TabId, BrowserError> {
        if let Some(existing) = self
            .collection
            .tabs()
            .iter()
            .find(|t| is_internal_settings(&t.url))
        {
            let id = existing.id;
            let _ = self.collection.select_tab(id);
            if let Some(sec) = section
                && let Some(tab) = self.collection.tabs().iter().find(|t| t.id == id)
            {
                tab.engine.select_settings_section(sec);
            }
            return Ok(id);
        }
        let target = match section {
            Some(sec) => format!("{INTERNAL_SETTINGS_URL}#{sec}"),
            None => INTERNAL_SETTINGS_URL.to_string(),
        };
        self.create_tab_with_url(window, &target, true)
    }

    pub fn restore_session_tabs(
        &mut self,
        window: &Window,
        urls: &[String],
        active_index: usize,
    ) -> Result<(), BrowserError> {
        for url in urls.iter().skip(1) {
            self.create_tab_with_url(window, url, false)?;
        }
        if let Some(target_id) = self.collection.tabs().get(active_index).map(|t| t.id) {
            let _ = self.collection.select_tab(target_id);
        }
        Ok(())
    }

    pub fn snapshot_session(&self) -> SessionState {
        let active_id = self.collection.active_id();
        let mut urls = Vec::new();
        let mut active_index = 0usize;
        for tab in self.collection.tabs() {
            if !is_internal_page(&tab.url) {
                let lower = tab.url.to_ascii_lowercase();
                if lower.starts_with("https://") || lower.starts_with("http://") {
                    if tab.id == active_id {
                        active_index = urls.len();
                    }
                    urls.push(tab.url.clone());
                }
            }
        }
        SessionState { urls, active_index }
    }

    pub fn close_tab(&mut self, id: TabId) -> bool {
        if !self.collection.tabs().iter().any(|t| t.id == id) {
            return false;
        }

        if self.collection.tabs().len() > 1 {
            let removed = self.collection.remove_non_last_tab(id);
            drop(removed);
            false
        } else {
            true
        }
    }

    fn attach_tab_engine(
        &mut self,
        window: &Window,
        tab_id: TabId,
        initial_url: &str,
        visible: bool,
        blocked_counter: Arc<AtomicU64>,
    ) -> Result<ContentEngine, BrowserError> {
        let bounds = self.collection.bounds();
        ContentEngine::attach(
            window,
            ContentAttachOptions {
                tab_id,
                initial_url,
                #[cfg(target_os = "windows")]
                shared_env: &self.shared_env,
                #[cfg(not(target_os = "windows"))]
                context: &mut self.context,
                bounds,
                visible,
                is_dark: self.theme.is_dark(),
                adblock: self.adblock.clone(),
                blocked_counter,
                proxy: self.proxy.clone(),
            },
        )
    }

    pub fn close_active_tab(&mut self) -> bool {
        let id = self.collection.active_id();
        self.close_tab(id)
    }

    pub fn select_tab(&mut self, id: TabId) -> bool {
        self.collection.select_tab(id)
    }

    pub fn select_next(&mut self) -> bool {
        self.collection.select_next()
    }

    pub fn select_prev(&mut self) -> bool {
        self.collection.select_prev()
    }

    pub fn select_slot(&mut self, slot: usize) -> bool {
        self.collection.select_slot(slot)
    }

    pub fn navigate_active(&mut self, target_url: &str) {
        let id = self.collection.active_id();
        if let Some(tab) = self.collection.tab_mut(id) {
            if origin_changed(&tab.url, target_url) {
                tab.favicon_url = None;
            }
            tab.url = target_url.to_string();
            tab.blocked_counter.store(0, Ordering::Relaxed);
            tab.blocked_count = 0;
            if is_internal_newtab(target_url) {
                tab.title = "Beranda".to_string();
                tab.is_loading = false;
            }
            tab.engine.navigate(target_url);
            if !is_internal_newtab(target_url) {
                tab.engine.focus();
            }
        }
    }

    pub fn handle_navigated(&mut self, tab_id: TabId, url: String) -> bool {
        let Some(tab) = self.collection.tab_mut(tab_id) else {
            return false;
        };
        if origin_changed(&tab.url, &url) {
            tab.favicon_url = None;
        }
        tab.url = url;
        if is_internal_newtab(&tab.url) {
            tab.title = "Beranda".to_string();
            tab.favicon_url = None;
        }
        true
    }

    pub fn handle_title_changed(&mut self, tab_id: TabId, title: String) -> bool {
        let Some(tab) = self.collection.tab_mut(tab_id) else {
            return false;
        };
        tab.title = if is_internal_newtab(&tab.url) {
            "Beranda".to_string()
        } else {
            title
        };
        true
    }

    pub fn handle_load_state(&mut self, tab_id: TabId, loading: bool, url: String) -> bool {
        let Some(tab) = self.collection.tab_mut(tab_id) else {
            return false;
        };
        tab.is_loading = loading;
        if loading {
            tab.blocked_counter.store(0, Ordering::Relaxed);
            tab.blocked_count = 0;
        } else {
            tab.blocked_count = tab.blocked_counter.load(Ordering::Relaxed);
        }
        if !url.is_empty() {
            if origin_changed(&tab.url, &url) {
                tab.favicon_url = None;
            }
            tab.url = url;
        }
        if is_internal_newtab(&tab.url) {
            tab.title = "Beranda".to_string();
            tab.favicon_url = None;
            tab.blocked_counter.store(0, Ordering::Relaxed);
            tab.blocked_count = 0;
        } else if is_internal_welcome(&tab.url) || is_internal_settings(&tab.url) {
            tab.favicon_url = None;
            tab.blocked_counter.store(0, Ordering::Relaxed);
            tab.blocked_count = 0;
        }
        true
    }

    pub fn update_tab_blocked_count(&mut self, tab_id: TabId, count: u64) -> bool {
        let Some(tab) = self.collection.tab_mut(tab_id) else {
            return false;
        };
        tab.blocked_count = count;
        true
    }

    pub fn sync_tab_blocked_count(&mut self, tab_id: TabId) {
        if let Some(tab) = self.collection.tab_mut(tab_id) {
            tab.blocked_count = tab.blocked_counter.load(Ordering::Relaxed);
        }
    }

    pub fn apply_cosmetic_for_tab(&self, tab_id: TabId) {
        if let Some(tab) = self.collection.tabs().iter().find(|t| t.id == tab_id)
            && !is_internal_page(&tab.url)
        {
            let css = self.adblock.cosmetic_css_for_url(&tab.url);
            tab.engine.apply_cosmetic_css(css.as_deref());
        }
    }

    pub fn apply_generic_cosmetic_for_tab(
        &self,
        tab_id: TabId,
        classes: &[String],
        ids: &[String],
    ) {
        if let Some(tab) = self.collection.tabs().iter().find(|t| t.id == tab_id)
            && !is_internal_page(&tab.url)
            && let Some(css) = self
                .adblock
                .generic_cosmetic_css_for_elements(&tab.url, classes, ids)
        {
            tab.engine.apply_generic_cosmetic_css(&css);
        }
    }

    pub fn sync_adblock_state_all(&self) {
        let global_enabled = self.adblock.is_enabled();
        for tab in self.collection.tabs() {
            if !is_internal_page(&tab.url) {
                let active = global_enabled && !self.adblock.is_host_whitelisted(&tab.url);
                tab.engine.set_adblock_state(active);
                if active {
                    let css = self.adblock.cosmetic_css_for_url(&tab.url);
                    tab.engine.apply_cosmetic_css(css.as_deref());
                } else {
                    tab.engine.apply_cosmetic_css(None);
                }
            }
        }
    }

    pub fn sync_adblock_state_tab(&self, tab_id: TabId) {
        if let Some(tab) = self.collection.tabs().iter().find(|t| t.id == tab_id)
            && !is_internal_page(&tab.url)
        {
            let active =
                self.adblock.is_enabled() && !self.adblock.is_host_whitelisted(&tab.url);
            tab.engine.set_adblock_state(active);
            if active {
                let css = self.adblock.cosmetic_css_for_url(&tab.url);
                tab.engine.apply_cosmetic_css(css.as_deref());
            } else {
                tab.engine.apply_cosmetic_css(None);
            }
        }
    }

    pub fn handle_favicon_changed(&mut self, tab_id: TabId, favicon_url: Option<String>) -> bool {
        let Some(tab) = self.collection.tab_mut(tab_id) else {
            return false;
        };
        if !is_internal_newtab(&tab.url) {
            tab.favicon_url = favicon_url;
        }
        true
    }

    pub fn stop_active(&mut self) {
        let id = self.collection.active_id();
        if let Some(tab) = self.collection.tab_mut(id) {
            tab.engine.stop();
            tab.is_loading = false;
        }
    }

    pub fn summaries(&self) -> Vec<ShellTabSummary> {
        let active_id = self.collection.active_id();
        self.collection
            .tabs()
            .iter()
            .map(|t| {
                let origin = format_origin_display(&t.url).origin;
                ShellTabSummary {
                    id: t.id.0,
                    title: t.title.clone(),
                    origin,
                    favicon_url: t.favicon_url.clone().unwrap_or_default(),
                    is_loading: t.is_loading,
                    is_active: t.id == active_id,
                }
            })
            .collect()
    }

    fn alloc_id(&mut self) -> TabId {
        let id = TabId(self.next_id);
        self.next_id += 1;
        id
    }
}

fn is_internal_page(url: &str) -> bool {
    is_internal_newtab(url)
        || is_internal_blog(url)
        || is_internal_welcome(url)
        || is_internal_settings(url)
}

fn canonical_url_for_state(url: &str) -> String {
    if is_internal_welcome(url) {
        INTERNAL_WELCOME_URL.to_string()
    } else if is_internal_settings(url) {
        INTERNAL_SETTINGS_URL.to_string()
    } else if is_internal_newtab(url) {
        INTERNAL_NEWTAB_URL.to_string()
    } else if is_internal_blog(url) {
        INTERNAL_BLOG_URL.to_string()
    } else {
        url.to_string()
    }
}

fn initial_title_for_url(url: &str) -> String {
    if is_internal_welcome(url) {
        "Selamat Datang".to_string()
    } else if is_internal_settings(url) {
        "Pengaturan".to_string()
    } else if is_internal_newtab(url) {
        "Beranda".to_string()
    } else if is_internal_blog(url) {
        "Ruthenium Notes".to_string()
    } else {
        let origin = format_origin_display(url).origin;
        if origin.is_empty() {
            "Tab Baru".to_string()
        } else {
            origin
        }
    }
}

fn origin_changed(prev: &str, next: &str) -> bool {
    format_origin_display(prev).origin != format_origin_display(next).origin
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;
    use wry::dpi::{LogicalPosition, LogicalSize};

    use super::*;

    #[derive(Clone)]
    struct MockEngine {
        visible: Rc<Cell<bool>>,
        dropped: Rc<Cell<bool>>,
        last_width: Rc<Cell<f64>>,
    }

    struct MockHandles {
        visible: Rc<Cell<bool>>,
        dropped: Rc<Cell<bool>>,
        last_width: Rc<Cell<f64>>,
    }

    impl MockEngine {
        fn new() -> (Self, MockHandles) {
            let visible = Rc::new(Cell::new(false));
            let dropped = Rc::new(Cell::new(false));
            let last_width = Rc::new(Cell::new(0.0));
            (
                Self {
                    visible: visible.clone(),
                    dropped: dropped.clone(),
                    last_width: last_width.clone(),
                },
                MockHandles {
                    visible,
                    dropped,
                    last_width,
                },
            )
        }
    }

    impl Drop for MockEngine {
        fn drop(&mut self) {
            self.dropped.set(true);
        }
    }

    impl TabRuntimeOps for MockEngine {
        fn set_bounds(&self, bounds: Rect) {
            let logical = bounds.size.to_logical::<f64>(1.0);
            self.last_width.set(logical.width);
        }

        fn set_visible(&self, visible: bool) {
            self.visible.set(visible);
        }
    }

    fn sample_rect(width: f64, height: f64) -> Rect {
        Rect {
            position: LogicalPosition::new(220.0, 36.0).into(),
            size: LogicalSize::new(width, height).into(),
        }
    }

    #[test]
    fn maintains_single_visible_webview_across_five_tabs_and_switches() {
        let (e1, h1) = MockEngine::new();
        let mut tabs = TabCollection::new(TabId(1), e1, sample_rect(1000.0, 700.0));
        assert!(h1.visible.get());
        assert_eq!(h1.last_width.get(), 1000.0);

        let mut handles = vec![h1];
        for raw_id in 2..=5 {
            let (eng, h) = MockEngine::new();
            tabs.insert_tab(TabId(raw_id), eng, true);
            handles.push(h);
        }

        assert_eq!(tabs.tabs().len(), 5);
        assert_eq!(tabs.active_id(), TabId(5));
        assert_eq!(handles.iter().filter(|h| h.visible.get()).count(), 1);
        assert!(handles[4].visible.get());

        // Resize while Tab 5 is active, then switch back to Tab 2: Tab 2 must receive new bounds.
        tabs.set_bounds(sample_rect(820.0, 600.0));
        assert_eq!(handles[4].last_width.get(), 820.0);
        assert_eq!(handles[1].last_width.get(), 1000.0);

        assert!(tabs.select_tab(TabId(2)));
        assert_eq!(handles[1].last_width.get(), 820.0);
        assert_eq!(handles.iter().filter(|h| h.visible.get()).count(), 1);
        assert!(handles[1].visible.get());
    }

    #[test]
    fn cycles_next_prev_and_numeric_slots() {
        let (e1, _) = MockEngine::new();
        let mut tabs = TabCollection::new(TabId(1), e1, sample_rect(900.0, 600.0));
        for raw_id in 2..=5 {
            let (eng, _) = MockEngine::new();
            tabs.insert_tab(TabId(raw_id), eng, true);
        }

        assert!(tabs.select_slot(1));
        assert_eq!(tabs.active_id(), TabId(1));

        assert!(tabs.select_prev());
        assert_eq!(tabs.active_id(), TabId(5));

        assert!(tabs.select_next());
        assert_eq!(tabs.active_id(), TabId(1));

        assert!(tabs.select_slot(3));
        assert_eq!(tabs.active_id(), TabId(3));

        // Slot 9 jumps to the last tab.
        assert!(tabs.select_slot(9));
        assert_eq!(tabs.active_id(), TabId(5));

        // Out-of-range slot 7 on 5 tabs is ignored cleanly.
        assert!(!tabs.select_slot(7));
        assert_eq!(tabs.active_id(), TabId(5));
    }

    #[test]
    fn closing_tabs_drops_webview_and_selects_neighbor_safely() {
        let (e1, h1) = MockEngine::new();
        let (e2, h2) = MockEngine::new();
        let (e3, h3) = MockEngine::new();

        let mut tabs = TabCollection::new(TabId(1), e1, sample_rect(900.0, 600.0));
        tabs.insert_tab(TabId(2), e2, true);
        tabs.insert_tab(TabId(3), e3, true);

        // Close active last tab (Tab 3) -> activates Tab 2 and drops Tab 3.
        let removed = tabs.remove_non_last_tab(TabId(3));
        drop(removed);
        assert!(h3.dropped.get());
        assert!(!h1.dropped.get() && !h2.dropped.get());
        assert_eq!(tabs.active_id(), TabId(2));
        assert!(h2.visible.get() && !h1.visible.get() && !h3.visible.get());

        // Close inactive Tab 1 -> keeps Tab 2 active and drops Tab 1.
        let removed = tabs.remove_non_last_tab(TabId(1));
        drop(removed);
        assert!(h1.dropped.get() && !h2.dropped.get());
        assert_eq!(tabs.active_id(), TabId(2));

        // Closing the final remaining tab replaces it with a fresh tab and drops Tab 2.
        let (e4, h4) = MockEngine::new();
        let old = tabs.replace_last_tab(TabId(4), e4);
        drop(old);
        assert!(h2.dropped.get() && !h4.dropped.get());
        assert!(h4.visible.get());
        assert_eq!(tabs.active_id(), TabId(4));
        assert_eq!(tabs.tabs().len(), 1);
    }
}
