use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::ipc::{BrowserTheme, SearchEngine, escape_js_string};

const SETTINGS_FILE: &str = "settings.json";
const SESSION_FILE: &str = "session.json";
pub const VALID_ZOOMS: [u16; 6] = [80, 90, 100, 110, 125, 150];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StartupMode {
    #[default]
    RestoreSession,
    NewTab,
    CustomUrl,
}

impl StartupMode {
    pub fn from_wire(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "restore" => Some(Self::RestoreSession),
            "newtab" => Some(Self::NewTab),
            "custom" => Some(Self::CustomUrl),
            _ => None,
        }
    }

    pub fn as_wire(self) -> &'static str {
        match self {
            Self::RestoreSession => "restore",
            Self::NewTab => "newtab",
            Self::CustomUrl => "custom",
        }
    }
}

pub fn normalize_zoom(raw: u16) -> u16 {
    if VALID_ZOOMS.contains(&raw) {
        return raw;
    }
    VALID_ZOOMS
        .iter()
        .copied()
        .min_by_key(|&z| z.abs_diff(raw))
        .unwrap_or(100)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserSettings {
    pub theme: BrowserTheme,
    pub search_engine: SearchEngine,
    pub startup_mode: StartupMode,
    pub startup_url: String,
    pub default_zoom: u16,
    pub show_home_clock: bool,
    pub show_home_shortcuts: bool,
    pub suggest_enabled: bool,
    pub adblock_enabled: bool,
    pub adblock_whitelist: Vec<String>,
    pub download_dir: String,
}

impl Default for BrowserSettings {
    fn default() -> Self {
        Self {
            theme: BrowserTheme::Arctic,
            search_engine: SearchEngine::Google,
            startup_mode: StartupMode::RestoreSession,
            startup_url: String::new(),
            default_zoom: 100,
            show_home_clock: true,
            show_home_shortcuts: true,
            suggest_enabled: true,
            adblock_enabled: true,
            adblock_whitelist: Vec::new(),
            download_dir: String::new(),
        }
    }
}

impl BrowserSettings {
    pub fn effective_download_dir(&self) -> String {
        let trimmed = self.download_dir.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
        default_download_dir()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionState {
    pub urls: Vec<String>,
    pub active_index: usize,
}

pub fn default_download_dir() -> String {
    if let Some(profile) = env::var_os("USERPROFILE") {
        return PathBuf::from(profile)
            .join("Downloads")
            .to_string_lossy()
            .into_owned();
    }
    env::temp_dir().to_string_lossy().into_owned()
}

fn app_data_dir() -> PathBuf {
    env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(env::temp_dir)
        .join("Browser")
}

fn write_atomic(target: &Path, content: &str) -> std::io::Result<()> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = target.with_extension("tmp");
    fs::write(&tmp, content.as_bytes())?;
    if target.exists() {
        let _ = fs::remove_file(target);
    }
    fs::rename(&tmp, target)
}

pub fn load_settings() -> BrowserSettings {
    let path = app_data_dir().join(SETTINGS_FILE);
    match fs::read_to_string(path) {
        Ok(raw) => parse_settings_json(&raw),
        Err(_) => BrowserSettings::default(),
    }
}

pub fn save_settings(settings: &BrowserSettings) {
    let path = app_data_dir().join(SETTINGS_FILE);
    let _ = write_atomic(&path, &serialize_settings_json(settings));
}

pub fn load_session() -> SessionState {
    let path = app_data_dir().join(SESSION_FILE);
    match fs::read_to_string(path) {
        Ok(raw) => parse_session_json(&raw),
        Err(_) => SessionState::default(),
    }
}

pub fn save_session(session: &SessionState) {
    let path = app_data_dir().join(SESSION_FILE);
    let _ = write_atomic(&path, &serialize_session_json(session));
}

pub fn serialize_settings_json(s: &BrowserSettings) -> String {
    let mut whitelist_json = String::from("[");
    for (idx, item) in s.adblock_whitelist.iter().enumerate() {
        if idx > 0 {
            whitelist_json.push(',');
        }
        whitelist_json.push_str(&escape_js_string(item));
    }
    whitelist_json.push(']');

    format!(
        "{{\n  \"theme\": {},\n  \"searchEngine\": {},\n  \"startupMode\": {},\n  \"startupUrl\": {},\n  \"defaultZoom\": {},\n  \"showHomeClock\": {},\n  \"showHomeShortcuts\": {},\n  \"suggestEnabled\": {},\n  \"adblockEnabled\": {},\n  \"adblockWhitelist\": {},\n  \"downloadDir\": {}\n}}\n",
        escape_js_string(s.theme.as_wire()),
        escape_js_string(s.search_engine.as_wire()),
        escape_js_string(s.startup_mode.as_wire()),
        escape_js_string(&s.startup_url),
        normalize_zoom(s.default_zoom),
        s.show_home_clock,
        s.show_home_shortcuts,
        s.suggest_enabled,
        s.adblock_enabled,
        whitelist_json,
        escape_js_string(&s.download_dir),
    )
}

pub fn parse_settings_json(raw: &str) -> BrowserSettings {
    let mut out = BrowserSettings::default();
    let Some(JsonValue::Object(entries)) = parse_json(raw) else {
        return out;
    };

    for (key, val) in entries {
        match (key.as_str(), val) {
            ("theme", JsonValue::String(s)) => {
                if let Some(theme) = BrowserTheme::from_wire(&s) {
                    out.theme = theme;
                }
            }
            ("searchEngine", JsonValue::String(s)) => {
                if let Some(engine) = SearchEngine::from_wire(&s) {
                    out.search_engine = engine;
                }
            }
            ("startupMode", JsonValue::String(s)) => {
                if let Some(mode) = StartupMode::from_wire(&s) {
                    out.startup_mode = mode;
                }
            }
            ("startupUrl", JsonValue::String(s)) => {
                out.startup_url = s.trim().chars().take(512).collect();
            }
            ("defaultZoom", JsonValue::Number(n)) => {
                out.default_zoom = normalize_zoom(n.min(u16::MAX as u64) as u16);
            }
            ("showHomeClock", JsonValue::Bool(b)) => out.show_home_clock = b,
            ("showHomeShortcuts", JsonValue::Bool(b)) => out.show_home_shortcuts = b,
            ("suggestEnabled", JsonValue::Bool(b)) => out.suggest_enabled = b,
            ("adblockEnabled", JsonValue::Bool(b)) => out.adblock_enabled = b,
            ("adblockWhitelist", JsonValue::Array(items)) => {
                let mut cleaned = Vec::new();
                for item in items {
                    if let JsonValue::String(domain) = item
                        && let Some(norm) = crate::adblock::normalize_whitelist_domain(&domain)
                        && !cleaned.contains(&norm)
                    {
                        cleaned.push(norm);
                    }
                }
                cleaned.sort();
                out.adblock_whitelist = cleaned;
            }
            ("downloadDir", JsonValue::String(s)) => {
                out.download_dir = s.trim().chars().take(512).collect();
            }
            _ => {}
        }
    }

    out
}

pub fn serialize_session_json(session: &SessionState) -> String {
    let mut urls_json = String::from("[");
    for (idx, url) in session.urls.iter().enumerate() {
        if idx > 0 {
            urls_json.push(',');
        }
        urls_json.push_str(&escape_js_string(url));
    }
    urls_json.push(']');

    let active_idx = if session.urls.is_empty() {
        0
    } else {
        session.active_index.min(session.urls.len() - 1)
    };
    format!(
        "{{\n  \"urls\": {urls_json},\n  \"activeIndex\": {active_idx}\n}}\n"
    )
}

fn is_persistable_session_url(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    let Some(rest) = lower
        .strip_prefix("https://")
        .or_else(|| lower.strip_prefix("http://"))
    else {
        return false;
    };
    let host_port = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host = host_port.split(':').next().unwrap_or("");
    !host.is_empty() && host != "browser.localhost"
}

pub fn parse_session_json(raw: &str) -> SessionState {
    let mut out = SessionState::default();
    let Some(JsonValue::Object(entries)) = parse_json(raw) else {
        return out;
    };

    for (key, val) in entries {
        match (key.as_str(), val) {
            ("urls", JsonValue::Array(items)) => {
                for item in items.into_iter().take(24) {
                    if let JsonValue::String(url) = item
                        && is_persistable_session_url(&url)
                    {
                        out.urls.push(url);
                    }
                }
            }
            ("activeIndex", JsonValue::Number(idx)) => {
                out.active_index = idx as usize;
            }
            _ => {}
        }
    }

    if out.urls.is_empty() {
        out.active_index = 0;
    } else {
        out.active_index = out.active_index.min(out.urls.len() - 1);
    }
    out
}

#[cfg(target_os = "windows")]
pub fn pick_download_folder(owner_hwnd: isize, title: &str) -> Option<String> {
    win32_folders::pick_folder(owner_hwnd, title)
}

#[cfg(not(target_os = "windows"))]
pub fn pick_download_folder(_owner_hwnd: isize, _title: &str) -> Option<String> {
    None
}

#[cfg(target_os = "windows")]
pub fn open_folder_in_explorer(path: &str) {
    let _ = fs::create_dir_all(path);
    win32_folders::open_explorer(path);
}

#[cfg(not(target_os = "windows"))]
pub fn open_folder_in_explorer(path: &str) {
    let _ = fs::create_dir_all(path);
}

#[cfg(target_os = "windows")]
mod win32_folders {
    use std::ffi::c_void;
    use std::ptr;

    const BIF_RETURNONLYFSDIRS: u32 = 0x0001;
    const BIF_NEWDIALOGSTYLE: u32 = 0x0040;
    const SW_SHOWNORMAL: i32 = 1;

    #[repr(C)]
    struct BrowseInfoW {
        hwnd_owner: isize,
        pidl_root: *const c_void,
        psz_display_name: *mut u16,
        lpsz_title: *const u16,
        ul_flags: u32,
        lpfn: *const c_void,
        lparam: isize,
        i_image: i32,
    }

    #[link(name = "shell32")]
    unsafe extern "system" {
        fn SHBrowseForFolderW(lpbi: *mut BrowseInfoW) -> *mut c_void;
        fn SHGetPathFromIDListW(pidl: *const c_void, psz_path: *mut u16) -> i32;
        fn ShellExecuteW(
            hwnd: isize,
            lp_operation: *const u16,
            lp_file: *const u16,
            lp_parameters: *const u16,
            lp_directory: *const u16,
            n_show_cmd: i32,
        ) -> isize;
    }

    #[link(name = "ole32")]
    unsafe extern "system" {
        fn CoTaskMemFree(pv: *const c_void);
    }

    fn to_wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    pub(super) fn pick_folder(owner_hwnd: isize, title: &str) -> Option<String> {
        let wide_title = to_wide(title);
        let mut display = [0u16; 260];
        let mut bi = BrowseInfoW {
            hwnd_owner: owner_hwnd,
            pidl_root: ptr::null(),
            psz_display_name: display.as_mut_ptr(),
            lpsz_title: wide_title.as_ptr(),
            ul_flags: BIF_RETURNONLYFSDIRS | BIF_NEWDIALOGSTYLE,
            lpfn: ptr::null(),
            lparam: 0,
            i_image: 0,
        };

        let pidl = unsafe { SHBrowseForFolderW(&mut bi) };
        if pidl.is_null() {
            return None;
        }

        let mut path_buf = [0u16; 1024];
        let ok = unsafe { SHGetPathFromIDListW(pidl, path_buf.as_mut_ptr()) };
        unsafe { CoTaskMemFree(pidl) };
        if ok == 0 {
            return None;
        }

        let len = path_buf
            .iter()
            .position(|&unit| unit == 0)
            .unwrap_or(path_buf.len());
        let selected = String::from_utf16_lossy(&path_buf[..len]).trim().to_string();
        if selected.is_empty() {
            None
        } else {
            Some(selected)
        }
    }

    pub(super) fn open_explorer(path: &str) {
        let op = to_wide("open");
        let wide_path = to_wide(path);
        unsafe {
            let _ = ShellExecuteW(
                0,
                op.as_ptr(),
                wide_path.as_ptr(),
                ptr::null(),
                ptr::null(),
                SW_SHOWNORMAL,
            );
        }
    }
}

enum JsonValue {
    Null,
    Bool(bool),
    Number(u64),
    String(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
}

struct JsonParser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> JsonParser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            bytes: input.as_bytes(),
            pos: 0,
        }
    }

    fn skip_ws(&mut self) {
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.skip_ws();
        self.bytes.get(self.pos).copied()
    }

    fn expect(&mut self, ch: u8) -> bool {
        self.skip_ws();
        if self.bytes.get(self.pos) == Some(&ch) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn parse_value(&mut self) -> Option<JsonValue> {
        match self.peek()? {
            b'"' => Some(JsonValue::String(self.parse_string()?)),
            b'{' => self.parse_object(),
            b'[' => self.parse_array(),
            b't' => self.parse_lit(b"true", JsonValue::Bool(true)),
            b'f' => self.parse_lit(b"false", JsonValue::Bool(false)),
            b'n' => self.parse_lit(b"null", JsonValue::Null),
            b'0'..=b'9' => self.parse_number(),
            b'-' => {
                // Clamp negative numbers gracefully without aborting object parsing.
                self.pos += 1;
                let _ = self.parse_number()?;
                Some(JsonValue::Number(0))
            }
            _ => None,
        }
    }

    fn parse_lit(&mut self, lit: &[u8], val: JsonValue) -> Option<JsonValue> {
        if self.bytes.get(self.pos..self.pos + lit.len()) == Some(lit) {
            self.pos += lit.len();
            Some(val)
        } else {
            None
        }
    }

    fn parse_number(&mut self) -> Option<JsonValue> {
        let start = self.pos;
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_digit() {
            self.pos += 1;
        }
        let int_str = std::str::from_utf8(&self.bytes[start..self.pos]).ok()?;
        let num = int_str.parse::<u64>().ok()?;
        if self.bytes.get(self.pos) == Some(&b'.') {
            self.pos += 1;
            while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_digit() {
                self.pos += 1;
            }
        }
        Some(JsonValue::Number(num))
    }

    fn parse_string(&mut self) -> Option<String> {
        if !self.expect(b'"') {
            return None;
        }
        let mut out = String::new();
        while self.pos < self.bytes.len() {
            let b = self.bytes[self.pos];
            if b == b'"' {
                self.pos += 1;
                return Some(out);
            }
            if b == b'\\' {
                self.pos += 1;
                let esc = *self.bytes.get(self.pos)?;
                self.pos += 1;
                match esc {
                    b'"' => out.push('"'),
                    b'\\' => out.push('\\'),
                    b'/' => out.push('/'),
                    b'n' => out.push('\n'),
                    b'r' => out.push('\r'),
                    b't' => out.push('\t'),
                    b'b' => out.push('\u{0008}'),
                    b'f' => out.push('\u{000C}'),
                    b'u' => {
                        let hex = std::str::from_utf8(self.bytes.get(self.pos..self.pos + 4)?)
                            .ok()?;
                        self.pos += 4;
                        let code = u32::from_str_radix(hex, 16).ok()?;
                        if let Some(ch) = char::from_u32(code) {
                            out.push(ch);
                        }
                    }
                    _ => return None,
                }
            } else {
                let rest = std::str::from_utf8(&self.bytes[self.pos..]).ok()?;
                let ch = rest.chars().next()?;
                self.pos += ch.len_utf8();
                out.push(ch);
            }
        }
        None
    }

    fn parse_array(&mut self) -> Option<JsonValue> {
        if !self.expect(b'[') {
            return None;
        }
        let mut items = Vec::new();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Some(JsonValue::Array(items));
        }
        loop {
            items.push(self.parse_value()?);
            match self.peek()? {
                b',' => {
                    self.pos += 1;
                }
                b']' => {
                    self.pos += 1;
                    return Some(JsonValue::Array(items));
                }
                _ => return None,
            }
        }
    }

    fn parse_object(&mut self) -> Option<JsonValue> {
        if !self.expect(b'{') {
            return None;
        }
        let mut entries = Vec::new();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Some(JsonValue::Object(entries));
        }
        loop {
            let key = self.parse_string()?;
            if !self.expect(b':') {
                return None;
            }
            let val = self.parse_value()?;
            entries.push((key, val));
            match self.peek()? {
                b',' => {
                    self.pos += 1;
                }
                b'}' => {
                    self.pos += 1;
                    return Some(JsonValue::Object(entries));
                }
                _ => return None,
            }
        }
    }
}

