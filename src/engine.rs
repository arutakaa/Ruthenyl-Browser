use std::borrow::Cow;
use std::env;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use tao::event_loop::EventLoopProxy;
use tao::window::Window;
use wry::http::header::CONTENT_TYPE;
use wry::http::{Request, Response};
use wry::{PageLoadEvent, Rect, WebContext, WebView, WebViewBuilder};
#[cfg(target_os = "windows")]
use wry::{ScrollBarStyle, Theme, WebViewBuilderExtWindows};

use crate::error::BrowserError;
use crate::ipc::{AppEvent, TabId, escape_js_string, parse_authenticated_content_ipc};

const EMPTY_VIEWPORT_HTML: &[u8] = include_bytes!("../assets/empty_viewport.html");
const RUTHENYL_BLOG_HTML: &[u8] = include_bytes!("../assets/ruthenyl_blog.html");
const NEWTAB_CSS: &[u8] = include_bytes!("../assets/newtab.css");
const NEWTAB_JS: &[u8] = include_bytes!("../assets/newtab.js");
const DARK_VIEWPORT_BG: (u8, u8, u8, u8) = (18, 22, 28, 255);
const LIGHT_VIEWPORT_BG: (u8, u8, u8, u8) = (244, 247, 251, 255);

pub struct ContentEngine {
    webview: WebView,
}

impl ContentEngine {
    pub fn attach(
        window: &Window,
        tab_id: TabId,
        context: &mut WebContext,
        bounds: Rect,
        visible: bool,
        is_dark: bool,
        proxy: EventLoopProxy<AppEvent>,
    ) -> Result<Self, BrowserError> {
        let cap_token = generate_capability_token(tab_id);
        let init_script = build_content_bridge_script(&cap_token);
        let bg = if is_dark {
            DARK_VIEWPORT_BG
        } else {
            LIGHT_VIEWPORT_BG
        };

        let nav_proxy = proxy.clone();
        let title_proxy = proxy.clone();
        let load_proxy = proxy.clone();
        let ipc_proxy = proxy;

        let builder = WebViewBuilder::new_with_web_context(context)
            .with_bounds(bounds)
            .with_visible(visible)
            .with_focused(false)
            .with_background_color(bg)
            .with_initialization_script(&init_script)
            .with_custom_protocol("browser".into(), |_id, req| serve_newtab_asset(&req))
            .with_url("browser://localhost/newtab")
            .with_navigation_handler(move |url| {
                let _ = nav_proxy.send_event(AppEvent::TabNavigated { tab_id, url });
                true
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

    pub fn sync_profile_prefs(&self, name: &str, engine: &str) {
        let script = format!(
            "if (window.__syncProfileName) window.__syncProfileName({}); if (window.__syncSearchEngine) window.__syncSearchEngine({});",
            escape_js_string(name),
            escape_js_string(engine),
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
        "/blog" => ("text/html; charset=utf-8", RUTHENYL_BLOG_HTML),
        _ => ("text/html; charset=utf-8", EMPTY_VIEWPORT_HTML),
    };

    Response::builder()
        .header(CONTENT_TYPE, mime)
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

fn build_content_bridge_script(token: &str) -> String {
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
          if (mode === "dark" || mode === "light") {{
            send("newtab:theme:" + mode);
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

  if (window.top === window && location.hostname !== "browser.localhost") {{
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
  }}
}})();"#
    )
}
