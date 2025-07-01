use std::{borrow::Cow, collections::HashMap};

use crate::{config::Config, parser::Block};
use ab_glyph::{Font, FontRef, Glyph, Point, ScaleFont, point};
use textwrap::{Options, WrapAlgorithm, wrap_algorithms::Penalties};

#[derive(Debug, Clone)]
/// Represents a positioned glyph on the page.
pub struct GlyphInstance {
    pub glyph: Glyph,
    pub font_key: String,
}

#[derive(Debug, Clone)]
pub struct Redaction {
    pub start: Point,
    pub end: Option<Point>,
    pub thickness: f32,
}

impl Redaction {
    pub fn new(p: Point, t: f32) -> Self {
        Redaction {
            start: p,
            end: None,
            thickness: t,
        }
    }
    pub fn close(&mut self, p: Point) {
        if self.end.is_none() {
            self.end = Some(p);
        }
    }
    pub fn is_open(&self) -> bool {
        self.end.is_none()
    }
}

const header_scale: f32 = 50.0;

/// Lay out and position glyphs for each block of the document.
pub fn layout_blocks(
    blocks: &[Block],
    fonts: &HashMap<String, FontRef>,
    cfg: &Config,
) -> (Vec<GlyphInstance>, Vec<Redaction>) {
    let mut instances = Vec::new();
    let mut cursor_y = cfg.margin_top as f32;
    let mut redactions: Vec<Redaction> = Vec::new();

    // Wrap options with full width and optimal-fit algorithm
    let wrap_width = (cfg.page_width - (cfg.margin_left + cfg.margin_right)) as usize
        / cfg.letter_spacing as usize;
    let wrap_opts = Options::new(wrap_width)
        .wrap_algorithm(WrapAlgorithm::OptimalFit(Penalties::default()))
        .initial_indent("    ");

    for block in blocks {
        match block {
            Block::Heading { level, text } => {
                // Determine font scale per heading level
                let key = "heading".to_string();
                let font_ref = &fonts[&key];
                let scale = match level {
                    1 => 1.0,
                    2 => 0.85,
                    3 => 0.65,
                    _ => 0.55,
                };
                let font = font_ref.as_scaled(scale * cfg.heading_size);

                // Centered horizontally
                let total_w = measure_text_width(font_ref, text, scale * cfg.heading_size);
                let mut x = (cfg.page_width / 2) as f32 - (total_w * 0.5);
                let mut last: Option<ab_glyph::Glyph> = None;

                // Baseline for this heading line
                let baseline = cursor_y;
                for ch in text.chars() {
                    let id = font_ref.glyph_id(ch);
                    if let Some(prev) = last.take() {
                        x += font.kern(prev.id, id);
                    }
                    x += font.h_side_bearing(id);
                    let glyph =
                        id.with_scale_and_position(scale * cfg.heading_size, point(x, baseline));
                    instances.push(GlyphInstance {
                        glyph: glyph.clone(),
                        font_key: key.clone(),
                    });

                    last = Some(glyph.clone());
                    x += font.h_advance(id) * 1.2;
                }

                // Advance cursor with extra spacing
                cursor_y += scale;
            }

            Block::Paragraph(text) => {
                // Use helper to layout paragraphs
                let key = "default".to_string();
                let font_ref = &fonts[&key];
                let font = font_ref.as_scaled(cfg.font_size);
                let start = point(cfg.margin_left as f32, cursor_y);
                let text = textwrap::wrap(&text, &wrap_opts);
                // Collect glyphs, then convert to GlyphInstance
                let mut temp = Vec::new();

                let mut new_redactions = layout_paragraph(
                    font.clone(),
                    start,
                    (cfg.page_width - cfg.margin_left - cfg.margin_right) as f32,
                    &text,
                    &mut temp,
                    &cfg,
                );
                redactions.append(&mut new_redactions);

                // Move cursor down by paragraph height (approximate)
                cursor_y =
                    &temp.last().map(|g| g.position.y).unwrap_or(cursor_y) + cfg.line_spacing;

                for glyph in temp {
                    instances.push(GlyphInstance {
                        glyph: glyph.clone(),
                        font_key: key.clone(),
                    });
                }
            }

            Block::Stamp(inner) => {
                // TODO: schedule stamp drawing at bottom or top
            }
        }
    }

    (instances, redactions)
}

/// Layout a single paragraph of text into glyph positions.
/// Follows the example from ab-glyph docs.
pub fn layout_paragraph<F, SF>(
    font: SF,
    position: ab_glyph::Point,
    max_width: f32,
    text: &Vec<Cow<'_, str>>,
    target: &mut Vec<ab_glyph::Glyph>,
    cfg: &Config,
) -> Vec<Redaction>
where
    F: ab_glyph::Font,
    SF: ab_glyph::ScaleFont<F>,
{
    let mut caret = position + point(0.0, font.ascent());
    let mut last: Option<ab_glyph::Glyph> = None;
    let mut redactions: Vec<Redaction> = Vec::new();
    for line in text {
        println!("{}", line);
        for c in line.chars() {
            if c == '\u{20D2}' {
                if let Some(r) = redactions.last_mut() {
                    if r.is_open() {
                        r.close(caret);
                        continue;
                    }
                }
                redactions.push(Redaction::new(caret, font.height()));
                continue;
            }
            let mut glyph = font.scaled_glyph(c);
            // if let Some(prev) = last.take() {
            //     caret.x += font.kern(prev.id, glyph.id);
            // }
            glyph.position = caret;
            // last = Some(glyph.clone());
            caret.x += cfg.letter_spacing;
            target.push(glyph);
        }
        let new_caret = point(
            position.x + (fastrand::f32() * 2.0 - 1.0),
            caret.y + cfg.line_spacing,
        );
        if redactions.last().map_or(false, |r| r.end.is_none()) {
            redactions.last_mut().map(|r| r.close(caret));
            redactions.push(Redaction::new(new_caret, font.height()));
        }
        caret = new_caret;
        // last = None;
    }
    // redactions.last_mut().map(|r| {
    //     if r.is_open() {
    //         r.close(caret);
    //     }
    // });
    redactions
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