fn parse_json(raw: &str) -> Option<JsonValue> {
    JsonParser::new(raw.trim()).parse_value()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_settings_json_and_clamps_invalid_values() {
        let original = BrowserSettings {
            theme: BrowserTheme::Aurora,
            search_engine: SearchEngine::DuckDuckGo,
            startup_mode: StartupMode::CustomUrl,
            startup_url: "https://crates.io".to_string(),
            default_zoom: 125,
            show_home_clock: false,
            show_home_shortcuts: true,
            suggest_enabled: false,
            adblock_enabled: true,
            adblock_whitelist: vec!["detik.com".to_string(), "kompas.com".to_string()],
            download_dir: "D:\\Downloads\\Ruthenium".to_string(),
        };

        let encoded = serialize_settings_json(&original);
        let decoded = parse_settings_json(&encoded);
        assert_eq!(decoded, original);

        let clamped = parse_settings_json(
            r#"{"defaultZoom":118,"startupMode":"unknown","adblockWhitelist":["HTTPS://www.Kompas.com/news","localhost"]}"#,
        );
        assert_eq!(clamped.default_zoom, 125);
        assert_eq!(clamped.startup_mode, StartupMode::RestoreSession);
        assert_eq!(clamped.adblock_whitelist, vec!["kompas.com"]);
    }

    #[test]
    fn round_trips_session_json_and_caps_active_index() {
        let original = SessionState {
            urls: vec![
                "https://github.com".to_string(),
                "https://docs.rs/wry".to_string(),
            ],
            active_index: 1,
        };

        let encoded = serialize_session_json(&original);
        let decoded = parse_session_json(&encoded);
        assert_eq!(decoded, original);

        let capped = parse_session_json(
            r#"{"urls":["http://browser.localhost/newtab","https://rust-lang.org"],"activeIndex":99}"#,
        );
        assert_eq!(capped.urls, vec!["https://rust-lang.org"]);
        assert_eq!(capped.active_index, 0);
    }
}
