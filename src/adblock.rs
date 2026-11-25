use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::{Duration, SystemTime};

use adblock::Engine;
use adblock::lists::{FilterSet, ParseOptions};
use adblock::request::Request;
use tao::event_loop::EventLoopProxy;

use crate::ipc::AppEvent;

const CACHE_TTL: Duration = Duration::from_secs(24 * 60 * 60);
const ENGINE_CACHE_FILE: &str = "engine-v2-aggressive.dat";

struct FilterListSource {
    file_name: &'static str,
    host: &'static str,
    path: &'static str,
}

const FILTER_SOURCES: &[FilterListSource] = &[
    FilterListSource {
        file_name: "uassets-filters.txt",
        host: "raw.githubusercontent.com",
        path: "/uBlockOrigin/uAssets/master/filters/filters.txt",
    },
    FilterListSource {
        file_name: "uassets-privacy.txt",
        host: "raw.githubusercontent.com",
        path: "/uBlockOrigin/uAssets/master/filters/privacy.txt",
    },
    FilterListSource {
        file_name: "uassets-badware.txt",
        host: "raw.githubusercontent.com",
        path: "/uBlockOrigin/uAssets/master/filters/badware.txt",
    },
    FilterListSource {
        file_name: "uassets-quick-fixes.txt",
        host: "raw.githubusercontent.com",
        path: "/uBlockOrigin/uAssets/master/filters/quick-fixes.txt",
    },
    FilterListSource {
        file_name: "uassets-annoyances-others.txt",
        host: "raw.githubusercontent.com",
        path: "/uBlockOrigin/uAssets/master/filters/annoyances-others.txt",
    },
    FilterListSource {
        file_name: "uassets-annoyances-cookies.txt",
        host: "raw.githubusercontent.com",
        path: "/uBlockOrigin/uAssets/master/filters/annoyances-cookies.txt",
    },
    FilterListSource {
        file_name: "easylist.txt",
        host: "easylist-downloads.adblockplus.org",
        path: "/easylist.txt",
    },
    FilterListSource {
        file_name: "easyprivacy.txt",
        host: "easylist-downloads.adblockplus.org",
        path: "/easyprivacy.txt",
    },
    FilterListSource {
        file_name: "peterlowe-adservers.txt",
        host: "pgl.yoyo.org",
        path: "/adservers/serverlist.php?hostformat=adblockplus&showintro=1&mimetype=plaintext",
    },
    FilterListSource {
        file_name: "abpindo.txt",
        host: "raw.githubusercontent.com",
        path: "/ABPindo/indonesianadblockrules/master/subscriptions/abpindo.txt",
    },
];

