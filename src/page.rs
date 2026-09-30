use crate::config::Config;
use image::{GrayImage, RgbaImage, imageops::FilterType};

pub struct PageTextures {
    pub albedo: RgbaImage,
    pub normal: RgbaImage,
    pub roughness: GrayImage,
}

impl PageTextures {
    pub fn load(cfg: &Config) -> Result<Self, Box<dyn std::error::Error>> {
        let albedo = image::open(&cfg.paper_albedo)?.into_rgba8();
        let normal = image::open(&cfg.paper_normal)?.into_rgba8();
        let rough = image::open(&cfg.paper_roughness)?.to_luma8();
        let (width, height) = (cfg.page_width, cfg.page_height);
        Ok(PageTextures {
            albedo: image::imageops::resize(&albedo, width, height, FilterType::Triangle),
            normal: image::imageops::resize(&normal, width, height, FilterType::Triangle),
            roughness: image::imageops::resize(&rough, width, height, FilterType::Triangle),
        })
    }
}
