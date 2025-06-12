use std::collections::HashMap;

use crate::{config::Config, parser::Block};
use ab_glyph::{Font, FontRef, Glyph, ScaleFont, point};
use textwrap::{Options, wrap_algorithms::Penalties};

pub struct GlyphInstance {
    pub glyph: Glyph,
    pub x: f32,
    pub y: f32,
    pub font_key: String,
}

pub fn layout_blocks(
    blocks: &[Block],
    fonts: &HashMap<String, FontRef>,
    cfg: &Config,
) -> Vec<GlyphInstance> {
    let mut instances = Vec::new();
    let mut cursor_y = cfg.margin_top as f32;
    let options = Options::new((cfg.page_width - cfg.margin_left - cfg.margin_right) as usize)
        .wrap_algorithm(textwrap::WrapAlgorithm::OptimalFit(Penalties::default()))
        .initial_indent("    ");

    for block in blocks {
        match block {
            Block::Heading { level, text } => {
                // center heading
                let font = fonts.get("heading").unwrap();
                let scale = cfg.heading_size;
                let width = measure_text_width(font, text, scale);
                let x = (cfg.page_width as f32 - width) / 2.0;
                for (i, ch) in text.chars().enumerate() {
                    let g = font.glyph_id(ch).with_scale_and_position(
                        scale,
                        point(x + i as f32 * scale * 0.6, cursor_y),
                    );
                    instances.push(GlyphInstance {
                        glyph: g,
                        x,
                        y: cursor_y,
                        font_key: "heading".into(),
                    });
                }
                cursor_y += scale * 1.2;
            }
            Block::Paragraph(text) => {
                // word-wrap then measure each glyph
                for line in textwrap::wrap(text, &options) {
                    let mut x = cfg.margin_left as f32;
                    let font = fonts.get("default").unwrap();
                    let scaled_font = font.as_scaled(cfg.font_size);
                    for ch in line.chars() {
                        let g = font
                            .glyph_id(ch)
                            .with_scale_and_position(cfg.font_size, point(x, cursor_y));
                        let ch_height = font.glyph_bounds(&g).height();
                        instances.push(GlyphInstance {
                            glyph: g,
                            x,
                            y: cursor_y + ch_height,
                            font_key: "default".into(),
                        });
                        x += scaled_font.h_advance(font.glyph_id(ch));
                        // increment x by glyph advance
                        // (you’ll want to query font.h_advance and kerning)
                    }
                    cursor_y += scaled_font.height() + scaled_font.line_gap();
                }
            }
            Block::Redaction(inner) => {
                // treat content as a black bar rather than real glyphs
                // You can decide to place a filled rectangle instead of glyphs
            }
            Block::Stamp(inner) => {
                // place stamp later in the rendering phase
            }
        }

        // check cursor_y + next line height > page_height - margin_bottom => new page
    }

    instances
}

// helper to measure
fn measure_text_width(font: &FontRef, text: &str, scale: f32) -> f32 {
    let mut w = 0.0;
    let scaled_font = font.as_scaled(scale);
    for ch in text.chars() {
        let id = font.glyph_id(ch);
        w += scaled_font.h_advance(id) + scaled_font.kern(id, id);
    }
    w
}
