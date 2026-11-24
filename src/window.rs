use tao::dpi::LogicalSize;
use tao::event_loop::EventLoopWindowTarget;
#[cfg(target_os = "windows")]
use tao::platform::windows::WindowBuilderExtWindows;
use tao::window::{Window, WindowBuilder};

use crate::error::BrowserError;

pub const DEFAULT_WIDTH: f64 = 1280.0;
pub const DEFAULT_HEIGHT: f64 = 800.0;
const MIN_WIDTH: f64 = 520.0;
const MIN_HEIGHT: f64 = 360.0;

pub fn create_main_window<T>(
    event_loop: &EventLoopWindowTarget<T>,
) -> Result<Window, BrowserError> {
    let builder = WindowBuilder::new()
        .with_title("Ruthenyl")
        .with_inner_size(LogicalSize::new(DEFAULT_WIDTH, DEFAULT_HEIGHT))
        .with_min_inner_size(LogicalSize::new(MIN_WIDTH, MIN_HEIGHT))
        .with_decorations(false);

    #[cfg(target_os = "windows")]
    let builder = builder.with_undecorated_shadow(true);

    builder
        .build(event_loop)
        .map_err(BrowserError::WindowCreate)
}
