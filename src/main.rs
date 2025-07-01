mod config;
mod layout;
mod page;
mod parser;
mod renderer;

use ab_glyph::FontRef;
use config::Config;
use layout::layout_blocks;
use page::PageTextures;
use parser::parse_document;
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Load config
    print!("loading config...");
    let cfg = Config::load_from("profile.toml")?;
    println!("OK");
    // 2. Load fonts
    print!("loading fonts...");
    let mut fonts = HashMap::new();
    print!("default...");
    let binding = std::fs::read(&cfg.default_font)?;
    fonts.insert("default".into(), FontRef::try_from_slice(&binding)?);

    print!("header...");
    let binding = std::fs::read(&cfg.heading_font)?;
    fonts.insert("heading".into(), FontRef::try_from_slice(&binding)?);

    print!("stamp...");
    let binding = std::fs::read(&cfg.stamp_font)?;
    fonts.insert("stamp".into(), FontRef::try_from_slice(&binding)?);
    println!("OK");

    // 3. Load paper textures
    print!("loading paper texture...");
    let textures = PageTextures::load(&cfg)?;
    println!("OK");

    // 4. Read and parse text
    print!("reading input file...");
    let text = std::fs::read_to_string("input.md")?;
    let blocks = parse_document(&text, &cfg);
    println!("OK");

    // 5. Layout glyphs
    print!("creating text layout...");
    let (glyphs, redactions) = layout_blocks(&blocks, &fonts, &cfg);
    println!("OK");

    // 6. Render page(s)
    print!("rendering document...");
    let canvas = renderer::render_page(
        &fonts,
        &glyphs,
        &redactions,
        &textures.albedo,
        &textures.normal,
        &textures.roughness,
        &cfg,
    );
    println!("OK");

    // 7. Save
    print!("writing to image...");
    canvas.save("output.png")?;
    println!("OK");

    Ok(())
}