// Aggressive built-in rules active from 0ms while full 10 filter lists compile or offline.
const BOOTSTRAP_RULES: &str = "\
||doubleclick.net^\n\
||googlesyndication.com^\n\
||googleadservices.com^\n\
||adservice.google.com^\n\
||adservice.google.co.id^\n\
||google-analytics.com^\n\
||googletagmanager.com^\n\
||adnxs.com^\n\
||ads.pubmatic.com^\n\
||rubiconproject.com^\n\
||openx.net^\n\
||casalemedia.com^\n\
||criteo.com^\n\
||criteo.net^\n\
||taboola.com^\n\
||outbrain.com^\n\
||mgid.com^\n\
||popads.net^\n\
||popcash.net^\n\
||propellerads.com^\n\
||scorecardresearch.com^\n\
||quantserve.com^\n\
||amazon-adsystem.com^\n\
||moatads.com^\n\
||adsafeprotected.com^\n\
||ads.yahoo.com^\n\
||analytics.tiktok.com^\n\
||ads.tiktok.com^\n\
||connect.facebook.net/signals/\n\
||pixel.facebook.com^\n\
||an.facebook.com^\n\
||ads.youtube.com^\n\
||adsterra.com^\n\
||highrevenuegate.com^\n\
||profitablegate.com^\n\
||exoclick.com^\n\
||juicyads.com^\n\
||trafficjunky.net^\n\
||trafficstars.com^\n\
||hilltopads.net^\n\
||clickadu.com^\n\
||adcash.com^\n\
||admaven.com^\n\
||revcontent.com^\n\
||bidvertiser.com^\n\
||infolinks.com^\n\
||histats.com^\n\
||statcounter.com^\n\
||hotjar.com^\n\
||clarity.ms^\n\
||doubleverify.com^\n\
||adform.net^\n\
||advertising.com^\n\
||adroll.com^\n\
||smartadserver.com^\n\
||teads.tv^\n\
||spotxchange.com^\n\
||sharethrough.com^\n\
||indexww.com^\n\
||33across.com^\n\
||media.net^\n\
||zemanta.com^\n\
||yieldmo.com^\n\
||adcolony.com^\n\
||applovin.com^\n\
||unityads.unity3d.com^\n\
||vungle.com^\n\
||kiosiklan.com^\n\
||klikiklan.com^\n\
||adplus.co.id^\n\
||id.adplus.co.id^\n\
||ads.kompas.com^\n\
||ads.detik.com^\n\
||ads.tribunnews.com^\n\
||s.youtube.com/api/stats/ads\n\
||youtube.com/pagead/\n\
||youtube.com/api/stats/qoe?*adformat=\n\
||youtube.com/ptracking\n\
||popads.net^$popup\n\
||popcash.net^$popup\n\
||propellerads.com^$popup\n\
||adsterra.com^$popup\n\
||exoclick.com^$popup\n\
||clickadu.com^$popup\n\
||adcash.com^$popup\n\
||admaven.com^$popup\n\
||hilltopads.net^$popup\n\
";

struct AdblockStatus {
    loading: bool,
    summary: String,
}

struct AdblockInner {
    enabled: AtomicBool,
    total_blocked: AtomicU64,
    refresh_in_flight: AtomicBool,
    engine: RwLock<Arc<Engine>>,
    whitelist: RwLock<HashSet<String>>,
    status: RwLock<AdblockStatus>,
}

#[derive(Clone)]
pub struct AdblockService {
    inner: Arc<AdblockInner>,
}

impl Default for AdblockService {
    fn default() -> Self {
        Self::new()
    }
}

impl AdblockService {
    pub fn new() -> Self {
        Self::from_rules_text(BOOTSTRAP_RULES, "Aktif (Agresif) · Filter dasar siap")
    }

    fn from_rules_text(rules: &str, summary: &str) -> Self {
        let mut filter_set = FilterSet::new(false);
        filter_set.add_filter_list(rules.to_string(), ParseOptions::default());
        let engine = Engine::new_with_filter_set(filter_set);

        Self {
            inner: Arc::new(AdblockInner {
                enabled: AtomicBool::new(true),
                total_blocked: AtomicU64::new(0),
                refresh_in_flight: AtomicBool::new(false),
                engine: RwLock::new(Arc::new(engine)),
                whitelist: RwLock::new(HashSet::new()),
                status: RwLock::new(AdblockStatus {
                    loading: false,
                    summary: summary.to_string(),
                }),
            }),
        }
    }

    #[cfg(test)]
    pub(crate) fn with_rules(rules: &str) -> Self {
        Self::from_rules_text(rules, "Test rules")
    }

    pub fn is_enabled(&self) -> bool {
        self.inner.enabled.load(Ordering::Relaxed)
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.inner.enabled.store(enabled, Ordering::Relaxed);
    }

    pub fn set_whitelist(&self, domains: &[String]) {
        if let Ok(mut guard) = self.inner.whitelist.write() {
            guard.clear();
            for raw in domains {
                if let Some(norm) = normalize_whitelist_domain(raw) {
                    guard.insert(norm);
                }
            }
        }
    }

    pub fn whitelist(&self) -> Vec<String> {
        let mut items: Vec<String> = self
            .inner
            .whitelist
            .read()
            .map(|g| g.iter().cloned().collect())
            .unwrap_or_default();
        items.sort();
        items
    }

