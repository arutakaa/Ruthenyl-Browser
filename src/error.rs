use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum BrowserError {
    WindowCreate(tao::error::OsError),
    ShellInit(wry::Error),
    EngineInit(wry::Error),
}

impl fmt::Display for BrowserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WindowCreate(err) => write!(f, "failed to create native window: {err}"),
            Self::ShellInit(err) => write!(f, "failed to initialize browser shell webview: {err}"),
            Self::EngineInit(err) => write!(f, "failed to initialize content webview: {err}"),
        }
    }
}

impl Error for BrowserError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::WindowCreate(err) => Some(err),
            Self::ShellInit(err) | Self::EngineInit(err) => Some(err),
        }
    }
}
