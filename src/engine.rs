use std::borrow::Cow;
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use tao::event_loop::EventLoopProxy;
use tao::window::Window;
#[cfg(target_os = "windows")]
use webview2_com::Microsoft::Web::WebView2::Win32::{
    COREWEBVIEW2_BROWSING_DATA_KINDS, COREWEBVIEW2_BROWSING_DATA_KINDS_CACHE_STORAGE,
    COREWEBVIEW2_BROWSING_DATA_KINDS_COOKIES, COREWEBVIEW2_BROWSING_DATA_KINDS_DISK_CACHE,
    COREWEBVIEW2_WEB_RESOURCE_CONTEXT, COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL,
    COREWEBVIEW2_WEB_RESOURCE_CONTEXT_CSP_VIOLATION_REPORT,
    COREWEBVIEW2_WEB_RESOURCE_CONTEXT_DOCUMENT, COREWEBVIEW2_WEB_RESOURCE_CONTEXT_EVENT_SOURCE,
    COREWEBVIEW2_WEB_RESOURCE_CONTEXT_FETCH, COREWEBVIEW2_WEB_RESOURCE_CONTEXT_FONT,
    COREWEBVIEW2_WEB_RESOURCE_CONTEXT_IMAGE, COREWEBVIEW2_WEB_RESOURCE_CONTEXT_MEDIA,
    COREWEBVIEW2_WEB_RESOURCE_CONTEXT_PING, COREWEBVIEW2_WEB_RESOURCE_CONTEXT_SCRIPT,
    COREWEBVIEW2_WEB_RESOURCE_CONTEXT_STYLESHEET, COREWEBVIEW2_WEB_RESOURCE_CONTEXT_WEBSOCKET,
    COREWEBVIEW2_WEB_RESOURCE_CONTEXT_XML_HTTP_REQUEST,
    COREWEBVIEW2_WEB_RESOURCE_REQUEST_SOURCE_KINDS_ALL,
    GetAvailableCoreWebView2BrowserVersionString, ICoreWebView2_13, ICoreWebView2_22,
    ICoreWebView2Environment, ICoreWebView2HttpRequestHeaders, ICoreWebView2Profile2,
};
#[cfg(target_os = "windows")]
use webview2_com::{ClearBrowsingDataCompletedHandler, WebResourceRequestedEventHandler, take_pwstr};
#[cfg(target_os = "windows")]
use windows_core::{HSTRING, Interface, PCWSTR, PWSTR};
use wry::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use wry::http::{Request, Response};
#[cfg(not(target_os = "windows"))]
use wry::WebContext;
use wry::{NewWindowResponse, PageLoadEvent, Rect, WebView, WebViewBuilder};
#[cfg(target_os = "windows")]
use wry::{ScrollBarStyle, Theme, WebViewBuilderExtWindows, WebViewExtWindows};

use crate::adblock::AdblockService;
use crate::error::BrowserError;
use crate::ipc::{AppEvent, TabId, escape_js_string, parse_authenticated_content_ipc};

const EMPTY_VIEWPORT_HTML: &[u8] = include_bytes!("../assets/empty_viewport.html");
const RUTHENIUM_BLOG_HTML: &[u8] = include_bytes!("../assets/ruthenium_blog.html");
const WELCOME_HTML: &[u8] = include_bytes!("../assets/welcome.html");
const WELCOME_CSS: &[u8] = include_bytes!("../assets/welcome.css");
const WELCOME_JS: &[u8] = include_bytes!("../assets/welcome.js");
const SETTINGS_HTML: &[u8] = include_bytes!("../assets/settings.html");
const SETTINGS_CSS: &[u8] = include_bytes!("../assets/settings.css");
const SETTINGS_JS: &[u8] = include_bytes!("../assets/settings.js");
const NEWTAB_CSS: &[u8] = include_bytes!("../assets/newtab.css");
const NEWTAB_JS: &[u8] = include_bytes!("../assets/newtab.js");
const RUTHENIUM_LOGO_SVG: &[u8] = include_bytes!("../assets/ruthenium-logo.svg");
const RUTHENIUM_LOGO_3D_SVG: &[u8] = include_bytes!("../assets/ruthenium-logo-3d.svg");
const DARK_VIEWPORT_BG: (u8, u8, u8, u8) = (8, 13, 26, 255);
const LIGHT_VIEWPORT_BG: (u8, u8, u8, u8) = (248, 250, 252, 255);
#[cfg(target_os = "windows")]
pub const SHARED_WEBVIEW2_BROWSER_ARGS: &str = "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --autoplay-policy=no-user-gesture-required --process-per-site --renderer-process-limit=4";
const BASE_AGGRESSIVE_COSMETIC_CSS: &str = "ins.adsbygoogle,[id^=\"google_ads_iframe\"],[id^=\"div-gpt-ad\"],[id^=\"aswift_\"],iframe[src*=\"doubleclick.net\"],iframe[src*=\"googlesyndication.com\"],iframe[src*=\"adnxs.com\"],iframe[src*=\"taboola.com\"],iframe[src*=\"outbrain.com\"],.OUTBRAIN,[data-widget-id*=\"outbrain\"],#taboola-below-article-thumbnails,[id^=\"taboola-\"],.trc_related_container,.mgbox,[id*=\"ScriptRoot\"],.ad-slot,.ad-container,.ads-container,.advertisement,.sponsored-ad,ytd-ad-slot-renderer,ytd-in-feed-ad-layout-renderer,ytd-banner-promo-renderer,ytd-promoted-sparkles-web-renderer,ytd-promoted-sparkles-text-search-renderer,ytd-display-ad-renderer,ytd-companion-slot-renderer,ytd-action-companion-ad-renderer,ytd-promoted-video-renderer,ytd-player-legacy-desktop-watch-ads-renderer,ytd-engagement-panel-section-list-renderer[target-id=\"engagement-panel-ads\"],ytd-statement-banner-renderer,#masthead-ad,#player-ads,.ytp-ad-module,.ytp-ad-overlay-container,.ytp-ad-image-overlay,.ytp-ad-text-overlay,ytd-enforcement-message-view-model,tp-yt-paper-dialog:has(ytd-enforcement-message-view-model){display:none!important;}";

