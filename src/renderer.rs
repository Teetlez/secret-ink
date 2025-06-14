use std::collections::HashMap;

use crate::{config::Config, layout::GlyphInstance};
use ab_glyph::{Font, FontRef, Glyph, ScaleFont};
use image::{GrayImage, Luma, Rgba, RgbaImage};
use imageproc::filter::gaussian_blur_f32;

/// Renders a page by stamping each glyph onto the paper canvas,
/// applying bleed blur, jitter, and ink blending with PBR textures.
pub fn render_page(
    fonts: &HashMap<String, FontRef>,
    glyphs: &[GlyphInstance],
    paper: &RgbaImage,
    normal: &RgbaImage,
    roughness: &GrayImage,
    cfg: &Config,
) -> RgbaImage {
    let mut canvas = paper.clone();

    for inst in glyphs {
        let font = &fonts[&inst.font_key];

        // 1) Rasterize: get mask and bounding-box offsets
        let (mask, off_x, off_y) = rasterize_glyph(&inst.glyph, font);

        // 2) Bleed blur
        let blurred = gaussian_blur_f32(&mask, cfg.blur_sigma);

        // 3) Jitter
        let dy = (fastrand::f32() - 0.5) * cfg.jitter_px;

        // 4) Compute top-left corner in canvas coords:
        // inst.glyph.position is baseline origin; px_bounds.min shifts to top-left of actual pixels
        let x0 = (inst.glyph.position.x as i32).max(0) as u32;
        let y0 = (inst.glyph.position.y as i32 + off_y + dy as i32).max(0) as u32;

        // 5) Blend ink into canvas
        blend_ink(&mut canvas, &blurred, normal, roughness, x0, y0, cfg);
    }
    draw_margins(&mut canvas, &cfg);
    canvas
}

/// Rasterize a glyph into a grayscale mask and return
/// the mask plus its pixel-bound offsets (min.x, min.y).
fn rasterize_glyph(glyph: &Glyph, font: &FontRef) -> (GrayImage, i32, i32) {
    let outlined = match font.outline_glyph(glyph.clone()) {
        Some(o) => o,
        None => return (GrayImage::new(0, 0), 0, 0),
    };
    let bounds = outlined.px_bounds();

    // Allocate mask of exactly the outline bounds
    let width = bounds.width().ceil() as u32;
    let height = bounds.height().ceil() as u32;
    let mut mask = GrayImage::new(width, height);

    // Draw into mask: coverage -> alpha
    if let Some(outline) = font.outline_glyph(glyph.clone()) {
        outline.draw(|x, y, c| {
            let pixel = mask.get_pixel_mut(x, y);
            *pixel = Luma([(c * 255.0 as f32).round() as u8]);
        });
    }

    (
        mask,
        bounds.min.x.floor() as i32,
        bounds.min.y.floor() as i32,
    )
}
///
/// Blend ink mask into the RGBA canvas using paper normal & roughness
fn blend_ink(
    canvas: &mut RgbaImage,
    mask: &GrayImage,
    normal: &RgbaImage,
    rough: &GrayImage,
    x0: u32,
    y0: u32,
    cfg: &Config,
) {
    let (w, h) = (mask.width(), mask.height());

    for x in 0..w {
        for y in 0..h {
            let alpha = mask.get_pixel(x, y)[0] as f32 / 255.0;
            if alpha == 0.0 {
                continue;
            }

            let cx = x0 + x;
            let cy = y0 + y;
            if cx >= canvas.width() || cy >= canvas.height() {
                continue;
            }

            // Sample normal Z component and roughness
            let nz = normal.get_pixel(cx, cy)[2] as f32 / 255.0;
            let roughness = rough.get_pixel(cx, cy)[0] as f32 / 255.0;

            // Modulate ink alpha by light & roughness
            let light_mod = 1.0 + 0.2 * (1.0 - nz);
            let ink_a = cfg.ink_opacity * alpha * light_mod * roughness;

            // Multiply blend: darken canvas
            let dst = canvas.get_pixel_mut(cx, cy);
            for i in 0..3 {
                dst[i] = ((dst[i] as f32) * (1.0 - ink_a)).round() as u8;
            }
        }
    }
}

fn draw_margins(canvas: &mut RgbaImage, cfg: &Config) {
    let left = cfg.margin_left;
    let right = cfg.page_width - cfg.margin_right;
    let top = cfg.margin_top;
    let bottom = cfg.page_height - cfg.margin_bottom;
    let center_h = cfg.page_height / 2;
    let center_v = cfg.page_width / 2;
    let dim = canvas.dimensions();
    for i in 0..dim.0 {
        let px = canvas.get_pixel_mut(i, top);
        px[0] = 0;
        let px = canvas.get_pixel_mut(i, center_h);
        px[1] = 0;
        let px = canvas.get_pixel_mut(i, bottom);
        px[2] = 0;
    }
    for j in 0..dim.1 {
        let px = canvas.get_pixel_mut(left, j);
        px[0] = 0;
        let px = canvas.get_pixel_mut(center_v, j);
        px[1] = 0;
        let px = canvas.get_pixel_mut(right, j);
        px[2] = 0;
    }
}
