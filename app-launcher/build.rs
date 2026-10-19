use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));

    let wallpaper_jpg_path = manifest_dir.join("../../assets/image/wallpaper_milkyway.jpg");
    let hello_png_path = manifest_dir.join("../../assets/app-icons/hello.png");

    println!("cargo:rerun-if-changed={}", wallpaper_jpg_path.display());
    println!("cargo:rerun-if-changed={}", hello_png_path.display());

    // 1. Process Wallpaper (2560x1706 JPG -> 480x430 RGB565 Raw Binary)
    process_wallpaper(&wallpaper_jpg_path, &out_dir.join("wallpaper_milkyway.bin"));

    // 2. Process Hello App Icon (512x512 PNG -> 118x118 RGB565 + Alpha Mask + Rust code)
    process_hello_icon(&hello_png_path, &out_dir);
}

/// Processes the wallpaper JPEG:
/// - Aspect ratio crop to 480:430
/// - High-quality Lanczos3 resize to 480x430
/// - Floyd-Steinberg error diffusion dithering to RGB565
/// - Writes raw binary (480 * 430 * 2 = 412,800 bytes)
fn process_wallpaper(input_path: &Path, output_bin: &Path) {
    let img = image::open(input_path).unwrap_or_else(|e| {
        panic!("Failed to open wallpaper image at {}: {}", input_path.display(), e)
    });

    let (orig_w, orig_h) = (img.width(), img.height());
    let target_w = 480u32;
    let target_h = 430u32;

    let target_ratio = target_w as f64 / target_h as f64;
    let orig_ratio = orig_w as f64 / orig_h as f64;

    let cropped = if orig_ratio > target_ratio {
        let new_w = (orig_h as f64 * target_ratio).round() as u32;
        let left = (orig_w - new_w) / 2;
        img.crop_imm(left, 0, new_w, orig_h)
    } else {
        let new_h = (orig_w as f64 / target_ratio).round() as u32;
        let top = (orig_h - new_h) / 2;
        img.crop_imm(0, top, orig_w, new_h)
    };

    let resized = cropped.resize_exact(target_w, target_h, image::imageops::FilterType::Lanczos3);
    let rgb_img = resized.to_rgb8();

    let (w, h) = (target_w as usize, target_h as usize);
    let mut r_buf = vec![0.0f32; w * h];
    let mut g_buf = vec![0.0f32; w * h];
    let mut b_buf = vec![0.0f32; w * h];

    for y in 0..h {
        for x in 0..w {
            let p = rgb_img.get_pixel(x as u32, y as u32);
            let idx = y * w + x;
            r_buf[idx] = p[0] as f32;
            g_buf[idx] = p[1] as f32;
            b_buf[idx] = p[2] as f32;
        }
    }

    let mut raw_bytes = Vec::with_capacity(w * h * 2);

    for y in 0..h {
        for x in 0..w {
            let idx = y * w + x;
            let r_val = r_buf[idx].clamp(0.0, 255.0);
            let g_val = g_buf[idx].clamp(0.0, 255.0);
            let b_val = b_buf[idx].clamp(0.0, 255.0);

            let r5 = ((r_val * 31.0 / 255.0).round() as u32).min(31) as u16;
            let g6 = ((g_val * 63.0 / 255.0).round() as u32).min(63) as u16;
            let b5 = ((b_val * 31.0 / 255.0).round() as u32).min(31) as u16;

            let p565 = (r5 << 11) | (g6 << 5) | b5;
            raw_bytes.extend_from_slice(&p565.to_le_bytes());

            let r_quant = r5 as f32 * 255.0 / 31.0;
            let g_quant = g6 as f32 * 255.0 / 63.0;
            let b_quant = b5 as f32 * 255.0 / 31.0;

            let err_r = r_val - r_quant;
            let err_g = g_val - g_quant;
            let err_b = b_val - b_quant;

            // Floyd-Steinberg error diffusion
            if x + 1 < w {
                r_buf[y * w + (x + 1)] += err_r * (7.0 / 16.0);
                g_buf[y * w + (x + 1)] += err_g * (7.0 / 16.0);
                b_buf[y * w + (x + 1)] += err_b * (7.0 / 16.0);
            }
            if y + 1 < h {
                if x > 0 {
                    r_buf[(y + 1) * w + (x - 1)] += err_r * (3.0 / 16.0);
                    g_buf[(y + 1) * w + (x - 1)] += err_g * (3.0 / 16.0);
                    b_buf[(y + 1) * w + (x - 1)] += err_b * (3.0 / 16.0);
                }
                r_buf[(y + 1) * w + x] += err_r * (5.0 / 16.0);
                g_buf[(y + 1) * w + x] += err_g * (5.0 / 16.0);
                b_buf[(y + 1) * w + x] += err_b * (5.0 / 16.0);
                if x + 1 < w {
                    r_buf[(y + 1) * w + (x + 1)] += err_r * (1.0 / 16.0);
                    g_buf[(y + 1) * w + (x + 1)] += err_g * (1.0 / 16.0);
                    b_buf[(y + 1) * w + (x + 1)] += err_b * (1.0 / 16.0);
                }
            }
        }
    }

    std::fs::write(output_bin, raw_bytes).expect("write wallpaper binary");
}

