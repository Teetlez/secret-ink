use std::collections::HashMap;

use crate::{config::Config, layout::GlyphInstance};
use ab_glyph::{Font, FontRef, Glyph};
use image::{GrayImage, Luma, Rgba, RgbaImage};
use imageproc::filter::gaussian_blur_f32;

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
        // rasterize to alpha mask
        let mask = rasterize_glyph(&inst.glyph, &fonts[&inst.font_key]);

        // blur for bleed
        let blurred = gaussian_blur_f32(&mask, cfg.blur_sigma);

        // jitter
        let dy = (fastrand::f32() - 0.5) * cfg.jitter_px;
        let moved = translate_mask(&blurred, 0.0, dy);

        // blend onto canvas
        blend_ink(
            &mut canvas,
            &moved,
            &normal,
            &roughness,
            inst.x as u32,
            (inst.y + dy) as u32,
            cfg,
        );
    }

    canvas
}

// stubs—fill in with actual APIs:
fn rasterize_glyph(glyph: &Glyph, font: &impl Font) -> GrayImage {
    let bounding_box = match font.outline_glyph(glyph.clone()) {
        Some(og) => og.px_bounds(),
        None => return GrayImage::new(0, 0),
    };

    let width = bounding_box.width().ceil() as u32;
    let height = bounding_box.height().ceil() as u32;

    let mut image = GrayImage::new(width, height);

    if let Some(outline) = font.outline_glyph(glyph.clone()) {
        outline.draw(|x, y, c| {
            let pixel = image.get_pixel_mut(x, y);
            *pixel = Luma([(c * 255.0).round() as u8]);
        });
    }

    image
}

fn translate_mask(mask: &GrayImage, dx: f32, dy: f32) -> GrayImage {
    let width = mask.width();
    let height = mask.height();
    let mut result = GrayImage::new(width, height);

    for y in 0..height {
        for x in 0..width {
            let xf = x as f32 - dx;
            let yf = y as f32 - dy;
            let xi = xf.floor() as i32;
            let yi = yf.floor() as i32;

            if xi >= 0 && yi >= 0 && (xi as u32) < width && (yi as u32) < height {
                let src = mask.get_pixel(xi as u32, yi as u32)[0];
                result.put_pixel(x, y, Luma([src]));
            }
        }
    }

    result
}

fn blend_ink(
    canvas: &mut RgbaImage,
    mask: &GrayImage,
    normal: &RgbaImage,
    rough: &GrayImage,
    x0: u32,
    y0: u32,
    cfg: &Config,
) {
    let width = mask.width();
    let height = mask.height();

    for y in 0..height {
        for x in 0..width {
            let ink_mask_val = mask.get_pixel(x, y)[0];
            if ink_mask_val == 0 {
                continue;
            }

            let cx = x + x0;
            let cy = y + y0;
            if cx >= canvas.width() || cy >= canvas.height() {
                continue;
            }

            let normal_pixel = normal.get_pixel(cx, cy);
            let normal_z = normal_pixel[2] as f32 / 255.0; // Z-axis of the normal

            // Light hits more directly with high Z (perpendicular)
            let light_factor = normal_z; // 0 (flat) → 1 (facing camera)

            // Basic alpha modulation with light and ink pressure
            let base_alpha = (ink_mask_val as f32) / 255.0;
            let light_mod = 1.0 + 0.2 * (1.0 - light_factor);
            let ink_alpha = cfg.ink_opacity * base_alpha * light_mod;

            let dst = canvas.get_pixel_mut(cx, cy);
            let Rgba([r, g, b, a]) = *dst;

            // Blend ink (black ink assumed)
            let blended_r = (r as f32 * (1.0 - ink_alpha)).round() as u8;
            let blended_g = (g as f32 * (1.0 - ink_alpha)).round() as u8;
            let blended_b = (b as f32 * (1.0 - ink_alpha)).round() as u8;

            *dst = Rgba([blended_r, blended_g, blended_b, a]);
        }
    }
}
