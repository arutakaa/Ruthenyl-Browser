use std::borrow::Cow;
use std::collections::HashSet;
use std::thread;

use wry::RequestAsyncResponder;
use wry::http::Response;
use wry::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use wry::http::Request;

use crate::ipc::{escape_js_string, percent_encode_query};

const MAX_QUERY_CHARS: usize = 200;
const MAX_SUGGESTIONS: usize = 8;

/// Handles asynchronous `browser://localhost/suggest?q=...` requests by fetching live Google search suggestions off the UI thread.
pub fn handle_suggest_request(req: &Request<Vec<u8>>, responder: RequestAsyncResponder) {
    let query = extract_suggest_query(req.uri().query());
    if query.is_empty() || query.chars().count() > MAX_QUERY_CHARS {
        responder.respond(json_response(b"[]".to_vec()));
        return;
    }

    thread::spawn(move || {
        let encoded = percent_encode_query(&query);
        let path_and_query = format!(
            "/complete/search?client=firefox&hl=id&ie=utf-8&oe=utf-8&q={encoded}"
        );
        let body = match fetch_google_suggest_body("suggestqueries.google.com", &path_and_query) {
            Some(raw) => {
                let items = parse_google_suggestions(&raw);
                serialize_suggestions_json(&items)
            }
            None => b"[]".to_vec(),
        };
        responder.respond(json_response(body));
    });
}

fn json_response(body: Vec<u8>) -> Response<Cow<'static, [u8]>> {
    Response::builder()
        .header(CONTENT_TYPE, "application/json; charset=utf-8")
        .header(CACHE_CONTROL, "no-store, no-cache, must-revalidate")
        .status(200)
        .body(Cow::Owned(body))
        .expect("static suggest JSON response must be valid")
}

pub fn extract_suggest_query(raw_query: Option<&str>) -> String {
    let Some(query_str) = raw_query else {
        return String::new();
    };
    for pair in query_str.split('&') {
        if let Some(raw_val) = pair.strip_prefix("q=") {
            return percent_decode_form_component(raw_val).trim().to_string();
        }
    }
    String::new()
}

