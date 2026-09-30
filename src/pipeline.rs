use std::{collections::HashMap, path::Path};

use ab_glyph::FontRef;
use image::RgbaImage;

use crate::{
    config::Config, layout::layout_blocks, page::PageTextures, parser::parse_document,
    renderer::render_page,
};

pub fn render_document(
    config: &Config,
    input_path: &Path,
    preview: bool,
) -> Result<RgbaImage, Box<dyn std::error::Error>> {
    let config = config.clone();

    // Keep the preview and final render on the same layout. The UI downscales the
    // final canvas for the preview thumbnail instead of changing the document geometry.
    let _ = preview;

    let default_font_data = std::fs::read(&config.default_font)?;
    let heading_font_data = std::fs::read(&config.heading_font)?;
    let stamp_font_data = std::fs::read(&config.stamp_font)?;
    let mut fonts = HashMap::new();
    fonts.insert(
        "default".into(),
        FontRef::try_from_slice(&default_font_data)?,
    );
    fonts.insert(
        "heading".into(),
        FontRef::try_from_slice(&heading_font_data)?,
    );
    fonts.insert("stamp".into(), FontRef::try_from_slice(&stamp_font_data)?);

    let textures = PageTextures::load(&config)?;
    let text = std::fs::read_to_string(input_path)?;
    let blocks = parse_document(&text, &config);
    let (glyphs, redactions) = layout_blocks(&blocks, &fonts, &config);

    Ok(render_page(
        &fonts,
        &glyphs,
        &redactions,
        &textures.albedo,
        &textures.normal,
        &textures.roughness,
        &config,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn preview_and_final_render_share_the_same_layout() {
        let root = repo_root();
        let input = root.join("input.md");
        let cfg = sample_config();

        let final_render = render_document(&cfg, &input, false).unwrap();
        let preview_render = render_document(&cfg, &input, true).unwrap();

        assert_eq!(final_render.dimensions(), preview_render.dimensions());
    }
}
