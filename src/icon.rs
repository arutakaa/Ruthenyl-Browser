use tao::window::Icon;

const HEX_BASE: [(f32, f32); 6] = [
    (32.0, 4.0),
    (56.0, 18.0),
    (56.0, 46.0),
    (32.0, 60.0),
    (8.0, 46.0),
    (8.0, 18.0),
];

const SHEEN_TRI: [(f32, f32); 3] = [(32.0, 4.0), (56.0, 18.0), (32.0, 32.0)];

const R_STEM: [(f32, f32); 4] = [(19.0, 18.8), (25.5, 15.0), (25.5, 49.0), (19.0, 45.2)];
const R_UPPER: [(f32, f32); 5] = [
    (25.5, 15.0),
    (44.5, 26.0),
    (44.5, 28.8),
    (36.8, 28.8),
    (25.5, 22.3),
];
const R_RETURN: [(f32, f32); 5] = [
    (44.5, 28.8),
    (44.5, 31.6),
    (25.5, 42.6),
    (25.5, 35.3),
    (36.8, 28.8),
];
const R_LEG: [(f32, f32); 5] = [
    (27.5, 41.4),
    (34.8, 37.2),
    (45.0, 43.1),
    (45.0, 45.2),
    (38.8, 48.8),
];

const COLOR_STROKE: [u8; 3] = [0x35, 0x54, 0x8A];
const COLOR_LEFT_FACET: [u8; 3] = [0x0E, 0x17, 0x26];
const COLOR_RIGHT_FACET: [u8; 3] = [0x17, 0x27, 0x44];
const COLOR_SHEEN: [u8; 3] = [0x1F, 0x36, 0x5E];
const COLOR_R_STEM: [u8; 3] = [0xF4, 0xF7, 0xFB];
const COLOR_R_UPPER: [u8; 3] = [0xDC, 0xEB, 0xFC];
const COLOR_R_RETURN: [u8; 3] = [0x60, 0xA5, 0xFA];
const COLOR_R_LEG: [u8; 3] = [0x93, 0xC5, 0xFD];

fn scale_pt(p: (f32, f32), factor: f32) -> (f32, f32) {
    (32.0 + (p.0 - 32.0) * factor, 32.0 + (p.1 - 32.0) * factor)
}

fn scale_hex(factor: f32) -> [(f32, f32); 6] {
    [
        scale_pt(HEX_BASE[0], factor),
        scale_pt(HEX_BASE[1], factor),
        scale_pt(HEX_BASE[2], factor),
        scale_pt(HEX_BASE[3], factor),
        scale_pt(HEX_BASE[4], factor),
        scale_pt(HEX_BASE[5], factor),
    ]
}

fn point_in_polygon(x: f32, y: f32, poly: &[(f32, f32)]) -> bool {
    let mut inside = false;
    let Some(&(mut xj, mut yj)) = poly.last() else {
        return false;
    };
    for &(xi, yi) in poly {
        let intersects = ((yi > y) != (yj > y)) && (x < (xj - xi) * (y - yi) / (yj - yi) + xi);
        if intersects {
            inside = !inside;
        }
        xj = xi;
        yj = yi;
    }
    inside
}

/// Rasterizes the 64x64 Facet R emblem into straight-alpha RGBA8 pixels using 4x4 supersampling.
pub fn render_ruthenium_rgba(size: u32) -> Vec<u8> {
    if size == 0 {
        return Vec::new();
    }

    let hex_outer_stroke = scale_hex(1.0292);
    let hex_inner = scale_hex(0.9708);
    let scale = 64.0 / size as f32;
    let mut rgba = Vec::with_capacity((size as usize) * (size as usize) * 4);

    for py in 0..size {
        for px in 0..size {
            let mut sum_r: u32 = 0;
            let mut sum_g: u32 = 0;
            let mut sum_b: u32 = 0;
            let mut covered: u32 = 0;

            for sub_y in 0..4 {
                let sy = (py as f32 + (sub_y as f32 + 0.5) / 4.0) * scale;
                for sub_x in 0..4 {
                    let sx = (px as f32 + (sub_x as f32 + 0.5) / 4.0) * scale;
                    if !point_in_polygon(sx, sy, &hex_outer_stroke) {
                        continue;
                    }

                    let mut rgb = COLOR_STROKE;
                    if point_in_polygon(sx, sy, &hex_inner) {
                        rgb = if sx < 32.0 {
                            COLOR_LEFT_FACET
                        } else if point_in_polygon(sx, sy, &SHEEN_TRI) {
                            COLOR_SHEEN
                        } else {
                            COLOR_RIGHT_FACET
                        };
                    }

                    if point_in_polygon(sx, sy, &R_STEM) {
                        rgb = COLOR_R_STEM;
                    }
                    if point_in_polygon(sx, sy, &R_UPPER) {
                        rgb = COLOR_R_UPPER;
                    }
                    if point_in_polygon(sx, sy, &R_RETURN) {
                        rgb = COLOR_R_RETURN;
                    }
                    if point_in_polygon(sx, sy, &R_LEG) {
                        rgb = COLOR_R_LEG;
                    }

                    sum_r += u32::from(rgb[0]);
                    sum_g += u32::from(rgb[1]);
                    sum_b += u32::from(rgb[2]);
                    covered += 1;
                }
            }

            if let (Some(r), Some(g), Some(b)) = (
                sum_r.checked_div(covered),
                sum_g.checked_div(covered),
                sum_b.checked_div(covered),
            ) {
                rgba.push(r as u8);
                rgba.push(g as u8);
                rgba.push(b as u8);
                rgba.push(((covered * 255 + 8) / 16) as u8);
            } else {
                rgba.extend_from_slice(&[0, 0, 0, 0]);
            }
        }
    }

    rgba
}

/// Builds the 32x32 native window titlebar icon for Ruthenium.
pub fn window_icon() -> Option<Icon> {
    Icon::from_rgba(render_ruthenium_rgba(32), 32, 32).ok()
}

/// Builds the 128x128 high-DPI taskbar icon for Ruthenium on Windows.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub fn taskbar_icon() -> Option<Icon> {
    Icon::from_rgba(render_ruthenium_rgba(128), 128, 128).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_rgba_buffer_with_transparent_corners_and_solid_center() {
        let rgba = render_ruthenium_rgba(64);
        assert_eq!(rgba.len(), 64 * 64 * 4);
        assert_eq!(rgba[3], 0);

        let center_idx = (32 * 64 + 32) * 4;
        assert_eq!(rgba[center_idx + 3], 255);
    }

    #[test]
    fn builds_window_and_taskbar_icons() {
        assert!(window_icon().is_some());
        assert!(taskbar_icon().is_some());
    }
}
