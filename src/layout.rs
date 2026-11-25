use wry::Rect;
use wry::dpi::{LogicalPosition, LogicalSize};

pub const TOP_BAR_HEIGHT: f64 = 92.0;
pub const DEFAULT_SIDEBAR_WIDTH: f64 = 220.0;
pub const MIN_SIDEBAR_WIDTH: f64 = 180.0;
pub const MAX_SIDEBAR_WIDTH: f64 = 320.0;
pub const DRAG_COLLAPSE_THRESHOLD: f64 = 120.0;
pub const SIDEBAR_AUTO_CLOSE_WIDTH: f64 = 640.0;

#[derive(Debug, Clone, PartialEq)]
pub struct ShellLayout {
    window_width: f64,
    window_height: f64,
    preferred_sidebar_width: f64,
    sidebar_open: bool,
}

impl ShellLayout {
    pub fn new(window_width: f64, window_height: f64) -> Self {
        Self {
            window_width: window_width.max(1.0),
            window_height: window_height.max(1.0),
            preferred_sidebar_width: DEFAULT_SIDEBAR_WIDTH,
            sidebar_open: false,
        }
    }

    pub fn set_window_size(&mut self, width: f64, height: f64) {
        self.window_width = width.max(1.0);
        self.window_height = height.max(1.0);
        if self.window_width < SIDEBAR_AUTO_CLOSE_WIDTH {
            self.sidebar_open = false;
        }
    }

    pub fn toggle_sidebar(&mut self) {
        self.set_sidebar_open(!self.sidebar_open);
    }

    pub fn set_sidebar_open(&mut self, open: bool) {
        if open && self.window_width < SIDEBAR_AUTO_CLOSE_WIDTH {
            return;
        }
        self.sidebar_open = open;
    }

    pub fn drag_sidebar_to(&mut self, cursor_x: f64) {
        if cursor_x < DRAG_COLLAPSE_THRESHOLD {
            self.sidebar_open = false;
        } else {
            self.sidebar_open = true;
            self.preferred_sidebar_width = cursor_x.clamp(MIN_SIDEBAR_WIDTH, MAX_SIDEBAR_WIDTH);
        }
    }

    pub fn reset_sidebar_width(&mut self) {
        self.sidebar_open = self.window_width >= SIDEBAR_AUTO_CLOSE_WIDTH;
        self.preferred_sidebar_width = DEFAULT_SIDEBAR_WIDTH;
    }

    pub fn is_compact(&self) -> bool {
        !self.sidebar_open
    }

    pub fn effective_sidebar_width(&self) -> f64 {
        if !self.sidebar_open {
            0.0
        } else {
            self.preferred_sidebar_width
                .min((self.window_width - 300.0).max(MIN_SIDEBAR_WIDTH))
        }
    }

    pub fn chrome_bounds(&self) -> Rect {
        Rect {
            position: LogicalPosition::new(0.0, 0.0).into(),
            size: LogicalSize::new(self.window_width, self.window_height).into(),
        }
    }

    pub fn content_bounds(&self) -> Rect {
        let x = self.effective_sidebar_width();
        let y = TOP_BAR_HEIGHT;
        let width = (self.window_width - x).max(1.0);
        let height = (self.window_height - y).max(1.0);

        Rect {
            position: LogicalPosition::new(x, y).into(),
            size: LogicalSize::new(width, height).into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_desktop_layout_starts_with_closed_sidebar() {
        let layout = ShellLayout::new(1280.0, 800.0);
        assert!(layout.is_compact());
        assert_eq!(layout.effective_sidebar_width(), 0.0);

        let content = layout.content_bounds();
        assert_eq!(content.position.to_logical::<f64>(1.0).x, 0.0);
        assert_eq!(content.position.to_logical::<f64>(1.0).y, 92.0);
    }

    #[test]
    fn toggling_sidebar_shifts_content_bounds() {
        let mut layout = ShellLayout::new(1280.0, 800.0);
        layout.toggle_sidebar();
        assert!(!layout.is_compact());
        assert_eq!(layout.effective_sidebar_width(), 220.0);
        assert_eq!(
            layout.content_bounds().position.to_logical::<f64>(1.0).x,
            220.0
        );

        layout.set_window_size(600.0, 700.0);
        assert!(layout.is_compact());
        assert_eq!(layout.effective_sidebar_width(), 0.0);
    }

    #[test]
    fn sidebar_drag_clamps_and_collapses_below_threshold() {
        let mut layout = ShellLayout::new(1280.0, 800.0);
        layout.drag_sidebar_to(410.0);
        assert_eq!(layout.effective_sidebar_width(), MAX_SIDEBAR_WIDTH);

        layout.drag_sidebar_to(90.0);
        assert!(layout.is_compact());
        assert_eq!(layout.effective_sidebar_width(), 0.0);
    }
}
