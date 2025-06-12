use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize, Debug)]
pub struct Config {
    // Page
    pub page_width: u32,
    pub page_height: u32,
    pub margin_top: u32,
    pub margin_bottom: u32,
    pub margin_left: u32,
    pub margin_right: u32,

    // Fonts
    pub default_font: PathBuf,
    pub heading_font: PathBuf,
    pub stamp_font: PathBuf,
    pub font_size: f32,
    pub heading_size: f32,
    pub stamp_size: f32,

    // Ink effects
    pub jitter_px: f32,
    pub blur_sigma: f32,
    pub ink_opacity: f32,

    // Text markers
    pub redaction_marker: String,
    pub stamp_marker: String,

    // Paper textures
    pub paper_albedo: PathBuf,
    pub paper_normal: PathBuf,
    pub paper_roughness: PathBuf,
}

impl Config {
    pub fn load_from(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let s = std::fs::read_to_string(path)?;
        let cfg: Config = toml::from_str(&s)?;
        Ok(cfg)
    }
}