pub struct ContentAttachOptions<'a> {
    pub tab_id: TabId,
    pub initial_url: &'a str,
    #[cfg(target_os = "windows")]
    pub shared_env: &'a ICoreWebView2Environment,
    #[cfg(not(target_os = "windows"))]
    pub context: &'a mut WebContext,
    pub bounds: Rect,
    pub visible: bool,
    pub is_dark: bool,
    pub adblock: AdblockService,
    pub blocked_counter: Arc<AtomicU64>,
    pub proxy: EventLoopProxy<AppEvent>,
}

pub struct ContentEngine {
    webview: WebView,
}

impl ContentEngine {
    pub fn attach(
        window: &Window,
        opts: ContentAttachOptions<'_>,
    ) -> Result<Self, BrowserError> {
        let ContentAttachOptions {
            tab_id,
            initial_url,
            #[cfg(target_os = "windows")]
            shared_env,
            #[cfg(not(target_os = "windows"))]
            context,
            bounds,
            visible,
            is_dark,
            adblock,
            blocked_counter,
            proxy,
        } = opts;
        let cap_token = generate_capability_token(tab_id);
        let bg = if is_dark {
            DARK_VIEWPORT_BG
        } else {
            LIGHT_VIEWPORT_BG
        };

        let nav_proxy = proxy.clone();
        let nw_adblock = adblock.clone();
        let nw_counter = Arc::clone(&blocked_counter);
        let nw_proxy = proxy.clone();
        let title_proxy = proxy.clone();
        let load_proxy = proxy.clone();
        let adblock_proxy = proxy.clone();
        let ipc_proxy = proxy;
        let init_script = build_content_bridge_script(&cap_token, adblock.is_enabled());

        #[cfg(target_os = "windows")]
        let builder = WebViewBuilder::new().with_environment(shared_env.clone());
        #[cfg(not(target_os = "windows"))]
        let builder = WebViewBuilder::new_with_web_context(context);

        let builder = builder
            .with_bounds(bounds)
            .with_visible(visible)
            .with_focused(false)
            .with_background_color(bg)
            .with_initialization_script(&init_script)
            .with_asynchronous_custom_protocol("browser".into(), |_id, req, responder| {
                if req.uri().path() == "/suggest" {
                    crate::suggest::handle_suggest_request(&req, responder);
                } else {
                    responder.respond(serve_newtab_asset(&req));
                }
            })
            .with_url(initial_url)
            .with_navigation_handler(move |url| {
                let _ = nav_proxy.send_event(AppEvent::TabNavigated { tab_id, url });
                true
            })
            .with_new_window_req_handler(move |uri, features| {
                #[cfg(target_os = "windows")]
                let source_url = {
                    let mut src_pwstr = PWSTR::null();
                    if unsafe { features.opener.webview.Source(&mut src_pwstr) }.is_ok() {
                        take_pwstr(src_pwstr)
                    } else {
                        String::new()
                    }
                };
                #[cfg(not(target_os = "windows"))]
                let source_url = {
                    let _ = &features;
                    String::new()
                };

                if nw_adblock.should_block_popup(&uri, &source_url) {
                    let count = nw_counter.fetch_add(1, Ordering::Relaxed) + 1;
                    let _ = nw_adblock.record_block();
                    let _ = nw_proxy.send_event(AppEvent::TabAdBlocked { tab_id, count });
                    return NewWindowResponse::Deny;
                }

                if features.size.is_none() {
                    if uri.starts_with("https://") || uri.starts_with("http://") {
                        let _ = nw_proxy.send_event(AppEvent::OpenUrlInNewTab(uri));
                        return NewWindowResponse::Deny;
                    }
                    if nw_adblock.is_enabled() {
                        return NewWindowResponse::Deny;
                    }
                }

                NewWindowResponse::Allow
            })
            .with_document_title_changed_handler(move |title| {
                let _ = title_proxy.send_event(AppEvent::TabTitleChanged { tab_id, title });
            })
            .with_on_page_load_handler(move |event, url| {
                let loading = matches!(event, PageLoadEvent::Started);
                let _ = load_proxy.send_event(AppEvent::TabLoadState {
                    tab_id,
                    loading,
                    url,
                });
            })
            .with_ipc_handler(move |req: Request<String>| {
                if let Some(event) = parse_authenticated_content_ipc(tab_id, &cap_token, req.body())
                {
                    let _ = ipc_proxy.send_event(event);
                }
            });

        #[cfg(target_os = "windows")]
        let builder = builder
            .with_theme(if is_dark { Theme::Dark } else { Theme::Light })
            .with_scroll_bar_style(ScrollBarStyle::FluentOverlay)
            .with_browser_accelerator_keys(false);

        let webview = builder
            .build_as_child(window)
            .map_err(BrowserError::EngineInit)?;

        #[cfg(target_os = "windows")]
        let _ = attach_adblock_interceptor(&webview, tab_id, adblock, blocked_counter, adblock_proxy);

        #[cfg(not(target_os = "windows"))]
        let _ = (adblock, blocked_counter, adblock_proxy);

        Ok(Self { webview })
    }

