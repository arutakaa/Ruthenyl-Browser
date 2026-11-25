use std::borrow::Cow;

use tao::dpi::LogicalSize;
use tao::event_loop::{EventLoopProxy, EventLoopWindowTarget};
#[cfg(target_os = "windows")]
use tao::platform::windows::{WindowBuilderExtWindows, WindowExtWindows};
use tao::window::{Window, WindowBuilder, WindowId};
#[cfg(target_os = "windows")]
use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Environment;
use wry::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use wry::http::{Request, Response};
#[cfg(not(target_os = "windows"))]
use wry::WebContext;
use wry::{WebView, WebViewBuilder};
#[cfg(target_os = "windows")]
use wry::{ScrollBarStyle, Theme, WebViewBuilderExtWindows};

use crate::error::BrowserError;
use crate::ipc::{AppEvent, OmniboxPopupBounds, escape_js_string, parse_omnibox_popup_ipc};

const OMNIBOX_POPUP_HTML: &[u8] = include_bytes!("../assets/omnibox_popup.html");
const OMNIBOX_POPUP_CSS: &[u8] = include_bytes!("../assets/omnibox_popup.css");
const OMNIBOX_POPUP_JS: &[u8] = include_bytes!("../assets/omnibox_popup.js");

const SHADOW_PAD_X: f64 = 16.0;
const SHADOW_PAD_BOTTOM: f64 = 24.0;
const TOP_SEAM_OVERLAP: f64 = 1.0;

pub struct OmniboxPopup {
    // Drop the WebView before the backing popup Window.
    webview: WebView,
    window: Window,
    visible: bool,
}

impl OmniboxPopup {
    pub fn create(
        event_loop: &EventLoopWindowTarget<AppEvent>,
        owner: &Window,
        #[cfg(target_os = "windows")] shared_env: ICoreWebView2Environment,
        #[cfg(not(target_os = "windows"))] context: &mut WebContext,
        proxy: EventLoopProxy<AppEvent>,
    ) -> Result<Self, BrowserError> {
        let builder = WindowBuilder::new()
            .with_decorations(false)
            .with_transparent(true)
            .with_visible(false)
            .with_focused(false)
            .with_resizable(false)
            .with_inner_size(LogicalSize::new(1.0, 1.0));

        #[cfg(target_os = "windows")]
        let builder = builder
            .with_owner_window(owner.hwnd())
            .with_skip_taskbar(true)
            .with_undecorated_shadow(false);

        let window = builder
            .build(event_loop)
            .map_err(BrowserError::WindowCreate)?;

        #[cfg(target_os = "windows")]
        win32::set_tool_window(window.hwnd());

        #[cfg(target_os = "windows")]
        let wv_builder = WebViewBuilder::new()
            .with_environment(shared_env)
            .with_profile_name("ruthenium-shell");
        #[cfg(not(target_os = "windows"))]
        let wv_builder = WebViewBuilder::new_with_web_context(context);

        let wv_builder = wv_builder
            .with_transparent(true)
            .with_visible(true)
            .with_focused(false)
            .with_asynchronous_custom_protocol("browser".into(), |_id, req, responder| {
                responder.respond(serve_popup_asset(&req));
            })
            .with_url("browser://localhost/omnibox-popup")
            .with_ipc_handler(move |req: Request<String>| {
                if let Some(event) = parse_omnibox_popup_ipc(req.body()) {
                    let _ = proxy.send_event(event);
                }
            });

        #[cfg(target_os = "windows")]
        let wv_builder = wv_builder
            .with_theme(Theme::Light)
            .with_scroll_bar_style(ScrollBarStyle::FluentOverlay)
            .with_browser_accelerator_keys(false);

        let webview = wv_builder
            .build(&window)
            .map_err(BrowserError::ShellInit)?;

        Ok(Self {
            webview,
            window,
            visible: false,
        })
    }

    pub fn window_id(&self) -> WindowId {
        self.window.id()
    }

    pub fn show_and_render(
        &mut self,
        owner: &Window,
        bounds: &OmniboxPopupBounds,
        payload_json: &str,
    ) {
        let script = format!(
            "if (window.__omniboxRenderJson) window.__omniboxRenderJson({});",
            escape_js_string(payload_json),
        );
        let _ = self.webview.evaluate_script(&script);

        let scale = owner.scale_factor();
        let origin = owner.inner_position().unwrap_or_default();
        let phys_x = origin.x + ((bounds.x - SHADOW_PAD_X) * scale).round() as i32;
        let phys_y = origin.y + ((bounds.y - TOP_SEAM_OVERLAP) * scale).round() as i32;
        let phys_w = ((bounds.width + SHADOW_PAD_X * 2.0) * scale)
            .round()
            .max(1.0) as i32;
        let phys_h = ((bounds.height + SHADOW_PAD_BOTTOM + TOP_SEAM_OVERLAP) * scale)
            .round()
            .max(1.0) as i32;

        #[cfg(target_os = "windows")]
        win32::show_at(self.window.hwnd(), phys_x, phys_y, phys_w, phys_h);

        #[cfg(not(target_os = "windows"))]
        {
            self.window
                .set_outer_position(tao::dpi::PhysicalPosition::new(phys_x, phys_y));
            self.window
                .set_inner_size(tao::dpi::PhysicalSize::new(phys_w as u32, phys_h as u32));
            self.window.set_visible(true);
        }

        self.visible = true;
    }