    pub fn is_host_whitelisted(&self, url_or_host: &str) -> bool {
        let Some(host) = normalize_whitelist_domain(url_or_host) else {
            return false;
        };
        let Ok(guard) = self.inner.whitelist.read() else {
            return false;
        };
        if guard.contains(&host) {
            return true;
        }
        let mut slice = host.as_str();
        while let Some((_, rest)) = slice.split_once('.') {
            if guard.contains(rest) {
                return true;
            }
            slice = rest;
        }
        false
    }

    pub fn toggle_site_whitelist(&self, url_or_host: &str) -> Option<(String, bool)> {
        let host = normalize_whitelist_domain(url_or_host)?;
        let Ok(mut guard) = self.inner.whitelist.write() else {
            return None;
        };
        if guard.remove(&host) {
            return Some((host, false));
        }
        let mut slice = host.as_str();
        while let Some((_, rest)) = slice.split_once('.') {
            if guard.contains(rest) {
                let parent = rest.to_string();
                guard.remove(&parent);
                return Some((parent, false));
            }
            slice = rest;
        }
        guard.insert(host.clone());
        Some((host, true))
    }

    pub fn add_whitelist_domain(&self, raw: &str) -> Option<String> {
        let host = normalize_whitelist_domain(raw)?;
        let Ok(mut guard) = self.inner.whitelist.write() else {
            return None;
        };
        guard.insert(host.clone());
        Some(host)
    }

    pub fn remove_whitelist_domain(&self, raw: &str) -> bool {
        let Some(host) = normalize_whitelist_domain(raw) else {
            return false;
        };
        let Ok(mut guard) = self.inner.whitelist.write() else {
            return false;
        };
        guard.remove(&host)
    }

    pub fn total_blocked(&self) -> u64 {
        self.inner.total_blocked.load(Ordering::Relaxed)
    }

    pub fn record_block(&self) -> u64 {
        self.inner.total_blocked.fetch_add(1, Ordering::Relaxed) + 1
    }

    pub fn status_text(&self) -> String {
        self.inner
            .status
            .read()
            .map(|s| s.summary.clone())
            .unwrap_or_else(|_| "Aktif (Agresif) · Filter dasar".to_string())
    }

    pub fn is_refreshing(&self) -> bool {
        self.inner
            .status
            .read()
            .map(|s| s.loading)
            .unwrap_or(false)
    }

    pub fn start_background_load(&self, proxy: EventLoopProxy<AppEvent>, force_refresh: bool) {
        if self
            .inner
            .refresh_in_flight
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return;
        }

        self.set_status(true, "Memperbarui 10 daftar filter agresif...");

