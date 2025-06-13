use std::{borrow::Cow, collections::HashMap};

use crate::{config::Config, parser::Block};
use ab_glyph::{Font, FontRef, Glyph, ScaleFont, point};
use textwrap::{Options, WrapAlgorithm, wrap_algorithms::Penalties};

/// Represents a positioned glyph on the page.
pub struct GlyphInstance {
    pub glyph: Glyph,
    pub font_key: String,
}

/// Lay out and position glyphs for each block of the document.
pub fn layout_blocks(
    blocks: &[Block],
    fonts: &HashMap<String, FontRef>,
    cfg: &Config,
) -> Vec<GlyphInstance> {
    let mut instances = Vec::new();
    let mut cursor_y = cfg.margin_top as f32;

    // Wrap options with full width and optimal-fit algorithm
    let wrap_width = (cfg.page_width - (cfg.margin_left + cfg.margin_right)) as usize;
    let wrap_opts =
        Options::new(wrap_width).wrap_algorithm(WrapAlgorithm::OptimalFit(Penalties::default()));

    for block in blocks {
        match block {
            Block::Heading { level, text } => {
                // Determine font scale per heading level
                let scale = match level {
                    1 => 128.0,
                    2 => 64.0,
                    3 => 32.0,
                    _ => 16.0,
                };
                let key = "heading".to_string();
                let font_ref = &fonts[&key];
                let font = font_ref.as_scaled(scale);

                // Centered horizontally
                let total_w = measure_text_width(font_ref, text, scale);
                let mut x = (cfg.page_width as f32 - total_w) / 2.0;

                // Baseline for this heading line
                let baseline = cursor_y + font.ascent();
                for ch in text.chars() {
                    let id = font_ref.glyph_id(ch);
                    x += font.h_side_bearing(id);
                    let glyph = id.with_scale_and_position(scale, point(x, baseline));
                    instances.push(GlyphInstance {
                        glyph: glyph.clone(),
                        font_key: key.clone(),
                    });
                    x += font.h_advance(id);
                }

                // Advance cursor with extra spacing
                cursor_y += font.height() + font.line_gap() * 1.2;
            }

            Block::Paragraph(text) => {
                // Use helper to layout paragraphs
                let key = "default".to_string();
                let font_ref = &fonts[&key];
                let font = font_ref.as_scaled(cfg.font_size);
                let start = point(cfg.margin_left as f32, cursor_y);
                let text = textwrap::wrap(text, &wrap_opts);
                // Collect glyphs, then convert to GlyphInstance
                let mut temp = Vec::new();
                layout_paragraph(
                    font.clone(),
                    start,
                    (cfg.page_width - cfg.margin_left - cfg.margin_right) as f32,
                    &text,
                    &mut temp,
                );

                // Move cursor down by paragraph height (approximate)
                cursor_y = &temp.last().map(|g| g.position.y).unwrap_or(cursor_y)
                    + font.height()
                    + font.line_gap();

                for glyph in temp {
                    instances.push(GlyphInstance {
                        glyph: glyph.clone(),
                        font_key: key.clone(),
                    });
                }
            }

            Block::Redaction(inner) => {
                // TODO: draw a solid black rectangle at (cfg.margin_left, cursor_y)
                // with width measured by text length and height = line height
                cursor_y += cfg.font_size * 1.2;
            }

            Block::Stamp(inner) => {
                // TODO: schedule stamp drawing at bottom or top
                cursor_y += cfg.stamp_size * 1.2;
            }
        }
    }

    instances
}

/// Layout a single paragraph of text into glyph positions.
/// Follows the example from ab-glyph docs.
pub fn layout_paragraph<F, SF>(
    font: SF,
    position: ab_glyph::Point,
    max_width: f32,
    text: &Vec<Cow<'_, str>>,
    target: &mut Vec<ab_glyph::Glyph>,
) where
    F: ab_glyph::Font,
    SF: ab_glyph::ScaleFont<F>,
{
    let v_advance = font.height() + font.line_gap();
    let mut caret = position + point(50.0, font.ascent());
    let mut last: Option<ab_glyph::Glyph> = None;

    for line in text {
        println!("{}", line);
        for c in line.chars() {
            let mut glyph = font.scaled_glyph(c);
            if let Some(prev) = last.take() {
                caret.x += font.kern(prev.id, glyph.id);
            }
            glyph.position = caret;

            last = Some(glyph.clone());
            caret.x += font.h_advance(glyph.id);
            target.push(glyph);
        }
        caret = point(position.x, caret.y + v_advance);
        last = None;
    }
}

// helper to measure
fn measure_text_width(font: &FontRef, text: &str, scale: f32) -> f32 {
    let mut w = 0.0;
    let scaled_font = font.as_scaled(scale);
    let mut last: Option<ab_glyph::Glyph> = None;
    for ch in text.chars() {
        let glyph = scaled_font.scaled_glyph(ch);
        w += if let Some(prev) = last.take() {
            scaled_font.kern(prev.id, glyph.id)
        } else {
            0.0
        } + scaled_font.h_advance(glyph.id);
        last = Some(glyph.clone());
    }
    w
}