/// Generates a squircle alpha mask for an icon of size x size pixels,
/// using 4x supersampling (16 samples per pixel) for GPU-quality antialiasing.
fn generate_squircle_mask(size: usize, radius_ratio: f64) -> Vec<u8> {
    let supersample = 4usize;
    let samples = (supersample * supersample) as f64;
    let mut mask = vec![0u8; size * size];
    let half = size as f64 * 0.5;
    let corner_r = radius_ratio * size as f64;
    let inner = half - corner_r;

    for y in 0..size {
        for x in 0..size {
            let mut coverage = 0.0f64;
            for sy in 0..supersample {
                let sub_y = y as f64 + (sy as f64 + 0.5) / supersample as f64;
                let dy = (sub_y - half).abs();
                let qy = (dy - inner).max(0.0);

                for sx in 0..supersample {
                    let sub_x = x as f64 + (sx as f64 + 0.5) / supersample as f64;
                    let dx = (sub_x - half).abs();
                    let qx = (dx - inner).max(0.0);
                    let dist = (qx * qx + qy * qy).sqrt();

                    if dist <= corner_r - 1.0 {
                        coverage += 1.0;
                    } else if dist < corner_r {
                        coverage += corner_r - dist;
                    }
                }
            }
            let alpha = ((coverage / samples) * 255.0 + 0.5).clamp(0.0, 255.0) as u8;
            mask[y * size + x] = alpha;
        }
    }
    mask
}