    pub fn set_bounds(&self, bounds: Rect) {
        let _ = self.webview.set_bounds(bounds);
    }

    pub fn set_visible(&self, visible: bool) {
        let _ = self.webview.set_visible(visible);
    }

    pub fn navigate(&self, url: &str) {
        let _ = self.webview.load_url(url);
    }

    pub fn go_back(&self) {
        if self.can_go_back() {
            let _ = self.webview.go_back();
        }
    }

    pub fn go_forward(&self) {
        if self.can_go_forward() {
            let _ = self.webview.go_forward();
        }
    }

    pub fn reload(&self) {
        let _ = self.webview.reload();
    }

    pub fn stop(&self) {
        let _ = self.webview.evaluate_script("window.stop();");
    }

    pub fn focus(&self) {
        let _ = self.webview.focus();
    }

    pub fn sync_internal_prefs(&self, engine: &str, theme: &str) {
        let script = format!(
            "if (window.__syncSearchEngine) window.__syncSearchEngine({}); if (window.__syncTheme) window.__syncTheme({});",
            escape_js_string(engine),
            escape_js_string(theme),
        );
        let _ = self.webview.evaluate_script(&script);
    }

    pub fn sync_theme(&self, theme: &str) {
        let script = format!(
            "if (window.__syncTheme) window.__syncTheme({});",
            escape_js_string(theme),
        );
        let _ = self.webview.evaluate_script(&script);
    }

    pub fn sync_default_browser(&self, is_default: bool) {
        let script = format!(
            "if (window.__syncDefaultBrowser) window.__syncDefaultBrowser({});",
            if is_default { "true" } else { "false" },
        );
        let _ = self.webview.evaluate_script(&script);
    }

    pub fn sync_home_prefs(&self, show_clock: bool, show_shortcuts: bool, suggest_enabled: bool) {
        let script = format!(
            "if (window.__syncHomePrefs) window.__syncHomePrefs({{showClock:{show_clock},showShortcuts:{show_shortcuts},suggestEnabled:{suggest_enabled}}});",
        );
        let _ = self.webview.evaluate_script(&script);
    }

    pub fn sync_settings_json(&self, json: &str) {
        let script = format!(
            "if (window.__syncSettings) window.__syncSettings(JSON.parse({}));",
            escape_js_string(json),
        );
        let _ = self.webview.evaluate_script(&script);
    }

    pub fn sync_settings_feedback(&self, msg: &str) {
        let script = format!(
            "if (window.__settingsFeedback) window.__settingsFeedback({});",
            escape_js_string(msg),
        );
        let _ = self.webview.evaluate_script(&script);
    }

    pub fn select_settings_section(&self, section: &str) {
        let script = format!(
            "if (window.__selectSettingsSection) window.__selectSettingsSection({});",
            escape_js_string(section),
        );
        let _ = self.webview.evaluate_script(&script);
    }

    pub fn set_zoom(&self, zoom_percent: u16) {
        let _ = self.webview.zoom(f64::from(zoom_percent) / 100.0);
    }

    pub fn set_download_folder(&self, path: &str) {
        let _ = std::fs::create_dir_all(path);
        #[cfg(target_os = "windows")]
        {
            let core_webview = self.webview.webview();
            if let Ok(wv13) = core_webview.cast::<ICoreWebView2_13>()
                && let Ok(profile) = unsafe { wv13.Profile() }
            {
                let _ = unsafe { profile.SetDefaultDownloadFolderPath(&HSTRING::from(path)) };
            }
        }
    }

