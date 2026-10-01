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
    let (glyphs, redactions, page_count) = layout_blocks(&blocks, &fonts, &config);

    let page_gap = 48u32;
    let total_height = config.page_height * page_count + page_gap * (page_count.saturating_sub(1));
    let mut albedo = RgbaImage::new(config.page_width, total_height);
    let mut normal = RgbaImage::new(config.page_width, total_height);
    let mut roughness = image::GrayImage::new(config.page_width, total_height);

    for page in 0..page_count {
        let y = page * (config.page_height + page_gap);
        image::imageops::overlay(&mut albedo, &textures.albedo, 0, y as i64);
        image::imageops::overlay(&mut normal, &textures.normal, 0, y as i64);
        image::imageops::overlay(&mut roughness, &textures.roughness, 0, y as i64);
    }

    Ok(render_page(
        &fonts,
        &glyphs,
        &redactions,
        &albedo,
        &normal,
        &roughness,
        &config,
    ))
}

pub fn render_document_pages(
    config: &Config,
    input_path: &Path,
    preview: bool,
) -> Result<Vec<RgbaImage>, Box<dyn std::error::Error>> {
    let tall = render_document(config, input_path, preview)?;
    let page_height = config.page_height;
    let page_gap = 48u32;

    if tall.height() <= page_height {
        return Ok(vec![tall]);
    }

    let page_stride = page_height + page_gap;
    let page_count = ((tall.height().saturating_sub(1)) / page_stride + 1) as usize;
    let mut pages = Vec::with_capacity(page_count);

    for index in 0..page_count {
        let y_start = index as u32 * page_stride;
        let y_end = (y_start + page_height).min(tall.height());
        let height = y_end.saturating_sub(y_start);
        let mut page = RgbaImage::new(config.page_width, page_height);

        let crop = image::imageops::crop_imm(&tall, 0, y_start, config.page_width, height);
        image::imageops::overlay(&mut page, &crop.to_image(), 0, 0);
        pages.push(page);
    }

    Ok(pages)
}

pub fn save_rendered_pages(
    output_path: &Path,
    pages: &[RgbaImage],
) -> Result<Vec<std::path::PathBuf>, Box<dyn std::error::Error>> {
    if pages.is_empty() {
        return Err("rendered page list is empty".into());
    }

    if pages.len() == 1 {
        pages[0].save(output_path)?;
        return Ok(vec![output_path.to_path_buf()]);
    }

    let parent = output_path.parent().unwrap_or_else(|| Path::new("."));
    let stem = output_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("document");
    let folder = parent.join(format!("{stem}_pages"));
    std::fs::create_dir_all(&folder)?;

    let mut saved = Vec::with_capacity(pages.len());
    for (index, page) in pages.iter().enumerate() {
        let page_path = folder.join(format!("page_{:02}.png", index + 1));
        page.save(&page_path)?;
        saved.push(page_path);
    }

    Ok(saved)
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

    #[test]
    fn multipage_render_saves_one_file_per_page() {
        let root = repo_root();
        let input = root.join("stress_input.md");
        let cfg = sample_config();
        let pages = render_document_pages(&cfg, &input, false).unwrap();
        assert!(pages.len() >= 1);

        let output_dir = root.join("target/test-output-pages");
        let _ = std::fs::remove_dir_all(&output_dir);
        let folder = output_dir.join("document_pages");
        let saved = save_rendered_pages(&output_dir.join("document.png"), &pages).unwrap();
        assert_eq!(saved.len(), pages.len());
        assert!(saved.iter().all(|p| p.parent().unwrap() == folder));
    }
}
