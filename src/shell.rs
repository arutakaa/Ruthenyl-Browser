use std::borrow::Cow;
use std::fmt::Write as _;

use tao::event_loop::{EventLoopProxy, EventLoopWindowTarget};
use tao::window::{Window, WindowId};
#[cfg(target_os = "windows")]
use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Environment;
use wry::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use wry::http::{Request, Response};
use wry::{Rect, WebContext, WebView, WebViewBuilder};
#[cfg(target_os = "windows")]
use wry::{ScrollBarStyle, Theme, WebViewBuilderExtWindows, WebViewExtWindows};

#[cfg(target_os = "windows")]
use crate::engine::SHARED_WEBVIEW2_BROWSER_ARGS;
use crate::engine::content_data_dir;
use crate::error::BrowserError;
use crate::ipc::{AppEvent, OmniboxPopupBounds, OriginDisplay, escape_js_string, parse_shell_ipc};
use crate::omnibox::OmniboxPopup;

const SHELL_HTML: &[u8] = include_bytes!("../assets/shell.html");
const SHELL_CSS: &[u8] = include_bytes!("../assets/shell.css");
const SHELL_JS: &[u8] = include_bytes!("../assets/shell.js");
const RUTHENIUM_LOGO_SVG: &[u8] = include_bytes!("../assets/ruthenium-logo.svg");

const RUTHENIUM_CHROME_BG: (u8, u8, u8, u8) = (232, 238, 246, 255);

pub struct ShellTabSummary {
    pub id: u64,
    pub title: String,
    pub origin: String,
    pub favicon_url: String,
    pub is_loading: bool,
    pub is_active: bool,
}

pub struct ShellSyncPayload<'a> {
    pub tabs: &'a [ShellTabSummary],
    pub origin_display: &'a OriginDisplay,
    pub title: &'a str,
    pub can_go_back: bool,
    pub can_go_forward: bool,
    pub is_loading: bool,
    pub sidebar_width: f64,
    pub is_compact: bool,
    pub is_maximized: bool,
    pub window_focused: bool,
    pub theme: &'a str,
    pub profile_name: &'a str,
    pub search_engine: &'a str,
    pub adblock_enabled: bool,
    pub adblock_blocked_tab: u64,
    pub adblock_blocked_total: u64,
    pub adblock_status: &'a str,
    pub adblock_refreshing: bool,
}

pub struct ShellView {
    // Drop the child/popup WebViews before the backing WebContext.
    popup: OmniboxPopup,
    webview: WebView,
    _context: WebContext,
}

impl ShellView {
    pub fn attach(
        event_loop: &EventLoopWindowTarget<AppEvent>,
        window: &Window,
        bounds: Rect,
        proxy: EventLoopProxy<AppEvent>,
    ) -> Result<Self, BrowserError> {
        let mut context = WebContext::new(Some(content_data_dir()));
        let shell_proxy = proxy.clone();
        let builder = WebViewBuilder::new_with_web_context(&mut context)
            .with_bounds(bounds)
            .with_background_color(RUTHENIUM_CHROME_BG)
            .with_asynchronous_custom_protocol("browser".into(), |_id, req, responder| {
                if req.uri().path() == "/suggest" {
                    crate::suggest::handle_suggest_request(&req, responder);
                } else {
                    responder.respond(serve_shell_asset(&req));
                }
            })
            .with_url("browser://localhost/shell")
            .with_ipc_handler(move |req: Request<String>| {
                if let Some(event) = parse_shell_ipc(req.body()) {
                    let _ = shell_proxy.send_event(event);
                }
            });

        #[cfg(target_os = "windows")]
        let builder = builder
            .with_theme(Theme::Light)
            .with_scroll_bar_style(ScrollBarStyle::FluentOverlay)
            .with_browser_accelerator_keys(false)
            .with_profile_name("ruthenium-shell")
            .with_additional_browser_args(SHARED_WEBVIEW2_BROWSER_ARGS);

        let webview = builder
            .build_as_child(window)
            .map_err(BrowserError::ShellInit)?;

        #[cfg(target_os = "windows")]
        let popup = OmniboxPopup::create(event_loop, window, webview.environment(), proxy)?;
        #[cfg(not(target_os = "windows"))]
        let popup = OmniboxPopup::create(event_loop, window, &mut context, proxy)?;

        Ok(Self {
            popup,
            webview,
            _context: context,
        })
    }

    #[cfg(target_os = "windows")]
    pub fn shared_environment(&self) -> ICoreWebView2Environment {
        self.webview.environment()
    }

    pub fn set_bounds(&self, bounds: Rect) {
        let _ = self.webview.set_bounds(bounds);
    }

    pub fn focus_address_bar(&self, open_dropdown: bool) {
        let _ = self.webview.focus();
        let script = format!(
            "if (window.__focusAddressBar) window.__focusAddressBar({open_dropdown});"
        );
        let _ = self.webview.evaluate_script(&script);
    }

    pub fn popup_window_id(&self) -> WindowId {
        self.popup.window_id()
    }

