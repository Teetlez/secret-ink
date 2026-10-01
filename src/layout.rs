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

const HEADER_SCALE: f32 = 50.0;
const PAGE_GAP_PX: u32 = 48;

/// Lay out and position glyphs for each block of the document.
pub fn layout_blocks(
    blocks: &[Block],
    fonts: &HashMap<String, FontRef>,
    cfg: &Config,
) -> (Vec<GlyphInstance>, Vec<Redaction>, u32) {
    let mut instances = Vec::new();
    let mut redactions: Vec<Redaction> = Vec::new();
    let mut page_index = 0u32;
    let mut cursor_y = cfg.margin_top as f32;
    let page_stride = cfg.page_height + PAGE_GAP_PX;

    let wrap_width = (cfg.page_width - (cfg.margin_left + cfg.margin_right)) as usize
        / cfg.letter_spacing as usize;
    let wrap_opts = Options::new(wrap_width)
        .wrap_algorithm(WrapAlgorithm::OptimalFit(Penalties::default()))
        .initial_indent("    ");

    let page_bottom = cfg.page_height as f32 - cfg.margin_bottom as f32;

    for block in blocks {
        match block {
            Block::Heading { level, text } => {
                let key = "heading".to_string();
                let font_ref = &fonts[&key];
                let scale = match level {
                    1 => 1.0,
                    2 => 0.85,
                    3 => 0.65,
                    _ => 0.55,
                };
                let heading_size = scale * cfg.heading_size;
                let heading_height = heading_size + cfg.line_spacing * 3.0;
                let page_origin = page_index as f32 * page_stride as f32;
                let page_limit = page_origin + page_bottom;

                if cursor_y + heading_height > page_limit {
                    page_index += 1;
                    cursor_y = cfg.margin_top as f32;
                }

                let font = font_ref.as_scaled(heading_size);
                let total_w = measure_text_width(font_ref, text, heading_size);
                let center_x = (cfg.page_width / 2) as f32;
                let mut x = center_x - (total_w * 0.5);
                let mut last: Option<ab_glyph::Glyph> = None;
                let baseline = page_origin + cursor_y + heading_size * 0.82;

                for ch in text.chars() {
                    let id = font_ref.glyph_id(ch);
                    if let Some(prev) = last.take() {
                        x += font.kern(prev.id, id);
                    }
                    let glyph = id.with_scale_and_position(heading_size, point(x, baseline));
                    instances.push(GlyphInstance {
                        glyph: glyph.clone(),
                        font_key: key.clone(),
                    });
                    last = Some(glyph.clone());
                    x += font.h_advance(id) * 1.12;
                }

                cursor_y = baseline + heading_size * 0.95 + cfg.line_spacing * 2.0;
            }

            Block::Paragraph(text) => {
                let page_origin = page_index as f32 * page_stride as f32;
                let page_limit = page_origin + page_bottom;
                let wrapped = textwrap::wrap(text, &wrap_opts);
                let line_height = cfg.font_size + cfg.line_spacing * 1.8;
                let estimated_height = wrapped.len() as f32 * line_height + cfg.line_spacing * 3.0;

                if cursor_y + estimated_height > page_limit {
                    page_index += 1;
                    cursor_y = cfg.margin_top as f32;
                }

                let page_origin = page_index as f32 * page_stride as f32;
                let key = "default".to_string();
                let font_ref = &fonts[&key];
                let font = font_ref.as_scaled(cfg.font_size);
                let start = point(
                    cfg.margin_left as f32 + 18.0,
                    page_origin + cursor_y + cfg.line_spacing * 0.3,
                );
                let mut temp = Vec::new();

                let mut new_redactions = layout_paragraph(
                    font,
                    start,
                    (cfg.page_width - cfg.margin_left - cfg.margin_right) as f32,
                    &wrapped,
                    &mut temp,
                    cfg,
                );
                redactions.append(&mut new_redactions);

                cursor_y = temp
                    .last()
                    .map(|g| g.position.y - page_origin + cfg.line_spacing * 1.7)
                    .unwrap_or(cursor_y + cfg.line_spacing * 2.0);

                for glyph in temp {
                    instances.push(GlyphInstance {
                        glyph: glyph.clone(),
                        font_key: key.clone(),
                    });
                }
            }

            Block::Stamp(_inner) => {
                // TODO: schedule stamp drawing at bottom or top
            }
        }
    }

    (instances, redactions, page_index + 1)
}

