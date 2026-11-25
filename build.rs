use std::env;
use std::fs;
use std::path::PathBuf;

const RT_ICON: u16 = 3;
const RT_GROUP_ICON: u16 = 14;
const RT_VERSION: u16 = 16;
const LANG_EN_US: u16 = 0x0409;
const ICON_SIZES: [u32; 5] = [16, 32, 48, 64, 128];

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

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR must be set"));
    let res_path = out_dir.join("ruthenium.res");
    let res_bytes = build_win32_res_file();
    fs::write(&res_path, res_bytes).expect("failed to write Windows .res file");

    println!("cargo:rustc-link-arg-bins={}", res_path.display());
}

fn build_win32_res_file() -> Vec<u8> {
    let mut out = Vec::with_capacity(96 * 1024);

    // Required empty 32-byte Win32 .res file header.
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&32u32.to_le_bytes());
    out.extend_from_slice(&0xFFFFu16.to_le_bytes());
    out.extend_from_slice(&0x0000u16.to_le_bytes());
    out.extend_from_slice(&0xFFFFu16.to_le_bytes());
    out.extend_from_slice(&0x0000u16.to_le_bytes());
    out.extend_from_slice(&[0u8; 16]);

    let mut group_dir = Vec::with_capacity(6 + ICON_SIZES.len() * 14);
    group_dir.extend_from_slice(&0u16.to_le_bytes());
    group_dir.extend_from_slice(&1u16.to_le_bytes());
    group_dir.extend_from_slice(&(ICON_SIZES.len() as u16).to_le_bytes());

    for (idx, &size) in ICON_SIZES.iter().enumerate() {
        let icon_id = (idx + 1) as u16;
        let dib = build_icon_dib(size);
        push_res_entry(&mut out, RT_ICON, icon_id, LANG_EN_US, 0x1010, &dib);

        let dim = if size >= 256 { 0 } else { size as u8 };
        group_dir.push(dim);
        group_dir.push(dim);
        group_dir.push(0);
        group_dir.push(0);
        group_dir.extend_from_slice(&1u16.to_le_bytes());
        group_dir.extend_from_slice(&32u16.to_le_bytes());
        group_dir.extend_from_slice(&(dib.len() as u32).to_le_bytes());
        group_dir.extend_from_slice(&icon_id.to_le_bytes());
    }

    push_res_entry(&mut out, RT_GROUP_ICON, 1, LANG_EN_US, 0x1030, &group_dir);

    let version_info = build_version_info();
    push_res_entry(&mut out, RT_VERSION, 1, LANG_EN_US, 0x0030, &version_info);

    out
}