    pub fn clear_browsing_data(&self, cache: bool, cookies: bool, proxy: EventLoopProxy<AppEvent>) {
        #[cfg(target_os = "windows")]
        {
            let mut kind_bits = 0i32;
            if cache {
                kind_bits |= COREWEBVIEW2_BROWSING_DATA_KINDS_DISK_CACHE.0
                    | COREWEBVIEW2_BROWSING_DATA_KINDS_CACHE_STORAGE.0;
            }
            if cookies {
                kind_bits |= COREWEBVIEW2_BROWSING_DATA_KINDS_COOKIES.0;
            }
            if kind_bits == 0 {
                let _ = proxy.send_event(AppEvent::BrowsingDataCleared(true));
                return;
            }

            let cb_proxy = proxy.clone();
            let core_webview = self.webview.webview();
            let res: windows_core::Result<()> = (|| {
                let wv13: ICoreWebView2_13 = core_webview.cast()?;
                let profile = unsafe { wv13.Profile()? };
                let profile2: ICoreWebView2Profile2 = profile.cast()?;
                let handler = ClearBrowsingDataCompletedHandler::create(Box::new(move |err| {
                    let _ = cb_proxy.send_event(AppEvent::BrowsingDataCleared(err.is_ok()));
                    Ok(())
                }));
                unsafe {
                    profile2.ClearBrowsingData(
                        COREWEBVIEW2_BROWSING_DATA_KINDS(kind_bits),
                        &handler,
                    )?;
                }
                Ok(())
            })();

            if res.is_err() {
                let _ = proxy.send_event(AppEvent::BrowsingDataCleared(false));
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (cache, cookies);
            let _ = proxy.send_event(AppEvent::BrowsingDataCleared(true));
        }
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

    pub fn apply_cosmetic_css(&self, css: Option<&str>) {
        let script = match css {
            Some(rules) if !rules.is_empty() => format!(
                r#"(() => {{
  const id = "__ruthenium_adblock_cosmetic";
  let el = document.getElementById(id);
  if (!el) {{
    el = document.createElement("style");
    el.id = id;
    (document.head || document.documentElement).appendChild(el);
  }}
  el.textContent = {};
}})();"#,
                escape_js_string(rules),
            ),
            _ => r#"(() => {
  const el = document.getElementById("__ruthenium_adblock_cosmetic");
  if (el) el.remove();
})();"#
                .to_string(),
        };
        let _ = self.webview.evaluate_script(&script);
    }

    pub fn apply_generic_cosmetic_css(&self, css: &str) {
        if css.is_empty() {
            return;
        }
        let script = format!(
            r#"(() => {{
  const id = "__ruthenium_adblock_generic";
  let el = document.getElementById(id);
  if (!el) {{
    el = document.createElement("style");
    el.id = id;
    (document.head || document.documentElement).appendChild(el);
  }}
  const chunk = {};
  if (el.textContent.length + chunk.length <= 65536) {{
    el.textContent += chunk;
  }}
}})();"#,
            escape_js_string(css),
        );
        let _ = self.webview.evaluate_script(&script);
    }

    pub fn set_adblock_state(&self, enabled: bool) {
        let script = format!(
            "if (window.__setAdblockState) window.__setAdblockState({});",
            if enabled { "true" } else { "false" }
        );
        let _ = self.webview.evaluate_script(&script);
    }

    pub fn can_go_back(&self) -> bool {
        self.webview.can_go_back().unwrap_or(false)
    }

    pub fn can_go_forward(&self) -> bool {
        self.webview.can_go_forward().unwrap_or(false)
    }
}