/// Layout a single paragraph of text into glyph positions.
/// Follows the example from ab-glyph docs.
pub fn layout_paragraph<F, SF>(
    font: SF,
    position: ab_glyph::Point,
    _max_width: f32,
    text: &Vec<Cow<'_, str>>,
    target: &mut Vec<ab_glyph::Glyph>,
    cfg: &Config,
) -> Vec<Redaction>
where
    F: ab_glyph::Font,
    SF: ab_glyph::ScaleFont<F>,
{
    let mut caret = position + point(0.0, font.ascent());
    let _last: Option<ab_glyph::Glyph> = None;
    let mut redactions: Vec<Redaction> = Vec::new();
    for line in text {
        for c in line.chars() {
            if c == '\u{20D2}' {
                if let Some(r) = redactions.last_mut()
                    && r.is_open()
                {
                    r.close(caret);
                    continue;
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
        if redactions.last().is_some_and(|r| r.end.is_none()) {
            if let Some(r) = redactions.last_mut() {
                r.close(caret)
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Block;
    use std::path::PathBuf;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn sample_config() -> Config {
        let root = repo_root();
        Config {
            page_width: 2048,
            page_height: 2048,
            margin_top: 100,
            margin_bottom: 200,
            margin_left: 100,
            margin_right: 100,
            letter_spacing: 20.0,
            line_spacing: 20.0,
            default_font: root.join("fonts/Special_Elite/SpecialElite-Regular.ttf"),
            heading_font: root.join("fonts/Pica/Pica.ttf"),
            stamp_font: root.join("fonts/stampwriter_kit/STAMPWRITER-KIT.ttf"),
            font_size: 32.0,
            heading_size: 64.0,
            stamp_size: 72.0,
            jitter_px: 2.0,
            blur_sigma: 0.3,
            ink_opacity: 0.95,
            redaction_marker: "==".to_string(),
            stamp_marker: "!!".to_string(),
            paper_albedo: root.join("paper/CC0-Texture-Paper01/PaperAlbedo.png"),
            paper_normal: root.join("paper/CC0-Texture-Paper01/PaperNormal.png"),
            paper_roughness: root.join("paper/CC0-Texture-Paper01/PaperRough.png"),
        }
    }

    #[test]
    fn heading_and_paragraph_spacing_should_stay_distinct() {
        let cfg = sample_config();
        let default_font_data = std::fs::read(&cfg.default_font).unwrap();
        let heading_font_data = std::fs::read(&cfg.heading_font).unwrap();
        let default_font = FontRef::try_from_slice(&default_font_data).unwrap();
        let heading_font = FontRef::try_from_slice(&heading_font_data).unwrap();
        let mut fonts = HashMap::new();
        fonts.insert("default".to_string(), default_font);
        fonts.insert("heading".to_string(), heading_font);

        let blocks = vec![
            Block::Heading {
                level: 1,
                text: "TOP SECRET".to_string(),
            },
            Block::Paragraph(
                "This is the introductory paragraph that should appear below the title."
                    .to_string(),
            ),
        ];

        let (glyphs, _, _) = layout_blocks(&blocks, &fonts, &cfg);
        let heading_max_y = glyphs
            .iter()
            .filter(|g| g.font_key == "heading")
            .map(|g| g.glyph.position.y)
            .fold(f32::NEG_INFINITY, f32::max);
        let paragraph_min_y = glyphs
            .iter()
            .filter(|g| g.font_key == "default")
            .map(|g| g.glyph.position.y)
            .fold(f32::INFINITY, f32::min);

        assert!(paragraph_min_y > heading_max_y + cfg.heading_size * 0.5);
    }

    #[test]
    fn long_documents_create_multiple_pages_instead_of_overflowing() {
        let cfg = sample_config();
        let default_font_data = std::fs::read(&cfg.default_font).unwrap();
        let heading_font_data = std::fs::read(&cfg.heading_font).unwrap();
        let default_font = FontRef::try_from_slice(&default_font_data).unwrap();
        let heading_font = FontRef::try_from_slice(&heading_font_data).unwrap();
        let mut fonts = HashMap::new();
        fonts.insert("default".to_string(), default_font);
        fonts.insert("heading".to_string(), heading_font);

        let text = "This paragraph is repeated ".repeat(250);
        let blocks = vec![
            Block::Heading {
                level: 1,
                text: "MULTI PAGE".to_string(),
            },
            Block::Paragraph(text),
        ];

        let (_, _, page_count) = layout_blocks(&blocks, &fonts, &cfg);
        assert!(page_count > 1);
    }
}