/// Processes the Hello icon PNG:
/// - Square crop
/// - Resize to 118x118 with Lanczos3
/// - Apply iOS squircle antialiased rounded corner mask
/// - Floyd-Steinberg error diffusion dithering to RGB565
/// - Writes hello_rgb565.bin, hello_alpha.bin, and hello_icon.rs
fn process_hello_icon(input_path: &Path, out_dir: &Path) {
    let img = image::open(input_path).unwrap_or_else(|e| {
        panic!("Failed to open hello icon at {}: {}", input_path.display(), e)
    });

    let (orig_w, orig_h) = (img.width(), img.height());
    let min_dim = orig_w.min(orig_h);
    let left = (orig_w - min_dim) / 2;
    let top = (orig_h - min_dim) / 2;
    let cropped = img.crop_imm(left, top, min_dim, min_dim);

    let target_size = 118u32;
    let resized = cropped.resize_exact(target_size, target_size, image::imageops::FilterType::Lanczos3);
    let rgba_img = resized.to_rgba8();

    let size = target_size as usize;
    let squircle_mask = generate_squircle_mask(size, 0.225);

    let mut r_buf = vec![0.0f32; size * size];
    let mut g_buf = vec![0.0f32; size * size];
    let mut b_buf = vec![0.0f32; size * size];
    let mut a_buf = vec![0u8; size * size];

    for y in 0..size {
        for x in 0..size {
            let p = rgba_img.get_pixel(x as u32, y as u32);
            let idx = y * size + x;
            r_buf[idx] = p[0] as f32;
            g_buf[idx] = p[1] as f32;
            b_buf[idx] = p[2] as f32;

            let sq_a = squircle_mask[idx] as u32;
            let orig_a = p[3] as u32;
            a_buf[idx] = ((orig_a * sq_a) / 255) as u8;
        }
    }

    let mut rgb565_values = Vec::with_capacity(size * size);
    let mut rgb565_bytes = Vec::with_capacity(size * size * 2);

    for y in 0..size {
        for x in 0..size {
            let idx = y * size + x;
            let r_val = r_buf[idx].clamp(0.0, 255.0);
            let g_val = g_buf[idx].clamp(0.0, 255.0);
            let b_val = b_buf[idx].clamp(0.0, 255.0);

            let r5 = ((r_val * 31.0 / 255.0).round() as u32).min(31) as u16;
            let g6 = ((g_val * 63.0 / 255.0).round() as u32).min(63) as u16;
            let b5 = ((b_val * 31.0 / 255.0).round() as u32).min(31) as u16;

            let p565 = (r5 << 11) | (g6 << 5) | b5;
            rgb565_values.push(p565);
            rgb565_bytes.extend_from_slice(&p565.to_le_bytes());

            let r_quant = r5 as f32 * 255.0 / 31.0;
            let g_quant = g6 as f32 * 255.0 / 63.0;
            let b_quant = b5 as f32 * 255.0 / 31.0;

            let err_r = r_val - r_quant;
            let err_g = g_val - g_quant;
            let err_b = b_val - b_quant;

            if x + 1 < size {
                r_buf[y * size + (x + 1)] += err_r * (7.0 / 16.0);
                g_buf[y * size + (x + 1)] += err_g * (7.0 / 16.0);
                b_buf[y * size + (x + 1)] += err_b * (7.0 / 16.0);
            }
            if y + 1 < size {
                if x > 0 {
                    r_buf[(y + 1) * size + (x - 1)] += err_r * (3.0 / 16.0);
                    g_buf[(y + 1) * size + (x - 1)] += err_g * (3.0 / 16.0);
                    b_buf[(y + 1) * size + (x - 1)] += err_b * (3.0 / 16.0);
                }
                r_buf[(y + 1) * size + x] += err_r * (5.0 / 16.0);
                g_buf[(y + 1) * size + x] += err_g * (5.0 / 16.0);
                b_buf[(y + 1) * size + x] += err_b * (5.0 / 16.0);
                if x + 1 < size {
                    r_buf[(y + 1) * size + (x + 1)] += err_r * (1.0 / 16.0);
                    g_buf[(y + 1) * size + (x + 1)] += err_g * (1.0 / 16.0);
                    b_buf[(y + 1) * size + (x + 1)] += err_b * (1.0 / 16.0);
                }
            }
        }
    }

    // Write binary files
    std::fs::write(out_dir.join("hello_rgb565.bin"), &rgb565_bytes).expect("write hello_rgb565.bin");
    std::fs::write(out_dir.join("hello_alpha.bin"), &a_buf).expect("write hello_alpha.bin");

    // Write hello_icon.rs for zero-cost const AppIcon::bitmap(...) in CATALOGUE
    let rs_path = out_dir.join("hello_icon.rs");
    let mut f = BufWriter::new(File::create(rs_path).expect("create hello_icon.rs"));
    writeln!(f, "pub static HELLO_RGB565: [u16; {}] = {:?};", rgb565_values.len(), rgb565_values).unwrap();
    writeln!(f, "pub static HELLO_ALPHA: [u8; {}] = {:?};", a_buf.len(), a_buf).unwrap();
}
