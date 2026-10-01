use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    process::Command,
    rc::Rc,
    sync::{Arc, Mutex},
};

use ab_glyph::FontRef;
use image::{Rgba, RgbaImage, imageops};
use slint::{ComponentHandle, Image, SharedPixelBuffer};

use crate::{config::Config, pipeline};

slint::include_modules!();

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let project_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let profile_path = project_dir.join("profile.toml");
    let mut config = Config::load_from(&profile_path)?;
    config.resolve_paths(profile_path.parent().unwrap_or(&project_dir));

    let ui = MainWindow::new()?;
    apply_config(&ui, &config);
    ui.set_input_path(project_dir.join("input.md").display().to_string().into());
    ui.set_output_path(project_dir.join("output.png").display().to_string().into());
    ui.set_status_text("Ready".into());
    refresh_previews(&ui);

    let active_profile = Rc::new(RefCell::new(profile_path));
    let preview_pages = Arc::new(Mutex::new(Vec::<RgbaImage>::new()));

    {
        let weak = ui.as_weak();
        ui.on_browse_file(move |kind| {
            let kind = kind.to_string();
            let dialog = rfd::FileDialog::new().set_title(match kind.as_str() {
                "input" => "Choose a Markdown document",
                "output" => "Choose the rendered image location",
                "default_font" | "heading_font" | "stamp_font" => "Choose a font file",
                _ => "Choose a paper texture",
            });
            let selected = match kind.as_str() {
                "output" => dialog
                    .add_filter("PNG image", &["png"])
                    .set_file_name("output.png")
                    .save_file(),
                "input" => dialog
                    .add_filter("Markdown", &["md", "markdown", "txt"])
                    .pick_file(),
                "default_font" | "heading_font" | "stamp_font" => {
                    dialog.add_filter("Font files", &["ttf", "otf"]).pick_file()
                }
                _ => dialog
                    .add_filter("Image files", &["png", "jpg", "jpeg", "webp", "tif", "bmp"])
                    .pick_file(),
            };
            let Some(path) = selected else { return };
            let Some(ui) = weak.upgrade() else { return };
            let path: slint::SharedString = path.display().to_string().into();
            match kind.as_str() {
                "input" => ui.set_input_path(path),
                "output" => ui.set_output_path(path),
                "default_font" => ui.set_default_font(path),
                "heading_font" => ui.set_heading_font(path),
                "stamp_font" => ui.set_stamp_font(path),
                "paper_albedo" => ui.set_paper_albedo(path),
                "paper_normal" => ui.set_paper_normal(path),
                "paper_roughness" => ui.set_paper_roughness(path),
                _ => return,
            }
            refresh_previews(&ui);
            ui.set_status_text("File selection updated".into());
        });
    }

    {
        let weak = ui.as_weak();
        let active_profile = Rc::clone(&active_profile);
        ui.on_load_profile(move || {
            let Some(path) = rfd::FileDialog::new()
                .add_filter("TOML profile", &["toml"])
                .pick_file()
            else {
                return;
            };
            let Some(ui) = weak.upgrade() else { return };
            match Config::load_from(&path) {
                Ok(mut config) => {
                    config.resolve_paths(path.parent().unwrap_or(Path::new(".")));
                    apply_config(&ui, &config);
                    *active_profile.borrow_mut() = path;
                    refresh_previews(&ui);
                    ui.set_status_text("Profile loaded".into());
                }
                Err(error) => ui.set_status_text(format!("Could not load profile: {error}").into()),
            }
        });
    }

    {
        let weak = ui.as_weak();
        let active_profile = Rc::clone(&active_profile);
        ui.on_save_profile(move || {
            let Some(ui) = weak.upgrade() else { return };
            match config_from_ui(&ui).and_then(|config| {
                config
                    .save_to(active_profile.borrow().as_path())
                    .map_err(|error| error.to_string())?;
                Ok(())
            }) {
                Ok(()) => ui.set_status_text("Profile saved".into()),
                Err(error) => ui.set_status_text(format!("Could not save profile: {error}").into()),
            }
        });
    }

    {
        let weak = ui.as_weak();
        let preview_pages = Arc::clone(&preview_pages);
        ui.on_preview_document(move || {
            start_render(weak.clone(), true, Arc::clone(&preview_pages))
        });
    }
    {
        let weak = ui.as_weak();
        let preview_pages = Arc::clone(&preview_pages);
        ui.on_render_document(move || {
            start_render(weak.clone(), false, Arc::clone(&preview_pages))
        });
    }
    {
        let weak = ui.as_weak();
        let preview_pages = Arc::clone(&preview_pages);
        ui.on_previous_preview_page(move || {
            let Some(ui) = weak.upgrade() else { return };
            let current = ui.get_preview_page_number().max(1) as usize;
            show_preview_page(&ui, &preview_pages, current.saturating_sub(2));
        });
    }
    {
        let weak = ui.as_weak();
        let preview_pages = Arc::clone(&preview_pages);
        ui.on_next_preview_page(move || {
            let Some(ui) = weak.upgrade() else { return };
            let current = ui.get_preview_page_number().max(1) as usize;
            show_preview_page(&ui, &preview_pages, current);
        });
    }
    {
        let weak = ui.as_weak();
        ui.on_open_output(move || {
            let Some(ui) = weak.upgrade() else { return };
            let path = PathBuf::from(ui.get_output_path().to_string());
            match open_image(&path) {
                Ok(()) => ui.set_status_text("Opened rendered image".into()),
                Err(error) => ui.set_status_text(error.into()),
            }
        });
    }

    ui.run()?;
    Ok(())
}