#[cfg(target_os = "windows")]
fn attach_adblock_interceptor(
    webview: &WebView,
    tab_id: TabId,
    adblock: AdblockService,
    blocked_counter: Arc<AtomicU64>,
    proxy: EventLoopProxy<AppEvent>,
) -> windows_core::Result<()> {
    let core_webview = webview.webview();
    let env = webview.environment();

    for pattern in ["https://*", "http://*"] {
        let filter = HSTRING::from(pattern);
        unsafe {
            match core_webview.cast::<ICoreWebView2_22>() {
                Ok(webview_22) => {
                    webview_22.AddWebResourceRequestedFilterWithRequestSourceKinds(
                        &filter,
                        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL,
                        COREWEBVIEW2_WEB_RESOURCE_REQUEST_SOURCE_KINDS_ALL,
                    )?;
                }
                _ => {
                    core_webview.AddWebResourceRequestedFilter(
                        &filter,
                        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL,
                    )?;
                }
            }
        }
    }

    let start = Instant::now();
    let last_notify_ms = AtomicU64::new(0);
    let mut token: i64 = 0;

    unsafe {
        core_webview.add_WebResourceRequested(
            &WebResourceRequestedEventHandler::create(Box::new(move |sender, args| {
                if !adblock.is_enabled() {
                    return Ok(());
                }
                let Some(args) = args else {
                    return Ok(());
                };

                let req = args.Request()?;
                let mut uri_pwstr = PWSTR::null();
                req.Uri(&mut uri_pwstr)?;
                let url = take_pwstr(uri_pwstr);
                let lower_url = url.to_ascii_lowercase();
                if lower_url.starts_with("http://browser.localhost/")
                    || lower_url.starts_with("https://browser.localhost/")
                {
                    return Ok(());
                }

                let headers = req.Headers().ok();
                let mut ctx = COREWEBVIEW2_WEB_RESOURCE_CONTEXT::default();
                args.ResourceContext(&mut ctx)?;
                let request_type = if ctx == COREWEBVIEW2_WEB_RESOURCE_CONTEXT_DOCUMENT {
                    let is_iframe = headers
                        .as_ref()
                        .and_then(|h| read_header(h, "Sec-Fetch-Dest"))
                        .is_some_and(|dest| {
                            dest.eq_ignore_ascii_case("iframe")
                                || dest.eq_ignore_ascii_case("frame")
                        });
                    if !is_iframe {
                        return Ok(());
                    }
                    "subdocument"
                } else {
                    map_resource_context(ctx)
                };

                let mut src_pwstr = PWSTR::null();
                let page_url = if let Some(wv) = sender.as_ref()
                    && wv.Source(&mut src_pwstr).is_ok()
                {
                    take_pwstr(src_pwstr)
                } else {
                    String::new()
                };
                let source_url = if !page_url.is_empty() && page_url != "about:blank" {
                    page_url
                } else if let Some(ref h) = headers {
                    read_header(h, "Referer").unwrap_or_default()
                } else {
                    String::new()
                };

                let mut method_pwstr = PWSTR::null();
                let method = if req.Method(&mut method_pwstr).is_ok() {
                    take_pwstr(method_pwstr)
                } else {
                    "GET".to_string()
                };

                if adblock.should_block_request(&url, &source_url, request_type, &method) {
                    let (status_code, status_reason, headers_str) =
                        blocked_response_spec(request_type);
                    let status = HSTRING::from(status_reason);
                    let resp_headers = HSTRING::from(headers_str);
                    if let Ok(resp) = env.CreateWebResourceResponse(
                        None,
                        status_code,
                        &status,
                        &resp_headers,
                    ) {
                        args.SetResponse(&resp)?;
                        let count = blocked_counter.fetch_add(1, Ordering::Relaxed) + 1;
                        let _ = adblock.record_block();
                        let now_ms = start.elapsed().as_millis() as u64;
                        let prev_ms = last_notify_ms.load(Ordering::Relaxed);
                        if count == 1 || now_ms.saturating_sub(prev_ms) >= 150 {
                            last_notify_ms.store(now_ms, Ordering::Relaxed);
                            let _ = proxy.send_event(AppEvent::TabAdBlocked { tab_id, count });
                        }
                    }
                }

                Ok(())
            })),
            &mut token,
        )?;
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn blocked_response_spec(request_type: &str) -> (i32, &'static str, &'static str) {
    match request_type {
        "script" => (
            200,
            "OK",
            "Content-Type: application/javascript; charset=utf-8\r\nAccess-Control-Allow-Origin: *\r\nCache-Control: no-store\r\n",
        ),
        "stylesheet" => (
            200,
            "OK",
            "Content-Type: text/css; charset=utf-8\r\nAccess-Control-Allow-Origin: *\r\nCache-Control: no-store\r\n",
        ),
        "image" | "ping" | "media" | "csp_report" => (
            204,
            "No Content",
            "Access-Control-Allow-Origin: *\r\nCache-Control: no-store\r\n",
        ),
        _ => (403, "Blocked", "Content-Type: text/plain\r\n"),
    }
}

#[cfg(target_os = "windows")]
fn read_header(headers: &ICoreWebView2HttpRequestHeaders, name: &str) -> Option<String> {
    let mut value = PWSTR::null();
    // SAFETY: `headers` is a valid COM request-headers handle obtained from `req.Headers()`.
    unsafe { headers.GetHeader(&HSTRING::from(name), &mut value) }.ok()?;
    let text = take_pwstr(value);
    if text.is_empty() { None } else { Some(text) }
}

#[cfg(target_os = "windows")]
fn map_resource_context(ctx: COREWEBVIEW2_WEB_RESOURCE_CONTEXT) -> &'static str {
    match ctx {
        c if c == COREWEBVIEW2_WEB_RESOURCE_CONTEXT_SCRIPT => "script",
        c if c == COREWEBVIEW2_WEB_RESOURCE_CONTEXT_IMAGE => "image",
        c if c == COREWEBVIEW2_WEB_RESOURCE_CONTEXT_STYLESHEET => "stylesheet",
        c if c == COREWEBVIEW2_WEB_RESOURCE_CONTEXT_XML_HTTP_REQUEST
            || c == COREWEBVIEW2_WEB_RESOURCE_CONTEXT_FETCH =>
        {
            "xmlhttprequest"
        }
        c if c == COREWEBVIEW2_WEB_RESOURCE_CONTEXT_MEDIA => "media",
        c if c == COREWEBVIEW2_WEB_RESOURCE_CONTEXT_FONT => "font",
        c if c == COREWEBVIEW2_WEB_RESOURCE_CONTEXT_PING
            || c == COREWEBVIEW2_WEB_RESOURCE_CONTEXT_EVENT_SOURCE =>
        {
            "ping"
        }
        c if c == COREWEBVIEW2_WEB_RESOURCE_CONTEXT_WEBSOCKET => "websocket",
        c if c == COREWEBVIEW2_WEB_RESOURCE_CONTEXT_CSP_VIOLATION_REPORT => "csp_report",
        _ => "other",
    }
}

pub fn webview2_runtime_version() -> String {
    #[cfg(target_os = "windows")]
    {
        let mut pwstr = PWSTR::null();
        if unsafe { GetAvailableCoreWebView2BrowserVersionString(PCWSTR::null(), &mut pwstr) }
            .is_ok()
        {
            let ver = take_pwstr(pwstr);
            if !ver.trim().is_empty() {
                return ver;
            }
        }
    }
    "WebView2".to_string()
}

pub fn content_data_dir() -> PathBuf {
    env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(env::temp_dir)
        .join("Browser")
        .join("webview2-content")
}

fn serve_newtab_asset(req: &Request<Vec<u8>>) -> Response<Cow<'static, [u8]>> {
    let (mime, body): (&str, &'static [u8]) = match req.uri().path() {
        "/newtab.css" => ("text/css; charset=utf-8", NEWTAB_CSS),
        "/newtab.js" => ("text/javascript; charset=utf-8", NEWTAB_JS),
        "/welcome" => ("text/html; charset=utf-8", WELCOME_HTML),
        "/welcome.css" => ("text/css; charset=utf-8", WELCOME_CSS),
        "/welcome.js" => ("text/javascript; charset=utf-8", WELCOME_JS),
        "/settings" => ("text/html; charset=utf-8", SETTINGS_HTML),
        "/settings.css" => ("text/css; charset=utf-8", SETTINGS_CSS),
        "/settings.js" => ("text/javascript; charset=utf-8", SETTINGS_JS),
        "/blog" => ("text/html; charset=utf-8", RUTHENIUM_BLOG_HTML),
        "/ruthenium-logo.svg" => ("image/svg+xml", RUTHENIUM_LOGO_SVG),
        "/ruthenium-logo-3d.svg" => ("image/svg+xml", RUTHENIUM_LOGO_3D_SVG),
        _ => ("text/html; charset=utf-8", EMPTY_VIEWPORT_HTML),
    };

    Response::builder()
        .header(CONTENT_TYPE, mime)
        .header(CACHE_CONTROL, "no-store, no-cache, must-revalidate")
        .status(200)
        .body(Cow::Borrowed(body))
        .expect("static newtab asset response must be valid")
}