    pub fn select_index(&self, index: i32) {
        let script = format!("if (window.__omniboxSelect) window.__omniboxSelect({index});");
        let _ = self.webview.evaluate_script(&script);
    }

    pub fn set_theme(&self, theme: &str) {
        let script = format!(
            "if (window.__omniboxSetTheme) window.__omniboxSetTheme({});",
            escape_js_string(theme),
        );
        let _ = self.webview.evaluate_script(&script);
    }

    pub fn hide(&mut self) {
        if !self.visible {
            return;
        }
        self.visible = false;

        #[cfg(target_os = "windows")]
        win32::hide(self.window.hwnd());

        #[cfg(not(target_os = "windows"))]
        self.window.set_visible(false);
    }

    pub fn is_pointer_or_focus_in_popup(&self) -> bool {
        if !self.visible {
            return false;
        }
        #[cfg(target_os = "windows")]
        {
            win32::is_pointer_or_focus_in_popup(self.window.hwnd())
        }
        #[cfg(not(target_os = "windows"))]
        {
            false
        }
    }

    pub fn is_foreground(&self) -> bool {
        if !self.visible {
            return false;
        }
        #[cfg(target_os = "windows")]
        {
            win32::is_foreground(self.window.hwnd())
        }
        #[cfg(not(target_os = "windows"))]
        {
            false
        }
    }
}

fn serve_popup_asset(req: &Request<Vec<u8>>) -> Response<Cow<'static, [u8]>> {
    let (mime, body): (&str, &'static [u8]) = match req.uri().path() {
        "/omnibox-popup.css" => ("text/css; charset=utf-8", OMNIBOX_POPUP_CSS),
        "/omnibox-popup.js" => ("text/javascript; charset=utf-8", OMNIBOX_POPUP_JS),
        _ => ("text/html; charset=utf-8", OMNIBOX_POPUP_HTML),
    };

    Response::builder()
        .header(CONTENT_TYPE, mime)
        .header(CACHE_CONTROL, "no-store, no-cache, must-revalidate")
        .status(200)
        .body(Cow::Borrowed(body))
        .expect("static omnibox popup asset response must be valid")
}

#[cfg(target_os = "windows")]
mod win32 {
    const GWL_EXSTYLE: i32 = -20;
    const WS_EX_TOOLWINDOW: i32 = 0x0000_0080;
    const HWND_TOP: isize = 0;
    const SW_HIDE: i32 = 0;
    const SWP_NOACTIVATE: u32 = 0x0010;
    const SWP_SHOWWINDOW: u32 = 0x0040;
    const SWP_NOOWNERZORDER: u32 = 0x0200;

    #[repr(C)]
    struct Point {
        x: i32,
        y: i32,
    }

    #[repr(C)]
    #[derive(Default)]
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    #[link(name = "user32")]
    unsafe extern "system" {
        fn ShowWindow(hwnd: isize, n_cmd_show: i32) -> i32;
        fn SetWindowPos(
            hwnd: isize,
            hwnd_insert_after: isize,
            x: i32,
            y: i32,
            cx: i32,
            cy: i32,
            flags: u32,
        ) -> i32;
        fn GetForegroundWindow() -> isize;
        fn GetCursorPos(lp_point: *mut Point) -> i32;
        fn GetWindowRect(hwnd: isize, lp_rect: *mut Rect) -> i32;
        fn GetWindowLongW(hwnd: isize, n_index: i32) -> i32;
        fn SetWindowLongW(hwnd: isize, n_index: i32, dw_new_long: i32) -> i32;
    }

    pub(super) fn set_tool_window(hwnd: isize) {
        unsafe {
            let ex = GetWindowLongW(hwnd, GWL_EXSTYLE);
            SetWindowLongW(hwnd, GWL_EXSTYLE, ex | WS_EX_TOOLWINDOW);
        }
    }

    pub(super) fn show_at(hwnd: isize, x: i32, y: i32, w: i32, h: i32) {
        unsafe {
            SetWindowPos(
                hwnd,
                HWND_TOP,
                x,
                y,
                w,
                h,
                SWP_NOACTIVATE | SWP_SHOWWINDOW | SWP_NOOWNERZORDER,
            );
        }
    }

    pub(super) fn hide(hwnd: isize) {
        unsafe {
            ShowWindow(hwnd, SW_HIDE);
        }
    }

    pub(super) fn is_foreground(hwnd: isize) -> bool {
        unsafe { GetForegroundWindow() == hwnd }
    }

    pub(super) fn is_pointer_or_focus_in_popup(hwnd: isize) -> bool {
        if is_foreground(hwnd) {
            return true;
        }
        let mut rect = Rect::default();
        let mut pt = Point { x: 0, y: 0 };
        if unsafe { GetWindowRect(hwnd, &mut rect) } != 0 && unsafe { GetCursorPos(&mut pt) } != 0 {
            return pt.x >= rect.left && pt.x < rect.right && pt.y >= rect.top && pt.y < rect.bottom;
        }
        false
    }
}
