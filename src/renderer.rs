use std::collections::HashMap;

use crate::{
    config::Config,
    layout::{GlyphInstance, Redaction},
};
use ab_glyph::{Font, FontRef, Glyph, ScaleFont};
use image::{GrayImage, Luma, Rgba, RgbaImage};
use imageproc::filter::gaussian_blur_f32;

/// Renders a page by stamping each glyph onto the paper canvas,
/// applying bleed blur, jitter, and ink blending with PBR textures.
pub fn render_page(
    fonts: &HashMap<String, FontRef>,
    glyphs: &[GlyphInstance],
    redactions: &[Redaction],
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
    for redaction in redactions {
        // 1) rasterize\
        let margin = 5;
        let (width, height) = (
            (redaction.end.unwrap_or_default().x - redaction.start.x).ceil() as u32 + (margin * 2),
            (redaction.thickness * 1.2).ceil() as u32 + (margin * 2),
        );
        let mut r_box = GrayImage::new(width, height);
        for x in margin..(width - margin) {
            for y in margin..(height - margin) {
                if let Some(p) = r_box.get_pixel_mut_checked(x, y) {
                    *p = Luma([(200.0 + (fastrand::f32() * 1000.0) - 100.0)
                        .clamp(0.0, 255.0)
                        .round() as u8])
                }
            }
        }

        // 2) bleed blur
        let blurred = gaussian_blur_f32(&r_box, cfg.blur_sigma * 1.5 + (fastrand::f32() * 0.5));

        // 3) jitter offset (rotate?)
        let (dx, dy) = (
            (fastrand::f32() - 0.5) * cfg.jitter_px,
            (fastrand::f32() - 0.5) * cfg.jitter_px,
        );
        let x0 = (redaction.start.x + dx).ceil() as u32 - margin;
        let y0 =
            ((redaction.start.y * 2.0) + dy).ceil() as u32 - (redaction.thickness as u32 + margin);

        // 4) blend over document
        blend_ink(&mut canvas, &blurred, normal, roughness, x0, y0, cfg);
        blend_ink(&mut canvas, &blurred, normal, roughness, x0, y0, cfg);
        blend_ink(&mut canvas, &blurred, normal, roughness, x0, y0, cfg);
        blend_ink(&mut canvas, &blurred, normal, roughness, x0, y0, cfg);
    }
    // redact(redactions, &mut canvas, cfg, normal, roughness);
    // draw_margins(&mut canvas, &cfg);
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
            *pixel = Luma([((c + ((fastrand::f32() * 0.2) - 0.1)) * 255.0 as f32).round() as u8]);
        });
    }

    (
        mask,
        bounds.min.x.floor() as i32,
        bounds.min.y.floor() as i32,
    )
}

fn redact(
    redactions: &[Redaction],
    canvas: &mut RgbaImage,
    cfg: &Config,
    normal: &RgbaImage,
    rough: &GrayImage,
) {
    for red in redactions {
        let (w, h) = (
            (red.end.unwrap_or_default().x - red.start.x).ceil() as u32 + 1,
            (red.thickness * 1.2) as u32,
        );
        for x in 0..w {
            for y in 0..h {
                let cx = red.start.x.ceil() as u32 + x;
                let cy = (red.start.y * 2.0).ceil() as u32 + y - red.thickness as u32;

                if cx >= canvas.width() || cy >= canvas.height() {
                    continue;
                }

                let nz = normal.get_pixel(cx, cy)[2] as f32 / 255.0;
                let roughness = rough.get_pixel(cx, cy)[0] as f32 / 255.0;

                // Modulate ink alpha by light & roughness
                let light_mod = 1.0 + 0.2 * (1.0 - nz);
                let ink_a = (1.0 - cfg.ink_opacity) * light_mod * roughness * fastrand::f32();

                // Multiply blend: darken canvas
                let dst = canvas.get_pixel_mut(cx, cy);
                for i in 0..3 {
                    dst[i] = ((dst[i] as f32) * ink_a).round() as u8;
                }
            }
        }
    }
}

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