fn push_res_entry(
    out: &mut Vec<u8>,
    res_type: u16,
    res_id: u16,
    lang: u16,
    mem_flags: u16,
    data: &[u8],
) {
    pad_to_4(out);
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(&32u32.to_le_bytes());
    out.extend_from_slice(&0xFFFFu16.to_le_bytes());
    out.extend_from_slice(&res_type.to_le_bytes());
    out.extend_from_slice(&0xFFFFu16.to_le_bytes());
    out.extend_from_slice(&res_id.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&mem_flags.to_le_bytes());
    out.extend_from_slice(&lang.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(data);
    pad_to_4(out);
}

fn pad_to_4(buf: &mut Vec<u8>) {
    while buf.len() % 4 != 0 {
        buf.push(0);
    }
}

fn build_icon_dib(size: u32) -> Vec<u8> {
    let rgba = render_ruthenium_rgba(size);
    let and_stride = ((size as usize).div_ceil(32)) * 4;
    let xor_size = (size as usize) * (size as usize) * 4;
    let and_size = and_stride * (size as usize);

    let mut dib = Vec::with_capacity(40 + xor_size + and_size);
    dib.extend_from_slice(&40u32.to_le_bytes());
    dib.extend_from_slice(&(size as i32).to_le_bytes());
    dib.extend_from_slice(&((size as i32) * 2).to_le_bytes());
    dib.extend_from_slice(&1u16.to_le_bytes());
    dib.extend_from_slice(&32u16.to_le_bytes());
    dib.extend_from_slice(&0u32.to_le_bytes());
    dib.extend_from_slice(&((xor_size + and_size) as u32).to_le_bytes());
    dib.extend_from_slice(&0i32.to_le_bytes());
    dib.extend_from_slice(&0i32.to_le_bytes());
    dib.extend_from_slice(&0u32.to_le_bytes());
    dib.extend_from_slice(&0u32.to_le_bytes());

    let stride = size as usize;
    for py in (0..stride).rev() {
        for px in 0..stride {
            let idx = (py * stride + px) * 4;
            dib.push(rgba[idx + 2]);
            dib.push(rgba[idx + 1]);
            dib.push(rgba[idx]);
            dib.push(rgba[idx + 3]);
        }
    }

    for py in (0..stride).rev() {
        let mut row = vec![0u8; and_stride];
        for px in 0..stride {
            let alpha = rgba[(py * stride + px) * 4 + 3];
            if alpha < 128 {
                row[px / 8] |= 1 << (7 - (px % 8));
            }
        }
        dib.extend_from_slice(&row);
    }

    dib
}

fn build_version_info() -> Vec<u8> {
    let entries = [
        ("CompanyName", "Ruthenium"),
        ("FileDescription", "Ruthenium"),
        ("FileVersion", "0.1.0.0"),
        ("InternalName", "Ruthenium"),
        ("OriginalFilename", "Ruthenium.exe"),
        ("ProductName", "Ruthenium"),
        ("ProductVersion", "0.1.0.0"),
    ];

    let string_nodes: Vec<Vec<u8>> = entries
        .iter()
        .map(|&(key, val)| {
            let utf16_val = encode_utf16_nul(val);
            let value_len_words = (utf16_val.len() / 2) as u16;
            build_version_node(value_len_words, 1, key, &utf16_val, &[])
        })
        .collect();

    let string_table = build_version_node(0, 1, "040904b0", &[], &string_nodes);
    let string_file_info = build_version_node(0, 1, "StringFileInfo", &[], &[string_table]);

    let translation_bytes = [0x09u8, 0x04, 0xB0, 0x04];
    let var_node = build_version_node(4, 0, "Translation", &translation_bytes, &[]);
    let var_file_info = build_version_node(0, 1, "VarFileInfo", &[], &[var_node]);

    let mut fixed = Vec::with_capacity(52);
    fixed.extend_from_slice(&0xFEEF_04BDu32.to_le_bytes());
    fixed.extend_from_slice(&0x0001_0000u32.to_le_bytes());
    fixed.extend_from_slice(&0x0000_0001u32.to_le_bytes());
    fixed.extend_from_slice(&0u32.to_le_bytes());
    fixed.extend_from_slice(&0x0000_0001u32.to_le_bytes());
    fixed.extend_from_slice(&0u32.to_le_bytes());
    fixed.extend_from_slice(&0x0000_003Fu32.to_le_bytes());
    fixed.extend_from_slice(&0u32.to_le_bytes());
    fixed.extend_from_slice(&0x0004_0004u32.to_le_bytes());
    fixed.extend_from_slice(&0x0000_0001u32.to_le_bytes());
    fixed.extend_from_slice(&0u32.to_le_bytes());
    fixed.extend_from_slice(&0u32.to_le_bytes());
    fixed.extend_from_slice(&0u32.to_le_bytes());

    build_version_node(52, 0, "VS_VERSION_INFO", &fixed, &[string_file_info, var_file_info])
}

fn build_version_node(
    w_value_len: u16,
    w_type: u16,
    key: &str,
    value: &[u8],
    children: &[Vec<u8>],
) -> Vec<u8> {
    let mut node = Vec::with_capacity(64 + value.len());
    node.extend_from_slice(&0u16.to_le_bytes());
    node.extend_from_slice(&w_value_len.to_le_bytes());
    node.extend_from_slice(&w_type.to_le_bytes());
    node.extend_from_slice(&encode_utf16_nul(key));
    pad_to_4(&mut node);

    node.extend_from_slice(value);
    if !children.is_empty() {
        pad_to_4(&mut node);
        for (idx, child) in children.iter().enumerate() {
            node.extend_from_slice(child);
            if idx + 1 < children.len() {
                pad_to_4(&mut node);
            }
        }
    }

    let total_len = (node.len() as u16).to_le_bytes();
    node[0] = total_len[0];
    node[1] = total_len[1];
    node
}

fn encode_utf16_nul(text: &str) -> Vec<u8> {
    let mut bytes = Vec::with_capacity((text.len() + 1) * 2);
    for unit in text.encode_utf16().chain(std::iter::once(0u16)) {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    bytes
}

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

fn render_ruthenium_rgba(size: u32) -> Vec<u8> {
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
