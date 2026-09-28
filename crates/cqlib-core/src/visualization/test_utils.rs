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
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

const FONT_SHA256: &str = "f8ace1f892b2bd9dc1792ba7f097fa7588f84fed48321480e04de5390828221f";
const FONT_URL: &str =
    "https://cdn.jsdelivr.net/npm/pdfjs-dist@5.4.624/standard_fonts/LiberationSans-Regular.ttf";

fn visual_font() -> &'static [u8] {
    // Share successful downloads and failures across parallel tests alike.
    static FONT: OnceLock<Result<Vec<u8>, String>> = OnceLock::new();
    FONT.get_or_init(prepare_visual_font)
        .as_ref()
        .unwrap_or_else(|error| panic!("visual-test font: {error}"))
}

fn prepare_visual_font() -> Result<Vec<u8>, String> {
    let target = env::var_os("CARGO_TARGET_DIR").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target"),
        PathBuf::from,
    );
    let override_path = env::var_os("CQLIB_VISUAL_FONT");
    let path = override_path.as_ref().map_or_else(
        || target.join("visual-fonts/LiberationSans-Regular.ttf"),
        PathBuf::from,
    );
    let valid = |data: &[u8]| format!("{:x}", Sha256::digest(data)) == FONT_SHA256;
    if let Ok(data) = fs::read(&path)
        && valid(&data)
    {
        return Ok(data);
    }
    if override_path.is_some() {
        return Err(format!(
            "{} is missing or has the wrong SHA-256",
            path.display()
        ));
    }
    let output = Command::new("curl")
        .args([
            "--fail",
            "--location",
            "--silent",
            "--show-error",
            "--proto",
            "=https",
            "--proto-redir",
            "=https",
            "--max-time",
            "45",
            "--retry",
            "1",
            FONT_URL,
        ])
        .output()
        .map_err(|error| {
            format!("cannot run curl: {error}; set CQLIB_VISUAL_FONT for offline tests")
        })?;
    if !output.status.success() || !valid(&output.stdout) {
        return Err(format!(
            "download failed or SHA-256 mismatch ({FONT_URL}): {}; \
             set CQLIB_VISUAL_FONT to a verified font for offline tests",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    fs::create_dir_all(path.parent().expect("font cache directory"))
        .map_err(|error| error.to_string())?;
    fs::write(&path, &output.stdout).map_err(|error| error.to_string())?;
    Ok(output.stdout)
}

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
    exported_png: PathBuf,
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
        exported_png: visual_root.join(filename.replace(".png", "_export.png")),
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

/// Render the production SVG at its intrinsic size with only our pinned font.
/// This module is test-only; production PNG export still uses system fonts.
fn rasterize_visual_reference(svg_path: &Path, png_path: &Path) {
    let mut options = resvg::usvg::Options {
        font_family: "Liberation Sans".to_string(),
        ..Default::default()
    };
    let fonts = options.fontdb_mut();
    fonts.load_font_data(visual_font().to_vec());
    fonts.set_sans_serif_family("Liberation Sans");
    fonts.set_serif_family("Liberation Sans");
    fonts.set_monospace_family("Liberation Sans");
    let svg = fs::read(svg_path).expect("failed to read generated SVG");
    let tree = resvg::usvg::Tree::from_data(&svg, &options)
        .expect("failed to parse generated SVG with pinned font");
    let size = tree.size().to_int_size();
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size.width(), size.height())
        .expect("failed to allocate visual-test pixmap");
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    pixmap
        .save_png(png_path)
        .expect("failed to save visual-test PNG");
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
    let reference = load_png_rgb(reference_png);
    let width = actual.width.max(reference.width);
    let height = actual.height.max(reference.height);
    let actual_padded = pad_rgb_to_canvas(&actual, width, height);
    let reference_padded = pad_rgb_to_canvas(&reference, width, height);
    let ratio = similarity_ratio(&actual_padded, &reference_padded);
    save_diff_png(&actual_padded, &reference_padded, width, height, diff_png)
        .expect("failed to write diff png");
    assert_eq!(
        (actual.width, actual.height),
        (reference.width, reference.height),
        "image dimensions changed: actual `{}`, reference `{}`, diff `{}`",
        actual_png.display(),
        reference_png.display(),
        diff_png.display()
    );
    ratio
}

pub(crate) fn assert_svg_visual_match<F>(output_dir: &[&str], filename: &str, mut render: F)
where
    F: FnMut(&Path) -> Result<(), VisualizationError>,
{
    let paths = visual_case_paths(output_dir, filename);

    render(&paths.actual_svg).expect("failed to render svg");
    // Exercise the real PNG exporter as a smoke check, but compare the SVG
    // through a hermetic rasterizer so OS font discovery cannot change goldens.
    render(&paths.exported_png).expect("failed to render png");
    let exported = load_png_rgb(&paths.exported_png);
    assert!(exported.width > 0 && exported.height > 0);
    rasterize_visual_reference(&paths.actual_svg, &paths.actual_png);

    if env::var("CQLIB_UPDATE_VISUAL_REFERENCES").as_deref() == Ok("1") {
        assert!(
            env::var_os("CI").is_none(),
            "visual references must be reviewed and updated locally, not in CI"
        );
        fs::copy(&paths.actual_png, &paths.reference_png)
            .expect("failed to update visual reference");
        return;
    }
    assert!(
        paths.reference_png.is_file(),
        "missing visual reference `{}`; actual image: `{}`. To explicitly regenerate \
         references locally, run CQLIB_UPDATE_VISUAL_REFERENCES=1 cargo test -p cqlib-core visualization::",
        paths.reference_png.display(),
        paths.actual_png.display()
    );
    let ratio = save_diff_and_similarity(&paths.actual_png, &paths.reference_png, &paths.diff_png);
    let threshold = visual_threshold();
    assert!(
        ratio >= threshold,
        "Similarity ratio {ratio:.6} < {threshold:.6} for {filename}; \
         actual: {}; reference: {}; diff: {}; SVG: {}; font: prepared Liberation Sans",
        paths.actual_png.display(),
        paths.reference_png.display(),
        paths.diff_png.display(),
        paths.actual_svg.display()
    );
}
