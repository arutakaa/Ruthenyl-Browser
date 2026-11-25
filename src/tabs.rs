use tao::event_loop::EventLoopProxy;
use tao::window::Window;
use wry::{Rect, WebContext};

use crate::engine::{ContentAttachOptions, ContentEngine, content_data_dir};
use crate::error::BrowserError;
use crate::ipc::{
    AppEvent, BrowserTheme, INTERNAL_NEWTAB_URL, TabId, format_origin_display, is_internal_blog,
    is_internal_newtab,
};
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
    // Drop all child WebView instances before dropping the shared WebContext.
    collection: TabCollection<ContentEngine>,
    next_id: u64,
    theme: BrowserTheme,
    context: WebContext,
    proxy: EventLoopProxy<AppEvent>,
}

impl TabManager {
    pub fn new(
        window: &Window,
        initial_bounds: Rect,
        proxy: EventLoopProxy<AppEvent>,
    ) -> Result<Self, BrowserError> {
        let mut context = WebContext::new(Some(content_data_dir()));
        let first_id = TabId(1);
        let theme = BrowserTheme::default();
        let first_engine = ContentEngine::attach(
            window,
            ContentAttachOptions {
                tab_id: first_id,
                context: &mut context,
                bounds: initial_bounds,
                visible: true,
                is_dark: theme.is_dark(),
                proxy: proxy.clone(),
            },
        )?;

        Ok(Self {
            collection: TabCollection::new(first_id, first_engine, initial_bounds),
            next_id: 2,
            theme,
            context,
            proxy,
        })
    }

    pub fn set_theme(&mut self, theme: BrowserTheme) {
        self.theme = theme;
        let wire = theme.as_wire();
        for tab in self.collection.tabs() {
            if is_internal_newtab(&tab.url) || is_internal_blog(&tab.url) {
                tab.engine.sync_theme(wire);
            }
        }
    }

    pub fn sync_internal_prefs_all(&self, engine: &str, theme: BrowserTheme) {
        let wire = theme.as_wire();
        for tab in self.collection.tabs() {
            if is_internal_newtab(&tab.url) || is_internal_blog(&tab.url) {
                tab.engine.sync_internal_prefs(engine, wire);
            }
        }
    }

    pub fn sync_internal_prefs_tab(&self, tab_id: TabId, engine: &str, theme: BrowserTheme) {
        let wire = theme.as_wire();
        if let Some(tab) = self.collection.tabs().iter().find(|t| t.id == tab_id)
            && (is_internal_newtab(&tab.url) || is_internal_blog(&tab.url))
        {
            tab.engine.sync_internal_prefs(engine, wire);
        }
    }

    pub fn active_tab(&self) -> &TabEntry<ContentEngine> {
        self.collection.active_tab()
    }

    pub fn set_bounds(&mut self, bounds: Rect) {
        self.collection.set_bounds(bounds);
    }

    pub fn create_tab(&mut self, window: &Window, activate: bool) -> Result<TabId, BrowserError> {
        let id = self.alloc_id();
        let engine = self.attach_tab_engine(window, id, activate)?;
        self.collection.insert_tab(id, engine, activate);
        Ok(id)
    }

    pub fn close_tab(&mut self, window: &Window, id: TabId) -> Result<bool, BrowserError> {
        if !self.collection.tabs().iter().any(|t| t.id == id) {
            return Ok(false);
        }

        if self.collection.tabs().len() > 1 {
            let removed = self.collection.remove_non_last_tab(id);
            drop(removed);
            Ok(false)
        } else {
            let replacement_id = self.alloc_id();
            let replacement = self.attach_tab_engine(window, replacement_id, true)?;
            let old = self
                .collection
                .replace_last_tab(replacement_id, replacement);
            drop(old);
            Ok(true)
        }
    }

    fn attach_tab_engine(
        &mut self,
        window: &Window,
        tab_id: TabId,
        visible: bool,
    ) -> Result<ContentEngine, BrowserError> {
        let bounds = self.collection.bounds();
        ContentEngine::attach(
            window,
            ContentAttachOptions {
                tab_id,
                context: &mut self.context,
                bounds,
                visible,
                is_dark: self.theme.is_dark(),
                proxy: self.proxy.clone(),
            },
        )
    }

    pub fn close_active_tab(&mut self, window: &Window) -> Result<bool, BrowserError> {
        let id = self.collection.active_id();
        self.close_tab(window, id)
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
        if !url.is_empty() {
            if origin_changed(&tab.url, &url) {
                tab.favicon_url = None;
            }
            tab.url = url;
        }
        if is_internal_newtab(&tab.url) {
            tab.title = "Beranda".to_string();
            tab.favicon_url = None;
        }
        true
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
