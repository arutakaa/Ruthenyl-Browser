use tao::event::{ElementState, KeyEvent};
use tao::keyboard::{KeyCode, ModifiersState};
use tao::window::ResizeDirection;

pub const INTERNAL_NEWTAB_URL: &str = "http://browser.localhost/newtab";
pub const INTERNAL_BLOG_URL: &str = "http://browser.localhost/blog";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TabId(pub u64);

#[derive(Debug, Clone)]
pub enum AppEvent {
    ShellReady,
    Navigate(String),
    Back,
    Forward,
    Reload,
    Stop,
    NewTab,
    SelectTab(TabId),
    CloseTab(TabId),
    CloseActiveTab,
    NextTab,
    PrevTab,
    SelectTabSlot(usize),
    FocusContent,
    FocusAddressBar,
    ToggleSidebar,
    SetSidebarOpen(bool),
    SetSidebarWidth(f64),
    ResetSidebarWidth,
    SetTheme(bool),
    SetSearchEngine(SearchEngine),
    SetProfileName(String),
    Minimize,
    ToggleMaximize,
    Close,
    DragWindow,
    DragResize(ResizeDirection),
    TabNavigated {
        tab_id: TabId,
        url: String,
    },
    TabTitleChanged {
        tab_id: TabId,
        title: String,
    },
    TabLoadState {
        tab_id: TabId,
        loading: bool,
        url: String,
    },
    TabFaviconChanged {
        tab_id: TabId,
        favicon_url: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginDisplay {
    pub raw_url: String,
    pub origin: String,
    pub path: String,
    pub security: &'static str,
}

pub fn parse_shell_ipc(message: &str) -> Option<AppEvent> {
    match message {
        "shell:ready" => Some(AppEvent::ShellReady),
        "nav:back" => Some(AppEvent::Back),
        "nav:forward" => Some(AppEvent::Forward),
        "nav:reload" => Some(AppEvent::Reload),
        "nav:stop" => Some(AppEvent::Stop),
        "tab:new" => Some(AppEvent::NewTab),
        "tab:close_active" => Some(AppEvent::CloseActiveTab),
        "tab:next" => Some(AppEvent::NextTab),
        "tab:prev" => Some(AppEvent::PrevTab),
        "content:focus" => Some(AppEvent::FocusContent),
        "sidebar:toggle" => Some(AppEvent::ToggleSidebar),
        "sidebar:open" => Some(AppEvent::SetSidebarOpen(true)),
        "sidebar:close" => Some(AppEvent::SetSidebarOpen(false)),
        "sidebar:reset_width" => Some(AppEvent::ResetSidebarWidth),
        "win:minimize" => Some(AppEvent::Minimize),
        "win:maximize" => Some(AppEvent::ToggleMaximize),
        "win:close" => Some(AppEvent::Close),
        "win:drag" => Some(AppEvent::DragWindow),
        _ => {
            if let Some(target) = message.strip_prefix("nav:go:") {
                return Some(AppEvent::Navigate(target.to_string()));
            }
            if let Some(raw_id) = message.strip_prefix("tab:select:") {
                let id = raw_id.parse::<u64>().ok()?;
                return Some(AppEvent::SelectTab(TabId(id)));
            }
            if let Some(raw_id) = message.strip_prefix("tab:close:") {
                let id = raw_id.parse::<u64>().ok()?;
                return Some(AppEvent::CloseTab(TabId(id)));
            }
            if let Some(raw_slot) = message.strip_prefix("tab:slot:") {
                let slot = raw_slot.parse::<usize>().ok()?;
                if (1..=9).contains(&slot) {
                    return Some(AppEvent::SelectTabSlot(slot));
                }
                return None;
            }
            if let Some(raw_width) = message.strip_prefix("sidebar:width:") {
                let width = raw_width.parse::<f64>().ok()?;
                return Some(AppEvent::SetSidebarWidth(width));
            }
            if let Some(raw_engine) = message.strip_prefix("prefs:engine:") {
                let engine = SearchEngine::from_wire(raw_engine)?;
                return Some(AppEvent::SetSearchEngine(engine));
            }
            if let Some(raw_name) = message.strip_prefix("prefs:name:") {
                return Some(AppEvent::SetProfileName(sanitize_profile_name(raw_name)));
            }
            if let Some(edge) = message.strip_prefix("win:resize:") {
                let dir = match edge {
                    "n" => ResizeDirection::North,
                    "s" => ResizeDirection::South,
                    "e" => ResizeDirection::East,
                    "w" => ResizeDirection::West,
                    "ne" => ResizeDirection::NorthEast,
                    "nw" => ResizeDirection::NorthWest,
                    "se" => ResizeDirection::SouthEast,
                    "sw" => ResizeDirection::SouthWest,
                    _ => return None,
                };
                return Some(AppEvent::DragResize(dir));
            }
            None
        }
    }
}

pub fn parse_authenticated_content_ipc(
    tab_id: TabId,
    expected_token: &str,
    message: &str,
) -> Option<AppEvent> {
    let payload = message.strip_prefix(expected_token)?.strip_prefix(':')?;

    match payload {
        "key:new_tab" => Some(AppEvent::NewTab),
        "key:close_tab" => Some(AppEvent::CloseActiveTab),
        "key:next_tab" => Some(AppEvent::NextTab),
        "key:prev_tab" => Some(AppEvent::PrevTab),
        "key:focus_address" => Some(AppEvent::FocusAddressBar),
        "key:toggle_sidebar" => Some(AppEvent::ToggleSidebar),
        "key:reload" => Some(AppEvent::Reload),
        "key:back" => Some(AppEvent::Back),
        "key:forward" => Some(AppEvent::Forward),
        "key:stop" => Some(AppEvent::Stop),
        "newtab:theme:dark" => Some(AppEvent::SetTheme(true)),
        "newtab:theme:light" => Some(AppEvent::SetTheme(false)),
        _ => {
            if let Some(raw_slot) = payload.strip_prefix("key:slot:") {
                let slot = raw_slot.parse::<usize>().ok()?;
                if (1..=9).contains(&slot) {
                    return Some(AppEvent::SelectTabSlot(slot));
                }
                return None;
            }
            if let Some(query) = payload.strip_prefix("newtab:go:") {
                return Some(AppEvent::Navigate(query.to_string()));
            }
            if let Some(raw_name) = payload.strip_prefix("newtab:name:") {
                return Some(AppEvent::SetProfileName(sanitize_profile_name(raw_name)));
            }
            if let Some(icon_url) = payload.strip_prefix("favicon:") {
                let trimmed = icon_url.trim();
                let valid = trimmed.starts_with("https://")
                    || trimmed.starts_with("http://")
                    || trimmed.starts_with("data:image/");
                return Some(AppEvent::TabFaviconChanged {
                    tab_id,
                    favicon_url: if valid {
                        Some(trimmed.to_string())
                    } else {
                        None
                    },
                });
            }
            None
        }
    }
}

pub fn map_window_shortcut(
    event: &KeyEvent,
    modifiers: ModifiersState,
    is_synthetic: bool,
) -> Option<AppEvent> {
    if is_synthetic || event.state != ElementState::Pressed {
        return None;
    }

    let ctrl = modifiers.control_key();
    let shift = modifiers.shift_key();
    let alt = modifiers.alt_key();

    if ctrl && !alt {
        return match event.physical_key {
            KeyCode::Tab => Some(if shift {
                AppEvent::PrevTab
            } else {
                AppEvent::NextTab
            }),
            KeyCode::KeyT if !shift && !event.repeat => Some(AppEvent::NewTab),
            KeyCode::KeyW | KeyCode::F4 if !shift && !event.repeat => {
                Some(AppEvent::CloseActiveTab)
            }
            KeyCode::KeyL if !shift && !event.repeat => Some(AppEvent::FocusAddressBar),
            KeyCode::KeyB if !shift && !event.repeat => Some(AppEvent::ToggleSidebar),
            KeyCode::KeyR if !shift && !event.repeat => Some(AppEvent::Reload),
            KeyCode::Digit1 if !shift => Some(AppEvent::SelectTabSlot(1)),
            KeyCode::Digit2 if !shift => Some(AppEvent::SelectTabSlot(2)),
            KeyCode::Digit3 if !shift => Some(AppEvent::SelectTabSlot(3)),
            KeyCode::Digit4 if !shift => Some(AppEvent::SelectTabSlot(4)),
            KeyCode::Digit5 if !shift => Some(AppEvent::SelectTabSlot(5)),
            KeyCode::Digit6 if !shift => Some(AppEvent::SelectTabSlot(6)),
            KeyCode::Digit7 if !shift => Some(AppEvent::SelectTabSlot(7)),
            KeyCode::Digit8 if !shift => Some(AppEvent::SelectTabSlot(8)),
            KeyCode::Digit9 if !shift => Some(AppEvent::SelectTabSlot(9)),
            _ => None,
        };
    }

    if alt && !ctrl && !shift && !event.repeat {
        return match event.physical_key {
            KeyCode::KeyD => Some(AppEvent::FocusAddressBar),
            KeyCode::ArrowLeft => Some(AppEvent::Back),
            KeyCode::ArrowRight => Some(AppEvent::Forward),
            _ => None,
        };
    }

    if !ctrl && !alt && !shift && !event.repeat {
        return match event.physical_key {
            KeyCode::F5 => Some(AppEvent::Reload),
            KeyCode::Escape => Some(AppEvent::Stop),
            _ => None,
        };
    }

    None
}

pub fn is_internal_newtab(url: &str) -> bool {
    let trimmed = url.trim_end_matches('/');
    trimmed.is_empty()
        || trimmed == "about:blank"
        || trimmed == "browser://newtab"
        || trimmed == "browser://localhost/newtab"
        || trimmed == "http://browser.localhost/newtab"
        || trimmed == "https://browser.localhost/newtab"
}

pub fn is_internal_blog(url: &str) -> bool {
    let trimmed = url.trim_end_matches('/');
    trimmed == "browser://blog"
        || trimmed == "browser://localhost/blog"
        || trimmed == "http://browser.localhost/blog"
        || trimmed == "https://browser.localhost/blog"
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SearchEngine {
    #[default]
    Google,
    DuckDuckGo,
    Bing,
}

impl SearchEngine {
    pub fn from_wire(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "google" => Some(Self::Google),
            "duckduckgo" => Some(Self::DuckDuckGo),
            "bing" => Some(Self::Bing),
            _ => None,
        }
    }

    pub fn as_wire(self) -> &'static str {
        match self {
            Self::Google => "google",
            Self::DuckDuckGo => "duckduckgo",
            Self::Bing => "bing",
        }
    }

    pub fn search_url(self, encoded_query: &str) -> String {
        match self {
            Self::Google => format!("https://www.google.com/search?q={encoded_query}"),
            Self::DuckDuckGo => format!("https://duckduckgo.com/?q={encoded_query}"),
            Self::Bing => format!("https://www.bing.com/search?q={encoded_query}"),
        }
    }
}

pub fn sanitize_profile_name(raw: &str) -> String {
    let trimmed: String = raw.trim().chars().take(32).collect();
    if trimmed.is_empty() {
        "Budi".to_string()
    } else {
        trimmed
    }
}

#[cfg(test)]
pub fn resolve_navigation_input(raw: &str) -> String {
    resolve_navigation_input_with_engine(raw, SearchEngine::default())
}

pub fn resolve_navigation_input_with_engine(raw: &str, engine: SearchEngine) -> String {
    let input = raw.trim();
    if is_internal_newtab(input) {
        return INTERNAL_NEWTAB_URL.to_string();
    }
    if is_internal_blog(input) {
        return INTERNAL_BLOG_URL.to_string();
    }

    if input.starts_with("https://") || input.starts_with("http://") {
        return input.to_string();
    }

    if looks_like_host_or_url(input) {
        let host_part = input.split('/').next().unwrap_or(input);
        let is_local = host_part == "localhost"
            || host_part.starts_with("localhost:")
            || host_part.starts_with("127.0.0.1")
            || host_part.starts_with("[::1]");
        let scheme = if is_local { "http" } else { "https" };
        return format!("{scheme}://{input}");
    }

    engine.search_url(&percent_encode_query(input))
}

fn looks_like_host_or_url(input: &str) -> bool {
    if input.contains(char::is_whitespace) {
        return false;
    }
    let host_port = input.split('/').next().unwrap_or(input);
    if host_port == "localhost" || host_port.starts_with("localhost:") {
        return true;
    }
    let host = host_port.split(':').next().unwrap_or(host_port);
    host.contains('.') && !host.starts_with('.') && !host.ends_with('.')
}

fn percent_encode_query(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 8);
    for b in input.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            b' ' => out.push('+'),
            _ => {
                out.push('%');
                out.push(
                    char::from_digit((b >> 4) as u32, 16)
                        .unwrap_or('0')
                        .to_ascii_uppercase(),
                );
                out.push(
                    char::from_digit((b & 0x0F) as u32, 16)
                        .unwrap_or('0')
                        .to_ascii_uppercase(),
                );
            }
        }
    }
    out
}

