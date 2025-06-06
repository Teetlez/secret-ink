use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use pulldown_cmark::{Event, Parser, Tag};
use rand::Rng;

use image::RgbImage;
use image::{DynamicImage, GenericImage, GenericImageView, ImageBuffer, LumaA};
use imageproc::drawing::{draw_filled_rect_mut, draw_text_mut};
use imageproc::rect::Rect;
use rusttype::{Font, Scale, point};

mod effects;
mod parser;
mod renderer;

use crate::effects::effects::apply_aging_effects;
use parser::{Document, DocumentElement};
use renderer::Renderer;

fn main() -> io::Result<()> {
    // 1. Load the Markdown-like source file
    let input_path = "input.md";
    let mut md_content = String::new();
    File::open(input_path)?.read_to_string(&mut md_content)?;

    // 2. Parse into our internal Document model
    let doc = parser::parse_markdown(&md_content);

    // 3. Set up rendering parameters (could be loaded from a front-matter in the MD)
    let params = renderer::RenderParams {
        page_width: 2480,  // e.g. 8.5"×11" at 300 DPI
        page_height: 3508, // ~ A4 at 300 DPI, adjust as needed
        margin: 100,
        font_path: "fonts/CourierPrime-Regular.ttf".into(),
        font_size: 32.0,
        line_spacing: 1.2,
        redaction_color: [0u8, 0u8, 0u8],
        text_color: [20u8, 20u8, 20u8],
        background_color: [240u8, 236u8, 220u8], // off-white parchment
        jitter_max: 2,                           // max ±2px jitter per glyph
    };

    // 4. Create a renderer and draw the page
    let mut renderer = Renderer::new(params);
    let mut page_image = renderer.render_document(&doc);

    // 5. Apply aging/scan effects
    effects::apply_aging_effects(&mut page_image);

    // 6. Save the final image
    let output_path = "output.png";
    page_image.save(output_path)?;
    println!("Rendered document saved to {}", output_path);

    Ok(())
}
