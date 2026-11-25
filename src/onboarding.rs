use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const FIRST_RUN_FILE: &str = "first-run.json";

fn first_run_file_path() -> PathBuf {
    env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(env::temp_dir)
        .join("Browser")
        .join(FIRST_RUN_FILE)
}

pub fn is_first_run_completed() -> bool {
    first_run_file_path().is_file()
}

pub fn mark_first_run_completed() -> std::io::Result<()> {
    let target = first_run_file_path();
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = target.with_extension("tmp");
    fs::write(&tmp, b"{\"completed\":true}\n")?;
    if target.exists() {
        let _ = fs::remove_file(&target);
    }
    fs::rename(&tmp, &target)
}

pub fn parse_startup_url_from_args<I, S>(args: I) -> Option<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    for raw in args.into_iter().skip(1) {
        let trimmed = raw.as_ref().trim();
        if trimmed.is_empty() || trimmed.starts_with('-') {
            continue;
        }

        let lower = trimmed.to_ascii_lowercase();
        if lower.starts_with("http://")
            || lower.starts_with("https://")
            || lower.starts_with("file://")
        {
            return Some(trimmed.to_string());
        }
        if crate::ipc::is_internal_welcome(trimmed) {
            return Some(crate::ipc::INTERNAL_WELCOME_URL.to_string());
        }
        if crate::ipc::is_internal_newtab(trimmed) {
            return Some(crate::ipc::INTERNAL_NEWTAB_URL.to_string());
        }
        if crate::ipc::is_internal_blog(trimmed) {
            return Some(crate::ipc::INTERNAL_BLOG_URL.to_string());
        }
        if let Some(file_url) = local_html_path_to_file_url(trimmed) {
            return Some(file_url);
        }
    }
    None
}

