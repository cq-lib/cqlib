// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

use super::VisualizationError;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
struct RgbImage {
    width: u32,
    height: u32,
    data: Vec<u8>,
}

#[derive(Debug)]
struct VisualCasePaths {
    actual_svg: PathBuf,
    actual_png: PathBuf,
    reference_png: PathBuf,
    diff_png: PathBuf,
}

fn visual_threshold() -> f64 {
    env::var("CQLIB_VISUAL_THRESHOLD")
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.995)
}

fn ensure_dir(path: &Path) {
    fs::create_dir_all(path).expect("failed to create test directory");
}

fn visual_case_paths(output_dir: &[&str], filename: &str) -> VisualCasePaths {
    let mut visual_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("visualization");
    for component in output_dir {
        visual_root = visual_root.join(component);
    }

    let references_dir = visual_root.join("references");
    let diffs_dir = visual_root.join("diffs");
    ensure_dir(&visual_root);
    ensure_dir(&references_dir);
    ensure_dir(&diffs_dir);

    VisualCasePaths {
        actual_svg: visual_root.join(filename.replace(".png", ".svg")),
        actual_png: visual_root.join(filename),
        reference_png: references_dir.join(filename),
        diff_png: diffs_dir.join(format!("diff_{filename}")),
    }
}

fn load_png_rgb(path: &Path) -> RgbImage {
    let pixmap = resvg::tiny_skia::Pixmap::load_png(path)
        .unwrap_or_else(|e| panic!("failed to load png `{}`: {e}", path.display()));
    let width = pixmap.width();
    let height = pixmap.height();
    let src = pixmap.data();
    let mut data = vec![255u8; (width as usize) * (height as usize) * 3];

    for idx in 0..(width as usize * height as usize) {
        let s = idx * 4;
        let d = idx * 3;
        let r = u32::from(src[s]);
        let g = u32::from(src[s + 1]);
        let b = u32::from(src[s + 2]);
        let a = u32::from(src[s + 3]);

        // tiny-skia stores premultiplied rgba, so composite over white here.
        let out_r = (r + ((255 * (255 - a) + 127) / 255)).min(255);
        let out_g = (g + ((255 * (255 - a) + 127) / 255)).min(255);
        let out_b = (b + ((255 * (255 - a) + 127) / 255)).min(255);

        data[d] = out_r as u8;
        data[d + 1] = out_g as u8;
        data[d + 2] = out_b as u8;
    }

    RgbImage {
        width,
        height,
        data,
    }
}

fn pad_rgb_to_canvas(img: &RgbImage, width: u32, height: u32) -> Vec<u8> {
    let mut out = vec![255u8; (width as usize) * (height as usize) * 3];
    for y in 0..img.height {
        let src_offset = (y as usize) * (img.width as usize) * 3;
        let dst_offset = (y as usize) * (width as usize) * 3;
        let row_bytes = (img.width as usize) * 3;
        out[dst_offset..dst_offset + row_bytes]
            .copy_from_slice(&img.data[src_offset..src_offset + row_bytes]);
    }
    out
}

fn similarity_ratio(a: &[u8], b: &[u8]) -> f64 {
    if a.len() != b.len() {
        return 0.0;
    }
    let mse = a
        .iter()
        .zip(b.iter())
        .map(|(x, y)| {
            let d = f64::from(*x) - f64::from(*y);
            d * d
        })
        .sum::<f64>()
        / (a.len() as f64);
    if mse <= 1e-12 {
        return 1.0;
    }
    (1.0 - mse / (255.0 * 255.0)).max(0.0)
}

fn save_diff_png(
    a: &[u8],
    b: &[u8],
    width: u32,
    height: u32,
    output_path: &Path,
) -> Result<(), String> {
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| "failed to allocate diff pixmap".to_string())?;
    let dst = pixmap.data_mut();
    const AMP: u16 = 4;

    for idx in 0..(width as usize * height as usize) {
        let i3 = idx * 3;
        let i4 = idx * 4;
        let dr = (i16::from(a[i3]) - i16::from(b[i3])).unsigned_abs();
        let dg = (i16::from(a[i3 + 1]) - i16::from(b[i3 + 1])).unsigned_abs();
        let db = (i16::from(a[i3 + 2]) - i16::from(b[i3 + 2])).unsigned_abs();
        dst[i4] = (dr.saturating_mul(AMP).min(255)) as u8;
        dst[i4 + 1] = (dg.saturating_mul(AMP).min(255)) as u8;
        dst[i4 + 2] = (db.saturating_mul(AMP).min(255)) as u8;
        dst[i4 + 3] = 255;
    }

    pixmap
        .save_png(output_path)
        .map_err(|e| format!("failed to save diff png `{}`: {e}", output_path.display()))
}

fn save_diff_and_similarity(actual_png: &Path, reference_png: &Path, diff_png: &Path) -> f64 {
    let actual = load_png_rgb(actual_png);
    if !reference_png.exists() {
        fs::copy(actual_png, reference_png).unwrap_or_else(|e| {
            panic!(
                "failed to bootstrap reference `{}` from `{}`: {e}",
                reference_png.display(),
                actual_png.display()
            )
        });
        return 1.0;
    }

    let reference = load_png_rgb(reference_png);
    let width = actual.width.max(reference.width);
    let height = actual.height.max(reference.height);
    let actual_padded = pad_rgb_to_canvas(&actual, width, height);
    let reference_padded = pad_rgb_to_canvas(&reference, width, height);
    let ratio = similarity_ratio(&actual_padded, &reference_padded);
    save_diff_png(&actual_padded, &reference_padded, width, height, diff_png)
        .expect("failed to write diff png");
    ratio
}

pub(crate) fn assert_svg_visual_match<F>(output_dir: &[&str], filename: &str, mut render: F)
where
    F: FnMut(&Path) -> Result<(), VisualizationError>,
{
    let paths = visual_case_paths(output_dir, filename);

    render(&paths.actual_svg).expect("failed to render svg");
    render(&paths.actual_png).expect("failed to render png");

    let ratio = save_diff_and_similarity(&paths.actual_png, &paths.reference_png, &paths.diff_png);
    let threshold = visual_threshold();
    assert!(
        ratio >= threshold,
        "Similarity ratio {ratio:.4} < {threshold:.4} for {filename}"
    );
}