pub fn format_origin_display(url: &str) -> OriginDisplay {
    if is_internal_newtab(url) {
        return OriginDisplay {
            raw_url: String::new(),
            origin: String::new(),
            path: String::new(),
            security: "internal",
        };
    }

    if is_internal_blog(url) {
        return OriginDisplay {
            raw_url: INTERNAL_BLOG_URL.to_string(),
            origin: "browser://blog".to_string(),
            path: String::new(),
            security: "internal",
        };
    }

    let (scheme, rest) = match url.split_once("://") {
        Some(parts) => parts,
        None => {
            return OriginDisplay {
                raw_url: url.to_string(),
                origin: url.to_string(),
                path: String::new(),
                security: "internal",
            };
        }
    };

    let (authority, tail) = match rest.find(['/', '?', '#']) {
        Some(idx) => (&rest[..idx], &rest[idx..]),
        None => (rest, ""),
    };

    let is_localhost = authority == "localhost"
        || authority.starts_with("localhost:")
        || authority == "127.0.0.1"
        || authority.starts_with("127.0.0.1:");

    let security = if scheme.eq_ignore_ascii_case("https") {
        "https"
    } else if scheme.eq_ignore_ascii_case("http") && !is_localhost {
        "insecure"
    } else {
        "internal"
    };

    let clean_path = if tail == "/" { "" } else { tail };

    OriginDisplay {
        raw_url: url.to_string(),
        origin: authority.to_string(),
        path: clean_path.to_string(),
        security,
    }
}