        let inner = Arc::clone(&self.inner);
        thread::spawn(move || {
            let _guard = RefreshFlagGuard(&inner.refresh_in_flight);
            let cache_dir = adblock_cache_dir();
            let _ = fs::create_dir_all(&cache_dir);
            let dat_path = cache_dir.join(ENGINE_CACHE_FILE);

            let mut loaded_stale_cache = false;
            if !force_refresh && let Some((cached_engine, is_fresh)) = try_load_cached_engine(&dat_path)
            {
                if let Ok(mut guard) = inner.engine.write() {
                    *guard = Arc::new(cached_engine);
                }
                if is_fresh {
                    set_inner_status(&inner, false, "Aktif (Agresif) · uAssets + EasyList + ABPindo");
                    let _ = proxy.send_event(AppEvent::AdblockUpdated);
                    return;
                }
                loaded_stale_cache = true;
                set_inner_status(&inner, true, "Memperbarui 10 daftar filter agresif...");
                let _ = proxy.send_event(AppEvent::AdblockUpdated);
            }

            let downloaded_lists: Vec<Option<String>> = thread::scope(|scope| {
                let handles: Vec<_> = FILTER_SOURCES
                    .iter()
                    .map(|source| {
                        let txt_path = cache_dir.join(source.file_name);
                        scope.spawn(move || match fetch_filter_list(source.host, source.path) {
                            Some(fresh) => {
                                let _ = write_file_atomic(&txt_path, fresh.as_bytes());
                                Some(fresh)
                            }
                            None => fs::read_to_string(&txt_path).ok().filter(|s| !s.is_empty()),
                        })
                    })
                    .collect();

                handles
                    .into_iter()
                    .map(|handle| handle.join().ok().flatten())
                    .collect()
            });

            let mut loaded_count = 0usize;
            let mut filter_set = FilterSet::new(false);
            filter_set.add_filter_list(BOOTSTRAP_RULES.to_string(), ParseOptions::default());

            for text in downloaded_lists.into_iter().flatten() {
                filter_set.add_filter_list(text, ParseOptions::default());
                loaded_count += 1;
            }

            if loaded_count > 0 {
                let compiled = Engine::new_with_filter_set(filter_set);
                let serialized = compiled.serialize();
                let _ = write_file_atomic(&dat_path, &serialized);

                if let Ok(mut guard) = inner.engine.write() {
                    *guard = Arc::new(compiled);
                }
                let summary = format!(
                    "Aktif (Agresif) · {loaded_count}/{} daftar aktif",
                    FILTER_SOURCES.len()
                );
                set_inner_status(&inner, false, &summary);
            } else if loaded_stale_cache {
                set_inner_status(&inner, false, "Aktif (Agresif) · Cache lokal (offline)");
            } else {
                set_inner_status(&inner, false, "Aktif (Agresif) · Filter dasar (offline)");
            }

            let _ = proxy.send_event(AppEvent::AdblockUpdated);
        });
    }

    fn set_status(&self, loading: bool, summary: &str) {
        set_inner_status(&self.inner, loading, summary);
    }

    pub fn should_block_request(
        &self,
        url: &str,
        source_url: &str,
        request_type: &str,
        method: &str,
    ) -> bool {
        if !self.is_enabled()
            || is_exempt_url(url)
            || is_internal_source(source_url)
            || self.is_host_whitelisted(source_url)
        {
            return false;
        }
        let Ok(request) = Request::new(url, source_url, request_type, method) else {
            return false;
        };
        let Ok(guard) = self.inner.engine.read() else {
            return false;
        };
        guard.check_network_request(&request).should_block()
    }

    pub fn should_block_popup(&self, url: &str, source_url: &str) -> bool {
        if !self.is_enabled()
            || is_exempt_url(url)
            || is_internal_source(source_url)
            || self.is_host_whitelisted(source_url)
        {
            return false;
        }
        let Ok(guard) = self.inner.engine.read() else {
            return false;
        };
        let check = |req_type: &str| {
            Request::new(url, source_url, req_type, "GET")
                .is_ok_and(|req| guard.check_network_request(&req).should_block())
        };
        check("popup") || check("document")
    }

    pub fn cosmetic_css_for_url(&self, url: &str) -> Option<String> {
        if !self.is_enabled() || is_exempt_url(url) || self.is_host_whitelisted(url) {
            return None;
        }
        let engine = self.inner.engine.read().ok().map(|g| Arc::clone(&g))?;
        let resources = engine.url_cosmetic_resources(url);
        format_hide_selectors_css(resources.hide_selectors.iter().map(String::as_str), 2500)
    }

    pub fn generic_cosmetic_css_for_elements(
        &self,
        url: &str,
        classes: &[String],
        ids: &[String],
    ) -> Option<String> {
        if !self.is_enabled()
            || is_exempt_url(url)
            || self.is_host_whitelisted(url)
            || (classes.is_empty() && ids.is_empty())
        {
            return None;
        }
        let engine = self.inner.engine.read().ok().map(|g| Arc::clone(&g))?;
        let resources = engine.url_cosmetic_resources(url);
        if resources.generichide {
            return None;
        }
        let hidden = engine.hidden_class_id_selectors(classes, ids, &resources.exceptions);
        format_hide_selectors_css(hidden.iter().map(String::as_str), 1200)
    }
}