fn generate_capability_token(tab_id: TabId) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let addr = (&nanos as *const u128 as usize) as u128;
    let mixed = nanos ^ addr.rotate_left(29) ^ ((tab_id.0 as u128) << 64);
    format!("{mixed:032x}")
}

fn build_content_bridge_script(token: &str, adblock_enabled: bool) -> String {
    let enabled_lit = if adblock_enabled { "true" } else { "false" };
    let base_css_lit = escape_js_string(BASE_AGGRESSIVE_COSMETIC_CSS);
    format!(
        r#"(() => {{
  if (!window.ipc || typeof window.ipc.postMessage !== "function") return;
  const rawPost = window.ipc.postMessage.bind(window.ipc);
  const TOKEN = "{token}";
  const send = (msg) => rawPost(TOKEN + ":" + msg);

  if (location.hostname === "browser.localhost") {{
    Object.defineProperties(window, {{
      __newtabNavigate: {{
        value: (query) => {{
          if (typeof query === "string" && query.trim()) {{
            send("newtab:go:" + query.trim());
          }}
        }},
        configurable: false,
        writable: false,
      }},
      __newtabNewTab: {{
        value: () => send("key:new_tab"),
        configurable: false,
        writable: false,
      }},
      __newtabToggleSidebar: {{
        value: () => send("key:toggle_sidebar"),
        configurable: false,
        writable: false,
      }},
      __newtabSetTheme: {{
        value: (mode) => {{
          if (mode === "midnight" || mode === "arctic" || mode === "aurora" || mode === "dark" || mode === "light") {{
            send("newtab:theme:" + mode);
          }}
        }},
        configurable: false,
        writable: false,
      }},
      __newtabSetSearchEngine: {{
        value: (engine) => {{
          if (engine === "google" || engine === "duckduckgo" || engine === "bing") {{
            send("newtab:engine:" + engine);
          }}
        }},
        configurable: false,
        writable: false,
      }},
      __newtabSetProfileName: {{
        value: (name) => {{
          if (typeof name === "string") {{
            send("newtab:name:" + name.trim().slice(0, 32));
          }}
        }},
        configurable: false,
        writable: false,
      }},
      __newtabRemoveSearchHistory: {{
        value: (query) => {{
          if (typeof query === "string" && query.trim()) {{
            send("newtab:history:remove:" + query.trim());
          }}
        }},
        configurable: false,
        writable: false,
      }},
      __newtabClearSearchHistory: {{
        value: () => send("newtab:history:clear"),
        configurable: false,
        writable: false,
      }},
      __welcomeComplete: {{
        value: () => send("welcome:complete"),
        configurable: false,
        writable: false,
      }},
      __welcomeSetDefaultBrowser: {{
        value: () => send("welcome:default_browser"),
        configurable: false,
        writable: false,
      }},
      __settingsCommand: {{
        value: (cmd) => {{
          if (typeof cmd === "string" && cmd.trim()) {{
            send("settings:" + cmd);
          }}
        }},
        configurable: false,
        writable: false,
      }},
    }});
  }}
  try {{ delete window.ipc; }} catch (_) {{}}

  window.addEventListener("keydown", (e) => {{
    if (!e.isTrusted) return;
    const key = e.key;
    const lower = key.toLowerCase();

    if (e.ctrlKey && !e.altKey) {{
      if (key === "Tab") {{
        e.preventDefault();
        e.stopPropagation();
        send(e.shiftKey ? "key:prev_tab" : "key:next_tab");
        return;
      }}
      if (!e.shiftKey && lower === "t") {{
        e.preventDefault();
        e.stopPropagation();
        send("key:new_tab");
        return;
      }}
      if ((!e.shiftKey && lower === "w") || key === "F4") {{
        e.preventDefault();
        e.stopPropagation();
        send("key:close_tab");
        return;
      }}
      if (!e.shiftKey && lower === "l") {{
        e.preventDefault();
        e.stopPropagation();
        send("key:focus_address");
        return;
      }}
      if (!e.shiftKey && lower === "b") {{
        e.preventDefault();
        e.stopPropagation();
        send("key:toggle_sidebar");
        return;
      }}
      if (!e.shiftKey && lower === "r") {{
        e.preventDefault();
        e.stopPropagation();
        send("key:reload");
        return;
      }}
      if (!e.shiftKey && key === ",") {{
        e.preventDefault();
        e.stopPropagation();
        send("key:open_settings");
        return;
      }}
      if (!e.shiftKey && key >= "1" && key <= "9") {{
        e.preventDefault();
        e.stopPropagation();
        send("key:slot:" + key);
        return;
      }}
    }}

    if (e.altKey && !e.ctrlKey && !e.shiftKey) {{
      if (lower === "d") {{
        e.preventDefault();
        e.stopPropagation();
        send("key:focus_address");
        return;
      }}
      if (key === "ArrowLeft") {{
        e.preventDefault();
        e.stopPropagation();
        send("key:back");
        return;
      }}
      if (key === "ArrowRight") {{
        e.preventDefault();
        e.stopPropagation();
        send("key:forward");
        return;
      }}
    }}

    if (!e.ctrlKey && !e.altKey && !e.shiftKey && key === "F5") {{
      e.preventDefault();
      e.stopPropagation();
      send("key:reload");
    }}
  }}, true);

  if (location.hostname !== "browser.localhost") {{
    let adblockActive = {enabled_lit};
    const BASE_COSMETIC_CSS = {base_css_lit};

    const syncBaseCosmetic = () => {{
      if (!adblockActive) {{
        for (const id of ["__ruthenium_adblock_base", "__ruthenium_adblock_cosmetic", "__ruthenium_adblock_generic"]) {{
          const node = document.getElementById(id);
          if (node) node.remove();
        }}
        return;
      }}
      const parent = document.head || document.documentElement;
      if (!parent) return;
      let el = document.getElementById("__ruthenium_adblock_base");
      if (!el) {{
        el = document.createElement("style");
        el.id = "__ruthenium_adblock_base";
        el.textContent = BASE_COSMETIC_CSS;
        parent.appendChild(el);
      }} else if (el.textContent !== BASE_COSMETIC_CSS) {{
        el.textContent = BASE_COSMETIC_CSS;
      }}
    }};

    syncBaseCosmetic();
    window.addEventListener("DOMContentLoaded", syncBaseCosmetic, {{ once: true }});

    const nativeOpen = window.open;
    if (typeof nativeOpen === "function") {{
      window.open = function () {{
        if (adblockActive && navigator.userActivation && !navigator.userActivation.isActive) {{
          return null;
        }}
        return nativeOpen.apply(this, arguments);
      }};
    }}

    Object.defineProperty(window, "__setAdblockState", {{
      value: (enabled) => {{
        adblockActive = Boolean(enabled);
        syncBaseCosmetic();
      }},
      configurable: false,
      writable: false,
    }});

    if (window.top === window) {{
      const reportFavicon = () => {{
        const el = document.querySelector('link[rel~="icon"][href]');
        if (!el) return;
        try {{
          const resolved = new URL(el.getAttribute("href"), location.href).href;
          send("favicon:" + resolved);
        }} catch (_) {{}}
      }};
      window.addEventListener("DOMContentLoaded", reportFavicon, {{ once: true }});
      window.addEventListener("load", reportFavicon, {{ once: true }});

      const VALID_TOKEN_RE = /^[A-Za-z0-9_-]{{1,64}}$/;
      const seenClasses = new Set();
      const seenIds = new Set();
      let pendingClasses = [];
      let pendingIds = [];
      let probeTimer = null;

      const flushCosmeticProbe = () => {{
        probeTimer = null;
        if (!adblockActive) {{
          pendingClasses = [];
          pendingIds = [];
          return;
        }}
        while (pendingClasses.length > 0 || pendingIds.length > 0) {{
          const clsBatch = pendingClasses.splice(0, 120);
          const idBatch = pendingIds.splice(0, 120);
          if (clsBatch.length > 0 || idBatch.length > 0) {{
            send("adblock:probe:" + clsBatch.join(",") + "|" + idBatch.join(","));
          }}
        }}
      }};

      const collectNodeTokens = (el) => {{
        if (!adblockActive || !el || el.nodeType !== 1) return;
        const rawId = el.id;
        if (typeof rawId === "string" && VALID_TOKEN_RE.test(rawId) && !seenIds.has(rawId) && seenIds.size < 4000) {{
          seenIds.add(rawId);
          pendingIds.push(rawId);
        }}
        const cl = el.classList;
        if (cl && cl.length > 0) {{
          for (let i = 0; i < cl.length; i += 1) {{
            const tok = cl[i];
            if (VALID_TOKEN_RE.test(tok) && !seenClasses.has(tok) && seenClasses.size < 4000) {{
              seenClasses.add(tok);
              pendingClasses.push(tok);
            }}
          }}
        }}
      }};

      const scheduleCosmeticProbe = () => {{
        if (!adblockActive || (pendingClasses.length === 0 && pendingIds.length === 0)) return;
        if (probeTimer !== null) return;
        probeTimer = setTimeout(flushCosmeticProbe, 180);
      }};

      const scanInitialElements = () => {{
        if (!adblockActive) return;
        const all = document.querySelectorAll("[id],[class]");
        const limit = Math.min(all.length, 1500);
        for (let i = 0; i < limit; i += 1) {{
          collectNodeTokens(all[i]);
        }}
        scheduleCosmeticProbe();

        const root = document.body || document.documentElement;
        if (root && typeof MutationObserver === "function") {{
          const obs = new MutationObserver((mutations) => {{
            if (!adblockActive) return;
            let scanned = 0;
            for (const m of mutations) {{
              for (const node of m.addedNodes) {{
                if (node.nodeType !== 1) continue;
                collectNodeTokens(node);
                scanned += 1;
                if (scanned < 250 && node.querySelectorAll) {{
                  const nested = node.querySelectorAll("[id],[class]");
                  const nLimit = Math.min(nested.length, 250 - scanned);
                  for (let j = 0; j < nLimit; j += 1) {{
                    collectNodeTokens(nested[j]);
                    scanned += 1;
                  }}
                }}
                if (scanned >= 250) break;
              }}
              if (scanned >= 250) break;
            }}
            scheduleCosmeticProbe();
          }});
          obs.observe(root, {{ childList: true, subtree: true }});
        }}
      }};

      if (document.readyState === "loading") {{
        window.addEventListener("DOMContentLoaded", scanInitialElements, {{ once: true }});
      }} else {{
        scanInitialElements();
      }}

      if (/(^|\.)youtube\.com$/i.test(location.hostname)) {{
        let savedMuted = null;
        let handlingAd = false;
        setInterval(() => {{
          if (!adblockActive || document.hidden) return;
          const skipBtn = document.querySelector(
            ".ytp-skip-ad-button, .ytp-ad-skip-button, .ytp-ad-skip-button-modern, .ytp-ad-skip-button-slot button",
          );
          if (skipBtn && typeof skipBtn.click === "function") {{
            skipBtn.click();
          }}
          const player = document.getElementById("movie_player");
          const video = document.querySelector("video.html5-main-video");
          const isAd = Boolean(
            player && (player.classList.contains("ad-showing") || player.classList.contains("ad-interrupting")),
          );
          if (video && isAd) {{
            if (!handlingAd) {{
              handlingAd = true;
              savedMuted = video.muted;
            }}
            video.muted = true;
            video.playbackRate = 16;
            if (Number.isFinite(video.duration) && video.duration > 0 && video.currentTime < video.duration - 0.1) {{
              video.currentTime = video.duration;
            }}
          }} else if (handlingAd && video) {{
            handlingAd = false;
            if (savedMuted !== null) {{
              video.muted = savedMuted;
              savedMuted = null;
            }}
            if (video.playbackRate > 2) {{
              video.playbackRate = 1;
            }}
          }}
          const popup = document.querySelector("ytd-enforcement-message-view-model");
          if (popup) {{
            const dialog = popup.closest("tp-yt-paper-dialog") || popup;
            dialog.remove();
            for (const backdrop of document.querySelectorAll("tp-yt-iron-overlay-backdrop")) {{
              backdrop.remove();
            }}
            if (video && video.paused && !isAd) {{
              video.play().catch(() => {{}});
            }}
          }}
        }}, 350);
      }}
    }}
  }}
}})();"#
    )
}
