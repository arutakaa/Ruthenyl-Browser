use std::borrow::Cow;
use std::env;
use std::fmt::Write as _;
use std::path::PathBuf;

use tao::event_loop::EventLoopProxy;
use tao::window::Window;
use wry::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use wry::http::{Request, Response};
use wry::{Rect, WebContext, WebView, WebViewBuilder};
#[cfg(target_os = "windows")]
use wry::{ScrollBarStyle, Theme, WebViewBuilderExtWindows};

use crate::error::BrowserError;
use crate::ipc::{AppEvent, OriginDisplay, escape_js_string, parse_shell_ipc};

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
}

pub struct ShellView {
    // Drop the WebView2 controller before its backing WebContext.
    webview: WebView,
    _context: WebContext,
}

impl ShellView {
    pub fn attach(
        window: &Window,
        bounds: Rect,
        proxy: EventLoopProxy<AppEvent>,
    ) -> Result<Self, BrowserError> {
        let mut context = WebContext::new(Some(shell_data_dir()));
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
                    let _ = proxy.send_event(event);
                }
            });

        #[cfg(target_os = "windows")]
        let builder = builder
            .with_theme(Theme::Light)
            .with_scroll_bar_style(ScrollBarStyle::FluentOverlay)
            .with_browser_accelerator_keys(false);

        let webview = builder
            .build_as_child(window)
            .map_err(BrowserError::ShellInit)?;

        Ok(Self {
            webview,
            _context: context,
        })
    }

    pub fn set_bounds(&self, bounds: Rect) {
        let _ = self.webview.set_bounds(bounds);
    }

    pub fn focus_address_bar(&self) {
        let _ = self.webview.focus();
        let _ = self
            .webview
            .evaluate_script("if (window.__focusAddressBar) window.__focusAddressBar();");
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
            "if (window.__shellSync) window.__shellSync({{tabs:{},rawUrl:{},origin:{},path:{},title:{},security:{},canGoBack:{},canGoForward:{},isLoading:{},sidebarWidth:{:.1},isCompact:{},isMaximized:{},windowFocused:{},theme:{},profileName:{},searchEngine:{}}});",
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

fn shell_data_dir() -> PathBuf {
    // Isolate privileged browser chrome storage from website content profiles.
    env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(env::temp_dir)
        .join("Browser")
        .join("webview2-shell")
}