pub fn normalize_whitelist_domain(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let host_part = if let Some((_, rest)) = trimmed.split_once("://") {
        rest.split(['/', '?', '#']).next().unwrap_or("")
    } else {
        trimmed.split(['/', '?', '#']).next().unwrap_or("")
    };
    let no_userinfo = host_part.rsplit('@').next().unwrap_or(host_part);
    let host_only = no_userinfo.split(':').next().unwrap_or("");
    let lower = host_only
        .trim()
        .trim_start_matches("www.")
        .to_ascii_lowercase();
    if lower.is_empty()
        || lower.len() > 253
        || !lower.contains('.')
        || lower.starts_with('.')
        || lower.ends_with('.')
        || lower.contains("..")
        || is_local_or_internal_host(&lower)
    {
        return None;
    }
    if !lower
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
    {
        return None;
    }
    Some(lower)
}

fn format_hide_selectors_css<'a>(
    selectors: impl IntoIterator<Item = &'a str>,
    limit: usize,
) -> Option<String> {
    let valid: Vec<&str> = selectors
        .into_iter()
        .map(str::trim)
        .filter(|sel| {
            !sel.is_empty() && !sel.contains('\0') && !sel.to_ascii_lowercase().contains("</style")
        })
        .take(limit)
        .collect();
    if valid.is_empty() {
        return None;
    }

    let mut css = String::with_capacity(valid.len() * 24);
    for chunk in valid.chunks(400) {
        for (idx, sel) in chunk.iter().enumerate() {
            if idx > 0 {
                css.push(',');
            }
            css.push_str(sel);
        }
        css.push_str("{display:none!important;}\n");
    }
    Some(css)
}

struct RefreshFlagGuard<'a>(&'a AtomicBool);

impl Drop for RefreshFlagGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

fn set_inner_status(inner: &AdblockInner, loading: bool, summary: &str) {
    if let Ok(mut guard) = inner.status.write() {
        guard.loading = loading;
        guard.summary = summary.to_string();
    }
}

fn adblock_cache_dir() -> PathBuf {
    env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(env::temp_dir)
        .join("Browser")
        .join("adblock-cache")
}

fn try_load_cached_engine(path: &Path) -> Option<(Engine, bool)> {
    let bytes = fs::read(path).ok()?;
    let mut engine = Engine::default();
    engine.deserialize(&bytes).ok()?;

    let is_fresh = fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|mtime| SystemTime::now().duration_since(mtime).ok())
        .is_some_and(|age| age < CACHE_TTL);

    Some((engine, is_fresh))
}

fn write_file_atomic(target: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = target.with_extension("tmp");
    fs::write(&tmp, bytes)?;
    if target.exists() {
        let _ = fs::remove_file(target);
    }
    fs::rename(&tmp, target)
}

fn extract_url_host(url: &str) -> &str {
    let Some((_, rest)) = url.split_once("://") else {
        return "";
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    let no_userinfo = authority.rsplit('@').next().unwrap_or(authority);
    if let Some(bracket_end) = no_userinfo.find(']')
        && no_userinfo.starts_with('[')
    {
        return &no_userinfo[..=bracket_end];
    }
    no_userinfo.split(':').next().unwrap_or("")
}

fn is_local_or_internal_host(host: &str) -> bool {
    let lower = host.trim().to_ascii_lowercase();
    lower.is_empty()
        || lower == "browser.localhost"
        || lower == "localhost"
        || lower.ends_with(".localhost")
        || lower == "127.0.0.1"
        || lower.starts_with("127.")
        || lower == "[::1]"
        || lower == "::1"
}

fn is_exempt_url(url: &str) -> bool {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return true;
    }
    let lower = trimmed.to_ascii_lowercase();
    if !(lower.starts_with("https://")
        || lower.starts_with("http://")
        || lower.starts_with("wss://")
        || lower.starts_with("ws://"))
    {
        return true;
    }
    is_local_or_internal_host(extract_url_host(&lower))
}

fn is_internal_source(source_url: &str) -> bool {
    let host = extract_url_host(source_url);
    !host.is_empty() && is_local_or_internal_host(host)
}

#[cfg(target_os = "windows")]
fn fetch_filter_list(host: &str, path: &str) -> Option<String> {
    winhttp_filter_client::fetch_https_list(host, path)
}

#[cfg(not(target_os = "windows"))]
fn fetch_filter_list(_host: &str, _path: &str) -> Option<String> {
    None
}