fn local_html_path_to_file_url(raw: &str) -> Option<String> {
    let path = Path::new(raw);
    if !path.is_file() {
        return None;
    }
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    if ext != "html" && ext != "htm" {
        return None;
    }

    let abs = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let raw_str = abs.to_string_lossy();
    let clean = raw_str.strip_prefix(r"\\?\").unwrap_or(&raw_str);
    let slashed = clean.replace('\\', "/").replace(' ', "%20");
    if slashed.starts_with('/') {
        Some(format!("file://{slashed}"))
    } else {
        Some(format!("file:///{slashed}"))
    }
}

#[cfg(target_os = "windows")]
pub fn register_default_browser_hkcu() {
    win32_reg::register_default_browser();
}

#[cfg(not(target_os = "windows"))]
pub fn register_default_browser_hkcu() {}

#[cfg(target_os = "windows")]
pub fn is_default_browser_windows() -> bool {
    win32_reg::is_default_browser()
}

#[cfg(not(target_os = "windows"))]
pub fn is_default_browser_windows() -> bool {
    false
}

#[cfg(target_os = "windows")]
pub fn open_windows_default_apps_settings() {
    win32_reg::register_default_browser();
    win32_reg::open_default_apps_ui();
}

#[cfg(not(target_os = "windows"))]
pub fn open_windows_default_apps_settings() {}

#[cfg(target_os = "windows")]
mod win32_reg {
    use std::env;
    use std::ffi::c_void;
    use std::ptr;

    const HKEY_CURRENT_USER: isize = 0x8000_0001u32 as i32 as isize;
    const KEY_READ: u32 = 0x0002_0019;
    const KEY_WRITE: u32 = 0x0002_0006;
    const REG_SZ: u32 = 1;
    const ERROR_SUCCESS: i32 = 0;
    const SW_SHOWNORMAL: i32 = 1;

    const APP_REG_NAME: &str = "Ruthenium";
    const PROG_ID: &str = "RutheniumHTM";
    const CAPABILITIES_PATH: &str = "Software\\Clients\\StartMenuInternet\\Ruthenium\\Capabilities";
    const APP_DESC: &str = "Browser desktop cepat dan privat berbasis Rust dan WebView2";

    #[link(name = "advapi32")]
    unsafe extern "system" {
        fn RegCreateKeyExW(
            hkey: isize,
            lp_sub_key: *const u16,
            reserved: u32,
            lp_class: *const u16,
            dw_options: u32,
            sam_desired: u32,
            lp_security_attributes: *const c_void,
            phk_result: *mut isize,
            lpdw_disposition: *mut u32,
        ) -> i32;
        fn RegOpenKeyExW(
            hkey: isize,
            lp_sub_key: *const u16,
            ul_options: u32,
            sam_desired: u32,
            phk_result: *mut isize,
        ) -> i32;
        fn RegSetValueExW(
            hkey: isize,
            lp_value_name: *const u16,
            reserved: u32,
            dw_type: u32,
            lp_data: *const u8,
            cb_data: u32,
        ) -> i32;
        fn RegQueryValueExW(
            hkey: isize,
            lp_value_name: *const u16,
            lp_reserved: *mut u32,
            lp_type: *mut u32,
            lp_data: *mut u8,
            lpcb_data: *mut u32,
        ) -> i32;
        fn RegCloseKey(hkey: isize) -> i32;
    }

    #[link(name = "shell32")]
    unsafe extern "system" {
        fn ShellExecuteW(
            hwnd: isize,
            lp_operation: *const u16,
            lp_file: *const u16,
            lp_parameters: *const u16,
            lp_directory: *const u16,
            n_show_cmd: i32,
        ) -> isize;
    }

    struct RegKeyGuard(isize);

    impl Drop for RegKeyGuard {
        fn drop(&mut self) {
            if self.0 != 0 {
                // SAFETY: Handle was opened by `RegCreateKeyExW` or `RegOpenKeyExW` and closed once.
                unsafe {
                    RegCloseKey(self.0);
                }
            }
        }
    }

    fn to_wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn set_reg_sz(sub_key: &str, value_name: Option<&str>, data: &str) {
        let wide_sub_key = to_wide(sub_key);
        let mut hkey: isize = 0;
        let status = unsafe {
            RegCreateKeyExW(
                HKEY_CURRENT_USER,
                wide_sub_key.as_ptr(),
                0,
                ptr::null(),
                0,
                KEY_WRITE,
                ptr::null(),
                &mut hkey,
                ptr::null_mut(),
            )
        };
        if status != ERROR_SUCCESS || hkey == 0 {
            return;
        }
        let _guard = RegKeyGuard(hkey);

        let wide_name = value_name.map(to_wide);
        let name_ptr = wide_name
            .as_ref()
            .map_or(ptr::null(), |units| units.as_ptr());
        let wide_data = to_wide(data);
        let cb_data = (wide_data.len() * std::mem::size_of::<u16>()) as u32;

        unsafe {
            let _ = RegSetValueExW(
                hkey,
                name_ptr,
                0,
                REG_SZ,
                wide_data.as_ptr().cast(),
                cb_data,
            );
        }
    }

    pub(super) fn register_default_browser() {
        let Ok(exe) = env::current_exe() else {
            return;
        };
        let raw_exe = exe.to_string_lossy();
        let exe_path = raw_exe.strip_prefix(r"\\?\").unwrap_or(&raw_exe);
        let icon_ref = format!("{exe_path},0");
        let open_url_cmd = format!("\"{exe_path}\" \"%1\"");
        let launch_cmd = format!("\"{exe_path}\"");

        let prog_root = format!("Software\\Classes\\{PROG_ID}");
        set_reg_sz(&prog_root, None, "Ruthenium HTML Document");
        set_reg_sz(&prog_root, Some("AppUserModelId"), "Ruthenium.Browser");
        set_reg_sz(&format!("{prog_root}\\DefaultIcon"), None, &icon_ref);
        set_reg_sz(
            &format!("{prog_root}\\Application"),
            Some("ApplicationName"),
            APP_REG_NAME,
        );
        set_reg_sz(
            &format!("{prog_root}\\Application"),
            Some("ApplicationDescription"),
            APP_DESC,
        );
        set_reg_sz(
            &format!("{prog_root}\\Application"),
            Some("ApplicationIcon"),
            &icon_ref,
        );
        set_reg_sz(
            &format!("{prog_root}\\shell\\open\\command"),
            None,
            &open_url_cmd,
        );

        let client_root = format!("Software\\Clients\\StartMenuInternet\\{APP_REG_NAME}");
        set_reg_sz(&client_root, None, APP_REG_NAME);
        set_reg_sz(&format!("{client_root}\\DefaultIcon"), None, &icon_ref);
        set_reg_sz(
            &format!("{client_root}\\shell\\open\\command"),
            None,
            &launch_cmd,
        );
        set_reg_sz(CAPABILITIES_PATH, Some("ApplicationName"), APP_REG_NAME);
        set_reg_sz(
            CAPABILITIES_PATH,
            Some("ApplicationDescription"),
            APP_DESC,
        );
        set_reg_sz(CAPABILITIES_PATH, Some("ApplicationIcon"), &icon_ref);
        set_reg_sz(
            &format!("{CAPABILITIES_PATH}\\Startmenu"),
            Some("StartMenuInternet"),
            APP_REG_NAME,
        );
        set_reg_sz(
            &format!("{CAPABILITIES_PATH}\\URLAssociations"),
            Some("http"),
            PROG_ID,
        );
        set_reg_sz(
            &format!("{CAPABILITIES_PATH}\\URLAssociations"),
            Some("https"),
            PROG_ID,
        );
        set_reg_sz(
            &format!("{CAPABILITIES_PATH}\\FileAssociations"),
            Some(".htm"),
            PROG_ID,
        );
        set_reg_sz(
            &format!("{CAPABILITIES_PATH}\\FileAssociations"),
            Some(".html"),
            PROG_ID,
        );

        set_reg_sz(
            "Software\\RegisteredApplications",
            Some(APP_REG_NAME),
            CAPABILITIES_PATH,
        );
    }

    pub(super) fn is_default_browser() -> bool {
        let sub_key =
            to_wide("Software\\Microsoft\\Windows\\Shell\\Associations\\UrlAssociations\\https\\UserChoice");
        let mut hkey: isize = 0;
        let open_status = unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, sub_key.as_ptr(), 0, KEY_READ, &mut hkey) };
        if open_status != ERROR_SUCCESS || hkey == 0 {
            return false;
        }
        let _guard = RegKeyGuard(hkey);

        let val_name = to_wide("ProgId");
        let mut reg_type: u32 = 0;
        let mut buf = [0u16; 128];
        let mut cb_data = (buf.len() * std::mem::size_of::<u16>()) as u32;

        let query_status = unsafe {
            RegQueryValueExW(
                hkey,
                val_name.as_ptr(),
                ptr::null_mut(),
                &mut reg_type,
                buf.as_mut_ptr().cast(),
                &mut cb_data,
            )
        };
        if query_status != ERROR_SUCCESS || reg_type != REG_SZ {
            return false;
        }

        let word_len = (cb_data as usize / std::mem::size_of::<u16>()).min(buf.len());
        let end = buf[..word_len]
            .iter()
            .position(|&unit| unit == 0)
            .unwrap_or(word_len);
        let prog_id = String::from_utf16_lossy(&buf[..end]);
        prog_id.trim().eq_ignore_ascii_case(PROG_ID)
    }

    pub(super) fn open_default_apps_ui() {
        let op = to_wide("open");
        let target = to_wide("ms-settings:defaultapps?registeredAppUser=Ruthenium");
        let res = unsafe {
            ShellExecuteW(
                0,
                op.as_ptr(),
                target.as_ptr(),
                ptr::null(),
                ptr::null(),
                SW_SHOWNORMAL,
            )
        };
        if res <= 32 {
            let fallback = to_wide("ms-settings:defaultapps");
            unsafe {
                let _ = ShellExecuteW(
                    0,
                    op.as_ptr(),
                    fallback.as_ptr(),
                    ptr::null(),
                    ptr::null(),
                    SW_SHOWNORMAL,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parses_cli_http_https_internal_and_ignores_flags() {
        assert_eq!(
            parse_startup_url_from_args([
                "Ruthenium.exe",
                "--no-sandbox",
                "https://github.com/uBlockOrigin/uAssets",
            ])
            .as_deref(),
            Some("https://github.com/uBlockOrigin/uAssets")
        );
        assert_eq!(
            parse_startup_url_from_args(["Ruthenium.exe", "browser://welcome"]).as_deref(),
            Some(crate::ipc::INTERNAL_WELCOME_URL)
        );
        assert_eq!(
            parse_startup_url_from_args(["Ruthenium.exe", "browser://blog"]).as_deref(),
            Some(crate::ipc::INTERNAL_BLOG_URL)
        );
        assert!(parse_startup_url_from_args(["Ruthenium.exe", "--flag-only", ""]).is_none());
    }

    #[test]
    fn converts_existing_local_html_file_arg_to_file_url() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let temp_file = env::temp_dir().join(format!("ruthenium test page {nanos}.html"));
        fs::write(&temp_file, b"<!doctype html><title>Test</title>")
            .expect("temp html file should be written");

        let parsed = parse_startup_url_from_args([
            "Ruthenium.exe",
            temp_file.to_string_lossy().as_ref(),
        ]);
        let _ = fs::remove_file(&temp_file);

        let url = parsed.expect("existing .html file arg should resolve to file:// url");
        assert!(url.starts_with("file:///"));
        assert!(!url.contains(' '));
        assert!(url.ends_with(".html"));
    }
}
