use tao::event::{ElementState, KeyEvent};
use tao::keyboard::{KeyCode, ModifiersState};
use tao::window::ResizeDirection;

pub const INTERNAL_NEWTAB_URL: &str = "http://browser.localhost/newtab";
pub const INTERNAL_BLOG_URL: &str = "http://browser.localhost/blog";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TabId(pub u64);

#[derive(Debug, Clone, PartialEq)]
pub struct OmniboxPopupBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone)]
pub enum AppEvent {
    ShellReady,
    Navigate(String),
    Back,
    Forward,
    Reload,
    Stop,
    NewTab,
    OpenUrlInNewTab(String),
    SelectTab(TabId),
    CloseTab(TabId),
    CloseActiveTab,
    NextTab,
    PrevTab,
    SelectTabSlot(usize),
    FocusContent,
    FocusAddressBar,
    OmniboxOpen {
        bounds: OmniboxPopupBounds,
        payload: String,
    },
    OmniboxSelect(i32),
    OmniboxClose,
    OmniboxBlur,
    OmniboxPopupGo(String),
    OmniboxPopupFill(String),
    OmniboxPopupDeleteHistory(String),
    OmniboxPopupClearHistory,
    OmniboxPopupDismiss,
    RemoveSearchHistory(String),
    ClearSearchHistory,
    ToggleSidebar,
    SetSidebarOpen(bool),
    SetSidebarWidth(f64),
    ResetSidebarWidth,
    SetTheme(BrowserTheme),
    SetSearchEngine(SearchEngine),
    SetProfileName(String),
    SetAdblockEnabled(bool),
    RefreshAdblockLists,
    AdblockUpdated,
    TabAdBlocked {
        tab_id: TabId,
        count: u64,
    },
    CosmeticProbe {
        tab_id: TabId,
        classes: Vec<String>,
        ids: Vec<String>,
    },
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
        "omnibox:close" => Some(AppEvent::OmniboxClose),
        "omnibox:blur" => Some(AppEvent::OmniboxBlur),
        "history:clear" => Some(AppEvent::ClearSearchHistory),
        "prefs:adblock:on" => Some(AppEvent::SetAdblockEnabled(true)),
        "prefs:adblock:off" => Some(AppEvent::SetAdblockEnabled(false)),
        "adblock:refresh" => Some(AppEvent::RefreshAdblockLists),
        _ => {
            if let Some(target) = message.strip_prefix("nav:go:") {
                return Some(AppEvent::Navigate(target.to_string()));
            }
            if let Some(raw_idx) = message.strip_prefix("omnibox:select:") {
                let idx = raw_idx.parse::<i32>().ok()?;
                if (-1..=32).contains(&idx) {
                    return Some(AppEvent::OmniboxSelect(idx));
                }
                return None;
            }
            if let Some(rest) = message.strip_prefix("omnibox:open:") {
                let (coords, payload) = rest.split_once('|')?;
                if payload.is_empty() {
                    return None;
                }
                let mut parts = coords.split(',');
                let x = parts.next()?.parse::<f64>().ok()?;
                let y = parts.next()?.parse::<f64>().ok()?;
                let width = parts.next()?.parse::<f64>().ok()?;
                let height = parts.next()?.parse::<f64>().ok()?;
                if parts.next().is_some()
                    || !x.is_finite()
                    || !y.is_finite()
                    || !width.is_finite()
                    || !height.is_finite()
                    || width < 100.0
                    || height < 36.0
                {
                    return None;
                }
                return Some(AppEvent::OmniboxOpen {
                    bounds: OmniboxPopupBounds {
                        x,
                        y,
                        width,
                        height,
                    },
                    payload: payload.to_string(),
                });
            }
            if let Some(query) = message.strip_prefix("history:remove:") {
                let trimmed = query.trim();
                if !trimmed.is_empty() {
                    return Some(AppEvent::RemoveSearchHistory(trimmed.to_string()));
                }
                return None;
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
            if let Some(raw_theme) = message.strip_prefix("theme:") {
                let theme = BrowserTheme::from_wire(raw_theme)?;
                return Some(AppEvent::SetTheme(theme));
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

pub fn parse_omnibox_popup_ipc(message: &str) -> Option<AppEvent> {
    match message {
        "popup:clear_history" => Some(AppEvent::OmniboxPopupClearHistory),
        "popup:dismiss" => Some(AppEvent::OmniboxPopupDismiss),
        _ => {
            if let Some(target) = message.strip_prefix("popup:go:") {
                let trimmed = target.trim();
                if !trimmed.is_empty() {
                    return Some(AppEvent::OmniboxPopupGo(trimmed.to_string()));
                }
                return None;
            }
            if let Some(query) = message.strip_prefix("popup:fill:") {
                let trimmed = query.trim();
                if !trimmed.is_empty() {
                    return Some(AppEvent::OmniboxPopupFill(trimmed.to_string()));
                }
                return None;
            }
            if let Some(query) = message.strip_prefix("popup:del_history:") {
                let trimmed = query.trim();
                if !trimmed.is_empty() {
                    return Some(AppEvent::OmniboxPopupDeleteHistory(trimmed.to_string()));
                }
                return None;
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
        "newtab:history:clear" => Some(AppEvent::ClearSearchHistory),
        _ => {
            if let Some(raw_theme) = payload.strip_prefix("newtab:theme:") {
                let theme = BrowserTheme::from_wire(raw_theme)?;
                return Some(AppEvent::SetTheme(theme));
            }
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
            if let Some(query) = payload.strip_prefix("newtab:history:remove:") {
                let trimmed = query.trim();
                if !trimmed.is_empty() {
                    return Some(AppEvent::RemoveSearchHistory(trimmed.to_string()));
                }
                return None;
            }
            if let Some(raw_name) = payload.strip_prefix("newtab:name:") {
                return Some(AppEvent::SetProfileName(sanitize_profile_name(raw_name)));
            }
            if let Some(raw_engine) = payload.strip_prefix("newtab:engine:") {
                let engine = SearchEngine::from_wire(raw_engine)?;
                return Some(AppEvent::SetSearchEngine(engine));
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
            if let Some(raw_probe) = payload.strip_prefix("adblock:probe:") {
                let (raw_classes, raw_ids) = raw_probe.split_once('|')?;
                let classes = parse_css_ident_list(raw_classes, 128);
                let ids = parse_css_ident_list(raw_ids, 128);
                if classes.is_empty() && ids.is_empty() {
                    return None;
                }
                return Some(AppEvent::CosmeticProbe {
                    tab_id,
                    classes,
                    ids,
                });
            }
            None
        }
    }
}

fn parse_css_ident_list(raw: &str, max: usize) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|tok| {
            !tok.is_empty()
                && tok.len() <= 64
                && tok
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
        })
        .take(max)
        .map(str::to_string)
        .collect()
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
pub enum BrowserTheme {
    Midnight,
    #[default]
    Arctic,
    Aurora,
}

impl BrowserTheme {
    pub fn from_wire(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "midnight" | "dark" => Some(Self::Midnight),
            "arctic" | "light" => Some(Self::Arctic),
            "aurora" => Some(Self::Aurora),
            _ => None,
        }
    }

    pub fn as_wire(self) -> &'static str {
        match self {
            Self::Midnight => "midnight",
            Self::Arctic => "arctic",
            Self::Aurora => "aurora",
        }
    }

    pub fn is_dark(self) -> bool {
        !matches!(self, Self::Arctic)
    }
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

pub fn is_recordable_search_query(raw: &str) -> bool {
    let input = raw.trim();
    if input.is_empty() || is_internal_newtab(input) || is_internal_blog(input) {
        return false;
    }
    let lower = input.to_ascii_lowercase();
    if lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("browser://")
    {
        return false;
    }
    !looks_like_host_or_url(input)
}

pub(crate) fn percent_encode_query(input: &str) -> String {
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
            parse_authenticated_content_ipc(id, "secret123", "secret123:newtab:theme:midnight"),
            Some(AppEvent::SetTheme(BrowserTheme::Midnight))
        ));
        assert!(matches!(
            parse_authenticated_content_ipc(id, "secret123", "secret123:newtab:theme:arctic"),
            Some(AppEvent::SetTheme(BrowserTheme::Arctic))
        ));
        assert!(matches!(
            parse_authenticated_content_ipc(id, "secret123", "secret123:newtab:theme:aurora"),
            Some(AppEvent::SetTheme(BrowserTheme::Aurora))
        ));
        assert!(matches!(
            parse_authenticated_content_ipc(id, "secret123", "secret123:newtab:theme:dark"),
            Some(AppEvent::SetTheme(BrowserTheme::Midnight))
        ));
        assert!(matches!(
            parse_authenticated_content_ipc(id, "secret123", "secret123:newtab:theme:light"),
            Some(AppEvent::SetTheme(BrowserTheme::Arctic))
        ));
        assert!(
            parse_authenticated_content_ipc(id, "secret123", "secret123:newtab:theme:auto")
                .is_none()
        );
        assert!(
            parse_authenticated_content_ipc(id, "secret123", "wrong:newtab:theme:midnight")
                .is_none()
        );
        assert!(matches!(
            parse_authenticated_content_ipc(id, "secret123", "secret123:newtab:engine:duckduckgo"),
            Some(AppEvent::SetSearchEngine(SearchEngine::DuckDuckGo))
        ));
        assert!(matches!(
            parse_authenticated_content_ipc(id, "secret123", "secret123:newtab:engine:bing"),
            Some(AppEvent::SetSearchEngine(SearchEngine::Bing))
        ));
        assert!(
            parse_authenticated_content_ipc(id, "secret123", "secret123:newtab:engine:yahoo")
                .is_none()
        );
        assert!(
            parse_authenticated_content_ipc(id, "secret123", "wrong:newtab:engine:google")
                .is_none()
        );
    }

    #[test]
    fn parses_theme_ipc_messages() {
        assert_eq!(BrowserTheme::default(), BrowserTheme::Arctic);
        assert!(matches!(
            parse_shell_ipc("theme:midnight"),
            Some(AppEvent::SetTheme(BrowserTheme::Midnight))
        ));
        assert!(matches!(
            parse_shell_ipc("theme:arctic"),
            Some(AppEvent::SetTheme(BrowserTheme::Arctic))
        ));
        assert!(matches!(
            parse_shell_ipc("theme:aurora"),
            Some(AppEvent::SetTheme(BrowserTheme::Aurora))
        ));
        assert!(matches!(
            parse_shell_ipc("theme:dark"),
            Some(AppEvent::SetTheme(BrowserTheme::Midnight))
        ));
        assert!(matches!(
            parse_shell_ipc("theme:light"),
            Some(AppEvent::SetTheme(BrowserTheme::Arctic))
        ));
        assert!(parse_shell_ipc("theme:auto").is_none());
    }

    #[test]
    fn parses_omnibox_popup_and_history_ipc_messages() {
        assert!(matches!(
            parse_shell_ipc("omnibox:close"),
            Some(AppEvent::OmniboxClose)
        ));
        assert!(matches!(
            parse_shell_ipc("omnibox:blur"),
            Some(AppEvent::OmniboxBlur)
        ));
        assert!(matches!(
            parse_shell_ipc("history:clear"),
            Some(AppEvent::ClearSearchHistory)
        ));
        assert!(matches!(
            parse_shell_ipc("history:remove:rust webview2"),
            Some(AppEvent::RemoveSearchHistory(ref q)) if q == "rust webview2"
        ));
        assert!(parse_shell_ipc("history:remove:   ").is_none());

        assert!(matches!(
            parse_shell_ipc("omnibox:select:-1"),
            Some(AppEvent::OmniboxSelect(-1))
        ));
        assert!(matches!(
            parse_shell_ipc("omnibox:select:7"),
            Some(AppEvent::OmniboxSelect(7))
        ));
        assert!(parse_shell_ipc("omnibox:select:-2").is_none());
        assert!(parse_shell_ipc("omnibox:select:40").is_none());

        match parse_shell_ipc("omnibox:open:120.5,83,640,210|{\"query\":\"rust\"}") {
            Some(AppEvent::OmniboxOpen { bounds, payload }) => {
                assert_eq!(bounds.x, 120.5);
                assert_eq!(bounds.y, 83.0);
                assert_eq!(bounds.width, 640.0);
                assert_eq!(bounds.height, 210.0);
                assert_eq!(payload, "{\"query\":\"rust\"}");
            }
            _ => panic!("expected OmniboxOpen event"),
        }
        assert!(parse_shell_ipc("omnibox:open:120,83,80,210|{}").is_none());
        assert!(parse_shell_ipc("omnibox:open:120,83,500,20|{}").is_none());
        assert!(parse_shell_ipc("omnibox:open:120,83,500,200|").is_none());

        assert!(matches!(
            parse_omnibox_popup_ipc("popup:go:crates.io"),
            Some(AppEvent::OmniboxPopupGo(ref q)) if q == "crates.io"
        ));
        assert!(matches!(
            parse_omnibox_popup_ipc("popup:fill:rust lang"),
            Some(AppEvent::OmniboxPopupFill(ref q)) if q == "rust lang"
        ));
        assert!(matches!(
            parse_omnibox_popup_ipc("popup:del_history:old query"),
            Some(AppEvent::OmniboxPopupDeleteHistory(ref q)) if q == "old query"
        ));
        assert!(matches!(
            parse_omnibox_popup_ipc("popup:clear_history"),
            Some(AppEvent::OmniboxPopupClearHistory)
        ));
        assert!(matches!(
            parse_omnibox_popup_ipc("popup:dismiss"),
            Some(AppEvent::OmniboxPopupDismiss)
        ));
        assert!(parse_omnibox_popup_ipc("popup:go:  ").is_none());

        let id = TabId(9);
        assert!(matches!(
            parse_authenticated_content_ipc(id, "tok", "tok:newtab:history:clear"),
            Some(AppEvent::ClearSearchHistory)
        ));
        assert!(matches!(
            parse_authenticated_content_ipc(id, "tok", "tok:newtab:history:remove:wry rust"),
            Some(AppEvent::RemoveSearchHistory(ref q)) if q == "wry rust"
        ));
        assert!(parse_authenticated_content_ipc(id, "tok", "wrong:newtab:history:clear").is_none());
    }

    #[test]
    fn validates_recordable_search_queries() {
        assert!(!is_recordable_search_query(""));
        assert!(!is_recordable_search_query("   "));
        assert!(!is_recordable_search_query("http://browser.localhost/newtab"));
        assert!(!is_recordable_search_query("HTTPS://example.com/search"));
        assert!(!is_recordable_search_query("youtube.com"));
        assert!(!is_recordable_search_query("localhost:5173"));
        assert!(is_recordable_search_query("youtube tutorial rust"));
        assert!(is_recordable_search_query("ruthenium"));
    }

    #[test]
    fn parses_adblock_ipc_messages() {
        assert!(matches!(
            parse_shell_ipc("prefs:adblock:on"),
            Some(AppEvent::SetAdblockEnabled(true))
        ));
        assert!(matches!(
            parse_shell_ipc("prefs:adblock:off"),
            Some(AppEvent::SetAdblockEnabled(false))
        ));
        assert!(matches!(
            parse_shell_ipc("adblock:refresh"),
            Some(AppEvent::RefreshAdblockLists)
        ));

        let id = TabId(3);
        match parse_authenticated_content_ipc(
            id,
            "tok",
            "tok:adblock:probe:ad-slot,bad{color:red},hero-card|div-gpt-ad-1,bad</style>",
        ) {
            Some(AppEvent::CosmeticProbe {
                tab_id,
                classes,
                ids,
            }) => {
                assert_eq!(tab_id, id);
                assert_eq!(classes, vec!["ad-slot", "hero-card"]);
                assert_eq!(ids, vec!["div-gpt-ad-1"]);
            }
            _ => panic!("expected CosmeticProbe event"),
        }

        assert!(parse_authenticated_content_ipc(id, "tok", "tok:adblock:probe:|").is_none());
        assert!(
            parse_authenticated_content_ipc(id, "tok", "wrong:adblock:probe:ad-slot|").is_none()
        );
    }
}