#[cfg(target_os = "windows")]
mod winhttp_filter_client {
    use std::ffi::c_void;
    use std::ptr;

    type HInternet = *mut c_void;

    const WINHTTP_ACCESS_TYPE_DEFAULT_PROXY: u32 = 0;
    const WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY: u32 = 4;
    const WINHTTP_FLAG_SECURE: u32 = 0x0080_0000;
    const WINHTTP_QUERY_STATUS_CODE: u32 = 19;
    const WINHTTP_QUERY_FLAG_NUMBER: u32 = 0x2000_0000;
    const INTERNET_DEFAULT_HTTPS_PORT: u16 = 443;
    const MAX_LIST_BYTES: usize = 10 * 1024 * 1024;

    #[link(name = "winhttp")]
    unsafe extern "system" {
        fn WinHttpOpen(
            psz_agent_w: *const u16,
            dw_access_type: u32,
            psz_proxy_w: *const u16,
            psz_proxy_bypass_w: *const u16,
            dw_flags: u32,
        ) -> HInternet;
        fn WinHttpConnect(
            h_session: HInternet,
            pswz_server_name: *const u16,
            n_server_port: u16,
            dw_reserved: u32,
        ) -> HInternet;
        fn WinHttpOpenRequest(
            h_connect: HInternet,
            pwsz_verb: *const u16,
            pwsz_object_name: *const u16,
            pwsz_version: *const u16,
            pwsz_referrer: *const u16,
            ppwsz_accept_types: *const *const u16,
            dw_flags: u32,
        ) -> HInternet;
        fn WinHttpSetTimeouts(
            h_internet: HInternet,
            n_resolve_timeout: i32,
            n_connect_timeout: i32,
            n_send_timeout: i32,
            n_receive_timeout: i32,
        ) -> i32;
        fn WinHttpSendRequest(
            h_request: HInternet,
            lpsz_headers: *const u16,
            dw_headers_length: u32,
            lp_optional: *mut c_void,
            dw_optional_length: u32,
            dw_total_length: u32,
            dw_context: usize,
        ) -> i32;
        fn WinHttpReceiveResponse(h_request: HInternet, lp_reserved: *mut c_void) -> i32;
        fn WinHttpQueryHeaders(
            h_request: HInternet,
            dw_info_level: u32,
            pwsz_name: *const u16,
            lp_buffer: *mut c_void,
            lpdw_buffer_length: *mut u32,
            lpdw_index: *mut u32,
        ) -> i32;
        fn WinHttpQueryDataAvailable(
            h_request: HInternet,
            lpdw_number_of_bytes_available: *mut u32,
        ) -> i32;
        fn WinHttpReadData(
            h_request: HInternet,
            lp_buffer: *mut c_void,
            dw_number_of_bytes_to_read: u32,
            lpdw_number_of_bytes_read: *mut u32,
        ) -> i32;
        fn WinHttpCloseHandle(h_internet: HInternet) -> i32;
    }

    struct InternetHandle(HInternet);

    impl Drop for InternetHandle {
        fn drop(&mut self) {
            if !self.0.is_null() {
                // SAFETY: Handle was returned by a matching WinHTTP constructor and is closed once.
                unsafe {
                    WinHttpCloseHandle(self.0);
                }
            }
        }
    }