fn percent_decode_form_component(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut idx = 0;
    while idx < bytes.len() {
        match bytes[idx] {
            b'+' => {
                out.push(b' ');
                idx += 1;
            }
            b'%' if idx + 2 < bytes.len() => {
                let hi = (bytes[idx + 1] as char).to_digit(16);
                let lo = (bytes[idx + 2] as char).to_digit(16);
                if let (Some(h), Some(l)) = (hi, lo) {
                    out.push(((h << 4) | l) as u8);
                    idx += 3;
                } else {
                    out.push(b'%');
                    idx += 1;
                }
            }
            byte => {
                out.push(byte);
                idx += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Parses Google's `client=firefox` OpenSearch JSON array `["query", ["item1", "item2", ...], ...]`.
pub fn parse_google_suggestions(raw_json: &str) -> Vec<String> {
    let bytes = raw_json.as_bytes();
    let mut pos = skip_ws(bytes, 0);
    if bytes.get(pos) != Some(&b'[') {
        return Vec::new();
    }
    pos = skip_ws(bytes, pos + 1);

    // Skip element 0 (echoed query string or arbitrary first value).
    pos = skip_json_value(bytes, pos);
    pos = skip_ws(bytes, pos);
    if bytes.get(pos) != Some(&b',') {
        return Vec::new();
    }
    pos = skip_ws(bytes, pos + 1);
    if bytes.get(pos) != Some(&b'[') {
        return Vec::new();
    }
    pos = skip_ws(bytes, pos + 1);

    let mut items = Vec::with_capacity(MAX_SUGGESTIONS);
    let mut seen = HashSet::with_capacity(MAX_SUGGESTIONS);

    while pos < bytes.len() {
        if bytes[pos] == b']' {
            break;
        }
        if bytes[pos] == b',' {
            pos = skip_ws(bytes, pos + 1);
            continue;
        }
        if bytes[pos] != b'"' {
            pos = skip_json_value(bytes, pos);
            pos = skip_ws(bytes, pos);
            continue;
        }

        let Some((text, next_pos)) = parse_json_string(raw_json, pos) else {
            return items;
        };
        pos = skip_ws(bytes, next_pos);

        let cleaned = text.trim();
        if !cleaned.is_empty() {
            let dedup_key = cleaned.to_lowercase();
            if seen.insert(dedup_key) {
                items.push(cleaned.to_string());
                if items.len() >= MAX_SUGGESTIONS {
                    break;
                }
            }
        }
    }

    items
}

pub fn serialize_suggestions_json(items: &[String]) -> Vec<u8> {
    let mut out = String::with_capacity(items.len() * 24 + 2);
    out.push('[');
    for (idx, item) in items.iter().enumerate() {
        if idx > 0 {
            out.push(',');
        }
        out.push_str(&escape_js_string(item));
    }
    out.push(']');
    out.into_bytes()
}

fn skip_ws(bytes: &[u8], mut pos: usize) -> usize {
    while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
        pos += 1;
    }
    pos
}

fn skip_json_value(bytes: &[u8], mut pos: usize) -> usize {
    pos = skip_ws(bytes, pos);
    if pos >= bytes.len() {
        return pos;
    }
    match bytes[pos] {
        b'"' => {
            pos += 1;
            let mut escaped = false;
            while pos < bytes.len() {
                let b = bytes[pos];
                pos += 1;
                if escaped {
                    escaped = false;
                } else if b == b'\\' {
                    escaped = true;
                } else if b == b'"' {
                    break;
                }
            }
            pos
        }
        b'[' | b'{' => {
            let mut depth = 0usize;
            let mut in_string = false;
            let mut escaped = false;
            while pos < bytes.len() {
                let b = bytes[pos];
                pos += 1;
                if in_string {
                    if escaped {
                        escaped = false;
                    } else if b == b'\\' {
                        escaped = true;
                    } else if b == b'"' {
                        in_string = false;
                    }
                } else {
                    match b {
                        b'"' => in_string = true,
                        b'[' | b'{' => depth += 1,
                        b']' | b'}' => {
                            depth = depth.saturating_sub(1);
                            if depth == 0 {
                                break;
                            }
                        }
                        _ => {}
                    }
                }
            }
            pos
        }
        _ => {
            while pos < bytes.len() && !matches!(bytes[pos], b',' | b']' | b'}' | b' ' | b'\n' | b'\r' | b'\t') {
                pos += 1;
            }
            pos
        }
    }
}

fn parse_json_string(src: &str, start_quote: usize) -> Option<(String, usize)> {
    let bytes = src.as_bytes();
    if bytes.get(start_quote) != Some(&b'"') {
        return None;
    }

    let mut pos = start_quote + 1;
    let mut out = String::new();
    let mut pending_utf16: Vec<u16> = Vec::new();

    let flush_utf16 = |out: &mut String, units: &mut Vec<u16>| {
        if units.is_empty() {
            return;
        }
        for decoded in char::decode_utf16(units.drain(..)) {
            out.push(decoded.unwrap_or('\u{FFFD}'));
        }
    };

    while pos < bytes.len() {
        let b = bytes[pos];
        if b == b'"' {
            flush_utf16(&mut out, &mut pending_utf16);
            return Some((out, pos + 1));
        }
        if b == b'\\' {
            pos += 1;
            let esc = *bytes.get(pos)?;
            if esc != b'u' {
                flush_utf16(&mut out, &mut pending_utf16);
            }
            match esc {
                b'"' => out.push('"'),
                b'\\' => out.push('\\'),
                b'/' => out.push('/'),
                b'b' => out.push('\u{0008}'),
                b'f' => out.push('\u{000C}'),
                b'n' => out.push('\n'),
                b'r' => out.push('\r'),
                b't' => out.push('\t'),
                b'u' => {
                    let hex_slice = src.get(pos + 1..pos + 5)?;
                    let unit = u16::from_str_radix(hex_slice, 16).ok()?;
                    pending_utf16.push(unit);
                    pos += 4;
                }
                _ => out.push(esc as char),
            }
            pos += 1;
            continue;
        }

        flush_utf16(&mut out, &mut pending_utf16);
        let ch = src[pos..].chars().next()?;
        out.push(ch);
        pos += ch.len_utf8();
    }

    None
}

#[cfg(target_os = "windows")]
fn fetch_google_suggest_body(host: &str, path_and_query: &str) -> Option<String> {
    winhttp_client::fetch_https_body(host, path_and_query)
}

#[cfg(not(target_os = "windows"))]
fn fetch_google_suggest_body(_host: &str, _path_and_query: &str) -> Option<String> {
    None
}

#[cfg(target_os = "windows")]
mod winhttp_client {
    use std::ffi::c_void;
    use std::ptr;

    type HInternet = *mut c_void;

    const WINHTTP_ACCESS_TYPE_DEFAULT_PROXY: u32 = 0;
    const WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY: u32 = 4;
    const WINHTTP_FLAG_SECURE: u32 = 0x0080_0000;
    const INTERNET_DEFAULT_HTTPS_PORT: u16 = 443;
    const WINHTTP_NO_REFERER: *const u16 = ptr::null();
    const WINHTTP_DEFAULT_ACCEPT_TYPES: *const *const u16 = ptr::null();
    const WINHTTP_NO_ADDITIONAL_HEADERS: *const u16 = ptr::null();
    const WINHTTP_NO_REQUEST_DATA: *mut c_void = ptr::null_mut();

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

    pub(super) fn fetch_https_body(host: &str, path_and_query: &str) -> Option<String> {
        let agent = to_wide(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
        );
        // SAFETY: `agent` is null-terminated UTF-16 valid for the call duration.
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
            // SAFETY: Fallback proxy resolution when automatic proxy lookup is unavailable.
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

        // SAFETY: Valid WinHTTP session handle.
        unsafe {
            WinHttpSetTimeouts(session.0, 2500, 2500, 2500, 2500);
        }

        let wide_host = to_wide(host);
        // SAFETY: Valid session handle and null-terminated UTF-16 host string.
        let raw_connect = unsafe {
            WinHttpConnect(session.0, wide_host.as_ptr(), INTERNET_DEFAULT_HTTPS_PORT, 0)
        };
        if raw_connect.is_null() {
            return None;
        }
        let connect = InternetHandle(raw_connect);

        let verb = to_wide("GET");
        let object_name = to_wide(path_and_query);
        // SAFETY: Valid connect handle and null-terminated UTF-16 verb and object strings.
        let raw_req = unsafe {
            WinHttpOpenRequest(
                connect.0,
                verb.as_ptr(),
                object_name.as_ptr(),
                ptr::null(),
                WINHTTP_NO_REFERER,
                WINHTTP_DEFAULT_ACCEPT_TYPES,
                WINHTTP_FLAG_SECURE,
            )
        };
        if raw_req.is_null() {
            return None;
        }
        let req = InternetHandle(raw_req);

        // SAFETY: Valid request handle; optional header/body pointers are null with zero lengths.
        let sent = unsafe {
            WinHttpSendRequest(
                req.0,
                WINHTTP_NO_ADDITIONAL_HEADERS,
                0,
                WINHTTP_NO_REQUEST_DATA,
                0,
                0,
                0,
            )
        };
        if sent == 0 {
            return None;
        }

        // SAFETY: Synchronous request already sent on `req.0`.
        if unsafe { WinHttpReceiveResponse(req.0, ptr::null_mut()) } == 0 {
            return None;
        }

        let mut bytes: Vec<u8> = Vec::with_capacity(2048);
        const MAX_BODY_BYTES: usize = 64 * 1024;

        loop {
            let mut available: u32 = 0;
            // SAFETY: Response received on `req.0`; `available` points to valid stack storage.
            if unsafe { WinHttpQueryDataAvailable(req.0, &mut available) } == 0 {
                break;
            }
            if available == 0 {
                break;
            }
            let remaining = MAX_BODY_BYTES.saturating_sub(bytes.len());
            if remaining == 0 {
                break;
            }
            let to_read = (available as usize).min(remaining).min(8192);
            let mut chunk = vec![0u8; to_read];
            let mut read: u32 = 0;
            // SAFETY: `chunk` holds `to_read` writable bytes and `read` points to stack storage.
            if unsafe {
                WinHttpReadData(req.0, chunk.as_mut_ptr().cast(), to_read as u32, &mut read)
            } == 0
            {
                break;
            }
            if read == 0 {
                break;
            }
            bytes.extend_from_slice(&chunk[..(read as usize).min(to_read)]);
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
    fn extracts_and_decodes_suggest_query_param() {
        assert_eq!(extract_suggest_query(Some("q=rust+lang")), "rust lang");
        assert_eq!(
            extract_suggest_query(Some("hl=id&q=cuaca%20jakarta%20hari%20ini")),
            "cuaca jakarta hari ini"
        );
        assert_eq!(extract_suggest_query(Some("foo=bar")), "");
        assert_eq!(extract_suggest_query(None), "");
    }

    #[test]
    fn parses_google_suggestions_handles_escapes_dedup_cap_and_malformed() {
        let sample = r#"["youtube m",["youtube music","YouTube Music","youtube\"quote\\slash","satu\u0020dua","empat","lima","enam","tujuh","delapan","sembilan","sepuluh"],[],{}]"#;
        let parsed = parse_google_suggestions(sample);
        assert_eq!(
            parsed,
            vec![
                "youtube music",
                "youtube\"quote\\slash",
                "satu dua",
                "empat",
                "lima",
                "enam",
                "tujuh",
                "delapan",
            ]
        );
        assert!(parse_google_suggestions("<html>not json</html>").is_empty());
        assert!(parse_google_suggestions(r#"["empty",[]]"#).is_empty());
    }

    #[test]
    fn serializes_suggestions_to_valid_json_array() {
        let encoded = serialize_suggestions_json(&[
            "rust language".to_string(),
            "quote \"test\"".to_string(),
        ]);
        assert_eq!(
            String::from_utf8(encoded).expect("valid utf8"),
            r#"["rust language","quote \"test\""]"#
        );
    }
}