    pub fn show_omnibox_popup(
        &mut self,
        owner: &Window,
        bounds: &OmniboxPopupBounds,
        payload_json: &str,
    ) {
        self.popup.show_and_render(owner, bounds, payload_json);
    }

    pub fn select_omnibox_popup(&self, index: i32) {
        self.popup.select_index(index);
    }

    pub fn hide_omnibox_popup(&mut self) {
        self.popup.hide();
    }

    pub fn is_pointer_or_focus_in_popup(&self) -> bool {
        self.popup.is_pointer_or_focus_in_popup()
    }

    pub fn is_popup_foreground(&self) -> bool {
        self.popup.is_foreground()
    }

    pub fn set_popup_theme(&self, theme: &str) {
        self.popup.set_theme(theme);
    }

    pub fn close_omnibox_ui(&self, blur_input: bool) {
        let script = format!(
            "if (window.__omniboxCloseUi) window.__omniboxCloseUi({blur_input});"
        );
        let _ = self.webview.evaluate_script(&script);
    }

    pub fn fill_address_bar(&self, text: &str) {
        let _ = self.webview.focus();
        let script = format!(
            "if (window.__omniboxFill) window.__omniboxFill({});",
            escape_js_string(text),
        );
        let _ = self.webview.evaluate_script(&script);
    }

    pub fn refresh_omnibox_after_history_change(&self, focus_input: bool) {
        if focus_input {
            let _ = self.webview.focus();
        }
        let script = format!(
            "if (window.__omniboxRefresh) window.__omniboxRefresh({focus_input});"
        );
        let _ = self.webview.evaluate_script(&script);
    }

    pub fn sync_history_add(&self, query: &str) {
        let script = format!(
            "if (window.__syncHistoryAdd) window.__syncHistoryAdd({});",
            escape_js_string(query),
        );
        let _ = self.webview.evaluate_script(&script);
    }

    pub fn sync_history_remove(&self, query: &str) {
        let script = format!(
            "if (window.__syncHistoryRemove) window.__syncHistoryRemove({});",
            escape_js_string(query),
        );
        let _ = self.webview.evaluate_script(&script);
    }

    pub fn sync_history_clear(&self) {
        let _ = self
            .webview
            .evaluate_script("if (window.__syncHistoryClear) window.__syncHistoryClear();");
    }

    pub fn sync_state(&self, state: &ShellSyncPayload<'_>) {
        let mut tabs_json = String::with_capacity(state.tabs.len() * 96 + 2);
        tabs_json.push('[');
        for (idx, tab) in state.tabs.iter().enumerate() {
            if idx > 0 {
                tabs_json.push(',');
            }
            let _ = write!(
                tabs_json,
                "{{id:{},title:{},origin:{},faviconUrl:{},isLoading:{},isActive:{}}}",
                tab.id,
                escape_js_string(&tab.title),
                escape_js_string(&tab.origin),
                escape_js_string(&tab.favicon_url),
                tab.is_loading,
                tab.is_active,
            );
        }
        tabs_json.push(']');

        let script = format!(
            "if (window.__shellSync) window.__shellSync({{tabs:{},rawUrl:{},origin:{},path:{},title:{},security:{},canGoBack:{},canGoForward:{},isLoading:{},sidebarWidth:{:.1},isCompact:{},isMaximized:{},windowFocused:{},theme:{},profileName:{},searchEngine:{},adblockEnabled:{},adblockBlockedTab:{},adblockBlockedTotal:{},adblockStatus:{},adblockRefreshing:{}}});",
            tabs_json,
            escape_js_string(&state.origin_display.raw_url),
            escape_js_string(&state.origin_display.origin),
            escape_js_string(&state.origin_display.path),
            escape_js_string(state.title),
            escape_js_string(state.origin_display.security),
            state.can_go_back,
            state.can_go_forward,
            state.is_loading,
            state.sidebar_width,
            state.is_compact,
            state.is_maximized,
            state.window_focused,
            escape_js_string(state.theme),
            escape_js_string(state.profile_name),
            escape_js_string(state.search_engine),
            state.adblock_enabled,
            state.adblock_blocked_tab,
            state.adblock_blocked_total,
            escape_js_string(state.adblock_status),
            state.adblock_refreshing,
        );
        let _ = self.webview.evaluate_script(&script);
    }
}

fn serve_shell_asset(req: &Request<Vec<u8>>) -> Response<Cow<'static, [u8]>> {
    let (mime, body): (&str, &'static [u8]) = match req.uri().path() {
        "/shell.css" => ("text/css; charset=utf-8", SHELL_CSS),
        "/shell.js" => ("text/javascript; charset=utf-8", SHELL_JS),
        "/ruthenium-logo.svg" => ("image/svg+xml", RUTHENIUM_LOGO_SVG),
        _ => ("text/html; charset=utf-8", SHELL_HTML),
    };

    Response::builder()
        .header(CONTENT_TYPE, mime)
        .header(CACHE_CONTROL, "no-store, no-cache, must-revalidate")
        .status(200)
        .body(Cow::Borrowed(body))
        .expect("static shell asset response must be valid")
}