pub fn escape_js_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len() + 2);
    escaped.push('"');
    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            '\u{2028}' => escaped.push_str("\\u2028"),
            '\u{2029}' => escaped.push_str("\\u2029"),
            c if c.is_control() => {
                use std::fmt::Write;
                let _ = write!(escaped, "\\u{:04x}", c as u32);
            }
            c => escaped.push(c),
        }
    }
    escaped.push('"');
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_urls_localhost_and_search_queries() {
        assert_eq!(resolve_navigation_input(""), INTERNAL_NEWTAB_URL);
        assert_eq!(
            resolve_navigation_input("docs.rs/wry"),
            "https://docs.rs/wry"
        );
        assert_eq!(
            resolve_navigation_input("localhost:3000/app"),
            "http://localhost:3000/app"
        );
        assert_eq!(
            resolve_navigation_input("rust wry browser"),
            "https://www.google.com/search?q=rust+wry+browser"
        );
        assert_eq!(
            resolve_navigation_input_with_engine("rust wry browser", SearchEngine::DuckDuckGo),
            "https://duckduckgo.com/?q=rust+wry+browser"
        );
        assert_eq!(
            resolve_navigation_input_with_engine("rust wry browser", SearchEngine::Bing),
            "https://www.bing.com/search?q=rust+wry+browser"
        );
    }

    #[test]
    fn formats_origin_and_security_cleanly() {
        let internal = format_origin_display(INTERNAL_NEWTAB_URL);
        assert_eq!(internal.origin, "");
        assert_eq!(internal.security, "internal");

        let blog = format_origin_display(INTERNAL_BLOG_URL);
        assert_eq!(blog.origin, "browser://blog");
        assert_eq!(blog.security, "internal");

        let https = format_origin_display("https://crates.io/crates/wry");
        assert_eq!(https.origin, "crates.io");
        assert_eq!(https.path, "/crates/wry");
        assert_eq!(https.security, "https");

        let local = format_origin_display("http://localhost:5173/");
        assert_eq!(local.origin, "localhost:5173");
        assert_eq!(local.path, "");
        assert_eq!(local.security, "internal");

        let http = format_origin_display("http://example.com/test");
        assert_eq!(http.security, "insecure");
    }

    #[test]
    fn rejects_unauthenticated_content_ipc_and_accepts_valid_tokens() {
        let id = TabId(7);
        assert!(parse_authenticated_content_ipc(id, "secret123", "wrong:key:new_tab").is_none());
        assert!(parse_authenticated_content_ipc(id, "secret123", "key:new_tab").is_none());
        assert!(matches!(
            parse_authenticated_content_ipc(id, "secret123", "secret123:key:close_tab"),
            Some(AppEvent::CloseActiveTab)
        ));
        assert!(matches!(
            parse_authenticated_content_ipc(id, "secret123", "secret123:key:slot:4"),
            Some(AppEvent::SelectTabSlot(4))
        ));
        assert!(parse_authenticated_content_ipc(id, "secret123", "secret123:key:slot:0").is_none());
        assert!(matches!(
            parse_authenticated_content_ipc(id, "secret123", "secret123:newtab:theme:dark"),
            Some(AppEvent::SetTheme(true))
        ));
        assert!(matches!(
            parse_authenticated_content_ipc(id, "secret123", "secret123:newtab:theme:light"),
            Some(AppEvent::SetTheme(false))
        ));
        assert!(
            parse_authenticated_content_ipc(id, "secret123", "secret123:newtab:theme:auto")
                .is_none()
        );
        assert!(
            parse_authenticated_content_ipc(id, "secret123", "wrong:newtab:theme:dark").is_none()
        );
    }
}