    fn to_wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    pub(super) fn fetch_https_list(host: &str, path: &str) -> Option<String> {
        let agent = to_wide("RutheniumBrowser/1.0 (+https://github.com/uBlockOrigin/uAssets)");
        let mut raw_session = unsafe {
            WinHttpOpen(
                agent.as_ptr(),
                WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
                ptr::null(),
                ptr::null(),
                0,
            )
        };
        if raw_session.is_null() {
            raw_session = unsafe {
                WinHttpOpen(
                    agent.as_ptr(),
                    WINHTTP_ACCESS_TYPE_DEFAULT_PROXY,
                    ptr::null(),
                    ptr::null(),
                    0,
                )
            };
        }
        if raw_session.is_null() {
            return None;
        }
        let session = InternetHandle(raw_session);

        unsafe {
            WinHttpSetTimeouts(session.0, 10_000, 10_000, 10_000, 15_000);
        }

        let wide_host = to_wide(host);
        let raw_connect = unsafe {
            WinHttpConnect(session.0, wide_host.as_ptr(), INTERNET_DEFAULT_HTTPS_PORT, 0)
        };
        if raw_connect.is_null() {
            return None;
        }
        let connect = InternetHandle(raw_connect);

        let verb = to_wide("GET");
        let object_name = to_wide(path);
        let raw_req = unsafe {
            WinHttpOpenRequest(
                connect.0,
                verb.as_ptr(),
                object_name.as_ptr(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                WINHTTP_FLAG_SECURE,
            )
        };
        if raw_req.is_null() {
            return None;
        }
        let req = InternetHandle(raw_req);

        let sent = unsafe {
            WinHttpSendRequest(req.0, ptr::null(), 0, ptr::null_mut(), 0, 0, 0)
        };
        if sent == 0 || unsafe { WinHttpReceiveResponse(req.0, ptr::null_mut()) } == 0 {
            return None;
        }

        let mut status_code: u32 = 0;
        let mut status_len: u32 = std::mem::size_of::<u32>() as u32;
        let header_ok = unsafe {
            WinHttpQueryHeaders(
                req.0,
                WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
                ptr::null(),
                (&mut status_code as *mut u32).cast(),
                &mut status_len,
                ptr::null_mut(),
            )
        };
        if header_ok == 0 || status_code != 200 {
            return None;
        }

        let mut bytes: Vec<u8> = Vec::with_capacity(64 * 1024);
        let mut buf = vec![0u8; 32 * 1024];

        loop {
            let mut available: u32 = 0;
            if unsafe { WinHttpQueryDataAvailable(req.0, &mut available) } == 0 || available == 0 {
                break;
            }
            let remaining = MAX_LIST_BYTES.saturating_sub(bytes.len());
            if remaining == 0 {
                break;
            }
            let to_read = (available as usize).min(remaining).min(buf.len());
            let mut read: u32 = 0;
            if unsafe {
                WinHttpReadData(req.0, buf.as_mut_ptr().cast(), to_read as u32, &mut read)
            } == 0
                || read == 0
            {
                break;
            }
            bytes.extend_from_slice(&buf[..(read as usize).min(to_read)]);
        }

        if bytes.is_empty() {
            None
        } else {
            Some(String::from_utf8_lossy(&bytes).into_owned())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_known_ad_and_tracker_requests_and_allows_normal_sites() {
        let blocker = AdblockService::new();
        assert!(blocker.should_block_request(
            "https://pagead2.googlesyndication.com/pagead/js/adsbygoogle.js",
            "https://news.example.com/post",
            "script",
            "GET"
        ));
        assert!(blocker.should_block_request(
            "https://www.google-analytics.com/analytics.js",
            "https://news.example.com/post",
            "script",
            "GET"
        ));
        assert!(!blocker.should_block_request(
            "https://cdn.example.com/app.js",
            "https://news.example.com/post",
            "script",
            "GET"
        ));
    }

    #[test]
    fn respects_toggle_disabled_state() {
        let blocker = AdblockService::new();
        blocker.set_enabled(false);
        assert!(!blocker.is_enabled());
        assert!(!blocker.should_block_request(
            "https://doubleclick.net/ad.js",
            "https://news.example.com/",
            "script",
            "GET"
        ));
        assert!(blocker.cosmetic_css_for_url("https://news.example.com/").is_none());

        blocker.set_enabled(true);
        assert!(blocker.should_block_request(
            "https://doubleclick.net/ad.js",
            "https://news.example.com/",
            "script",
            "GET"
        ));
    }

    #[test]
    fn generates_cosmetic_css_for_matching_domain() {
        let blocker = AdblockService::with_rules("example.com##.ad-banner\nexample.com###promo-box\n");
        let css = blocker
            .cosmetic_css_for_url("https://www.example.com/article")
            .expect("cosmetic css should be generated for example.com");
        assert!(css.contains(".ad-banner"));
        assert!(css.contains("#promo-box"));
        assert!(css.contains("{display:none!important;}"));
        assert!(blocker.cosmetic_css_for_url("https://other.org/").is_none());
    }

    #[test]
    fn exempts_internal_and_localhost_urls() {
        let blocker = AdblockService::with_rules("||localhost^\n||browser.localhost^\n||google.com^\n");
        assert!(!blocker.should_block_request(
            "http://browser.localhost/suggest?q=rust",
            "http://browser.localhost/newtab",
            "xmlhttprequest",
            "GET"
        ));
        assert!(!blocker.should_block_request(
            "https://www.google.com/s2/favicons?domain=youtube.com&sz=64",
            "http://browser.localhost/newtab",
            "image",
            "GET"
        ));
        assert!(!blocker.should_block_request(
            "http://localhost:5173/main.js",
            "http://localhost:5173/",
            "script",
            "GET"
        ));
        assert!(!blocker.should_block_request(
            "http://127.0.0.1:8080/api",
            "http://127.0.0.1:8080/",
            "xmlhttprequest",
            "GET"
        ));
    }

    #[test]
    fn blocks_ad_popups_and_allows_legit_new_windows() {
        let blocker = AdblockService::new();
        assert!(blocker.should_block_popup(
            "https://popads.net/popunder.html",
            "https://streaming.example.com/watch"
        ));
        assert!(blocker.should_block_popup(
            "https://adsterra.com/landing",
            "https://news.example.com/article"
        ));
        assert!(!blocker.should_block_popup(
            "https://docs.rs/wry/latest/wry/",
            "https://news.example.com/article"
        ));
        assert!(!blocker.should_block_popup(
            "https://accounts.google.com/o/oauth2/v2/auth",
            "https://github.com/login"
        ));
    }

    #[test]
    fn generates_generic_cosmetic_css_and_respects_generichide() {
        let blocker = AdblockService::with_rules(
            "##.generic-ad-slot\n###sponsored-widget\n@@||clean.example.com^$generichide\n",
        );
        let classes = vec!["generic-ad-slot".to_string(), "article-body".to_string()];
        let ids = vec!["sponsored-widget".to_string(), "main-content".to_string()];

        let css = blocker
            .generic_cosmetic_css_for_elements("https://other.example.com/post", &classes, &ids)
            .expect("generic cosmetic CSS should match .generic-ad-slot and #sponsored-widget");
        assert!(css.contains(".generic-ad-slot"));
        assert!(css.contains("#sponsored-widget"));
        assert!(!css.contains(".article-body"));
        assert!(!css.contains("#main-content"));

        assert!(
            blocker
                .generic_cosmetic_css_for_elements("https://clean.example.com/post", &classes, &ids)
                .is_none(),
            "generichide exception must suppress generic cosmetic selectors"
        );
    }

    #[test]
    fn respects_per_site_whitelist_including_subdomains() {
        let blocker = AdblockService::with_rules("||doubleclick.net^\nkompas.com##.ad-slot\n");
        assert!(blocker.should_block_request(
            "https://doubleclick.net/ad.js",
            "https://nasional.kompas.com/read/123",
            "script",
            "GET"
        ));
        assert!(blocker.cosmetic_css_for_url("https://nasional.kompas.com/read/123").is_some());

        let added = blocker.add_whitelist_domain("https://www.Kompas.com/news");
        assert_eq!(added.as_deref(), Some("kompas.com"));
        assert!(blocker.is_host_whitelisted("https://nasional.kompas.com/read/123"));
        assert!(!blocker.should_block_request(
            "https://doubleclick.net/ad.js",
            "https://nasional.kompas.com/read/123",
            "script",
            "GET"
        ));
        assert!(blocker.cosmetic_css_for_url("https://nasional.kompas.com/read/123").is_none());

        assert_eq!(
            blocker.toggle_site_whitelist("https://nasional.kompas.com/read/123"),
            Some(("kompas.com".to_string(), false))
        );
        assert!(blocker.should_block_request(
            "https://doubleclick.net/ad.js",
            "https://nasional.kompas.com/read/123",
            "script",
            "GET"
        ));
    }
}