fn apply_config(ui: &MainWindow, config: &Config) {
    ui.set_page_width(config.page_width as f32);
    ui.set_page_height(config.page_height as f32);
    ui.set_margin_top(config.margin_top as f32);
    ui.set_margin_bottom(config.margin_bottom as f32);
    ui.set_margin_left(config.margin_left as f32);
    ui.set_margin_right(config.margin_right as f32);
    ui.set_letter_spacing(config.letter_spacing);
    ui.set_line_spacing(config.line_spacing);
    ui.set_font_size(config.font_size);
    ui.set_heading_size(config.heading_size);
    ui.set_stamp_size(config.stamp_size);
    ui.set_jitter_px(config.jitter_px);
    ui.set_blur_sigma(config.blur_sigma);
    ui.set_ink_opacity(config.ink_opacity);
    ui.set_default_font(config.default_font.display().to_string().into());
    ui.set_heading_font(config.heading_font.display().to_string().into());
    ui.set_stamp_font(config.stamp_font.display().to_string().into());
    ui.set_paper_albedo(config.paper_albedo.display().to_string().into());
    ui.set_paper_normal(config.paper_normal.display().to_string().into());
    ui.set_paper_roughness(config.paper_roughness.display().to_string().into());
    ui.set_redaction_marker(config.redaction_marker.clone().into());
    ui.set_stamp_marker(config.stamp_marker.clone().into());
}

fn config_from_ui(ui: &MainWindow) -> Result<Config, String> {
    let page_width = ui.get_page_width().round().max(1.0) as u32;
    let page_height = ui.get_page_height().round().max(1.0) as u32;
    let margin_top = ui.get_margin_top().round().max(0.0) as u32;
    let margin_bottom = ui.get_margin_bottom().round().max(0.0) as u32;
    let margin_left = ui.get_margin_left().round().max(0.0) as u32;
    let margin_right = ui.get_margin_right().round().max(0.0) as u32;
    if margin_left + margin_right >= page_width || margin_top + margin_bottom >= page_height {
        return Err("Margins must leave room for the page content".into());
    }

    Ok(Config {
        page_width,
        page_height,
        margin_top,
        margin_bottom,
        margin_left,
        margin_right,
        letter_spacing: ui.get_letter_spacing().max(1.0),
        line_spacing: ui.get_line_spacing().max(1.0),
        default_font: ui.get_default_font().to_string().into(),
        heading_font: ui.get_heading_font().to_string().into(),
        stamp_font: ui.get_stamp_font().to_string().into(),
        font_size: ui.get_font_size().max(1.0),
        heading_size: ui.get_heading_size().max(1.0),
        stamp_size: ui.get_stamp_size().max(1.0),
        jitter_px: ui.get_jitter_px().max(0.0),
        blur_sigma: ui.get_blur_sigma().max(0.1),
        ink_opacity: ui.get_ink_opacity().clamp(0.0, 1.0),
        redaction_marker: ui.get_redaction_marker().to_string(),
        stamp_marker: ui.get_stamp_marker().to_string(),
        paper_albedo: ui.get_paper_albedo().to_string().into(),
        paper_normal: ui.get_paper_normal().to_string().into(),
        paper_roughness: ui.get_paper_roughness().to_string().into(),
    })
}

fn refresh_previews(ui: &MainWindow) {
    ui.set_albedo_preview(load_image_preview(ui.get_paper_albedo().as_ref()));
    ui.set_normal_preview(load_image_preview(ui.get_paper_normal().as_ref()));
    ui.set_roughness_preview(load_image_preview(ui.get_paper_roughness().as_ref()));
    ui.set_default_font_preview(load_font_preview(ui.get_default_font().as_ref()));
    ui.set_heading_font_preview(load_font_preview(ui.get_heading_font().as_ref()));
    ui.set_stamp_font_preview(load_font_preview(ui.get_stamp_font().as_ref()));
}

