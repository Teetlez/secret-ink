use crate::config::Config;
use image::{GrayImage, RgbaImage};

pub struct PageTextures {
    pub albedo: RgbaImage,
    pub normal: RgbaImage,
    pub roughness: GrayImage,
}

impl PageTextures {
    pub fn load(cfg: &Config) -> Result<Self, Box<dyn std::error::Error>> {
        print!("color...");
        let albedo = image::open(&cfg.paper_albedo)?.into_rgba8();
        print!("normal...");
        let normal = image::open(&cfg.paper_normal)?.into_rgba8();
        print!("roughness...");
        let rough = image::open(&cfg.paper_roughness)?.to_luma8();
        Ok(PageTextures {
            albedo,
            normal,
            roughness: rough,
        })
    }
}