fn load_image_preview(path: &str) -> Image {
    let Ok(image) = image::open(path) else {
        return Image::default();
    };
    let image = image.thumbnail(360, 180).to_rgba8();
    image_to_slint(&image)
}

fn load_font_preview(path: &str) -> Image {
    let Ok(data) = std::fs::read(path) else {
        return Image::default();
    };
    let Ok(font) = FontRef::try_from_slice(&data) else {
        return Image::default();
    };
    let mut sample = RgbaImage::from_pixel(560, 56, Rgba([246, 242, 231, 255]));
    imageproc::drawing::draw_text_mut(
        &mut sample,
        Rgba([33, 43, 40, 255]),
        10,
        11,
        25.0,
        &font,
        "Confidential  0123456789",
    );
    image_to_slint(&sample)
}

fn image_to_slint(image: &RgbaImage) -> Image {
    let pixels = SharedPixelBuffer::clone_from_slice(image.as_raw(), image.width(), image.height());
    Image::from_rgba8(pixels)
}

fn start_render(
    weak: slint::Weak<MainWindow>,
    preview: bool,
    preview_pages: Arc<Mutex<Vec<RgbaImage>>>,
) {
    let Some(ui) = weak.upgrade() else { return };
    let config = match config_from_ui(&ui) {
        Ok(config) => config,
        Err(error) => {
            ui.set_status_text(error.into());
            return;
        }
    };
    let input_path = PathBuf::from(ui.get_input_path().to_string());
    let output_path = PathBuf::from(ui.get_output_path().to_string());
    ui.set_working(true);
    ui.set_status_text(if preview {
        "Rendering reduced preview...".into()
    } else {
        "Rendering full-size image...".into()
    });

    std::thread::spawn(move || {
        let result =
            pipeline::render_document_pages(&config, &input_path, preview)
                .map_err(|error| error.to_string())
                .and_then(|pages| {
                    let saved = if preview {
                        None
                    } else {
                        let saved = pipeline::save_rendered_pages(&output_path, &pages)
                            .map_err(|error| error.to_string())?;
                        Some(saved)
                    };
                    let status =
                        if let Some(saved) = saved {
                            if saved.len() > 1 {
                                let folder =
                                    saved.first().and_then(|path| path.parent()).unwrap_or_else(
                                        || output_path.parent().unwrap_or_else(|| Path::new(".")),
                                    );
                                format!("Rendered {} pages to {}", saved.len(), folder.display())
                            } else {
                                let (width, height) = pages[0].dimensions();
                                format!(
                                    "Rendered {width} x {height} px to {}",
                                    output_path.display()
                                )
                            }
                        } else {
                            format!("Preview ready: {} page(s)", pages.len())
                        };
                    let thumbs = pages
                        .iter()
                        .map(|page| imageops::thumbnail(page, 768, 768))
                        .collect::<Vec<_>>();
                    Ok((thumbs, status))
                });

        let _ = slint::invoke_from_event_loop(move || {
            let Some(ui) = weak.upgrade() else { return };
            ui.set_working(false);
            match result {
                Ok((pages, status)) => {
                    if let Ok(mut stored_pages) = preview_pages.lock() {
                        *stored_pages = pages;
                    }
                    show_preview_page(&ui, &preview_pages, 0);
                    ui.set_has_preview(true);
                    ui.set_status_text(status.into());
                }
                Err(error) => ui.set_status_text(format!("Render failed: {error}").into()),
            }
        });
    });
}

fn show_preview_page(ui: &MainWindow, pages: &Arc<Mutex<Vec<RgbaImage>>>, index: usize) {
    let Ok(pages) = pages.lock() else { return };
    let Some(page) = pages.get(index) else { return };
    ui.set_result_preview(image_to_slint(page));
    ui.set_preview_page_number(index as i32 + 1);
    ui.set_preview_page_count(pages.len() as i32);
    ui.set_preview_page_label(format!("Page {} / {}", index + 1, pages.len()).into());
}

fn open_image(path: &Path) -> Result<(), String> {
    if !path.is_file() {
        return Err("Render an image before opening it".into());
    }
    #[cfg(target_os = "windows")]
    let result = Command::new("cmd")
        .args(["/C", "start", ""])
        .arg(path)
        .spawn();
    #[cfg(target_os = "macos")]
    let result = Command::new("open").arg(path).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let result = Command::new("xdg-open").arg(path).spawn();
    result
        .map(|_| ())
        .map_err(|error| format!("Could not open image: {error}"))
}
