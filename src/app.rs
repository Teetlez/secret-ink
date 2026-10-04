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
    update_studio_config(&ui, |state| {
        state.input_path = project_dir.join("input.md").display().to_string().into();
        state.output_path = project_dir.join("output.png").display().to_string().into();
        true
    });
    set_status_text(&ui, "Ready");
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
            let changed = update_studio_config(&ui, |state| {
                match kind.as_str() {
                    "input" => state.input_path = path,
                    "output" => state.output_path = path,
                    "default_font" => state.default_font = path,
                    "heading_font" => state.heading_font = path,
                    "stamp_font" => state.stamp_font = path,
                    "paper_albedo" => state.paper_albedo = path,
                    "paper_normal" => state.paper_normal = path,
                    "paper_roughness" => state.paper_roughness = path,
                    _ => return false,
                }
                true
            });
            if !changed {
                return;
            }
            refresh_previews(&ui);
            set_status_text(&ui, "File selection updated");
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
                    set_status_text(&ui, "Profile loaded");
                }
                Err(error) => set_status_text(&ui, format!("Could not load profile: {error}")),
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
                Ok(()) => set_status_text(&ui, "Profile saved"),
                Err(error) => set_status_text(&ui, format!("Could not save profile: {error}")),
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
            let current = studio_view_state(&ui).preview_page_number.max(1) as usize;
            show_preview_page(&ui, &preview_pages, current.saturating_sub(2));
        });
    }
    {
        let weak = ui.as_weak();
        let preview_pages = Arc::clone(&preview_pages);
        ui.on_next_preview_page(move || {
            let Some(ui) = weak.upgrade() else { return };
            let current = studio_view_state(&ui).preview_page_number.max(1) as usize;
            show_preview_page(&ui, &preview_pages, current);
        });
    }
    {
        let weak = ui.as_weak();
        ui.on_open_output(move || {
            let Some(ui) = weak.upgrade() else { return };
            let path = PathBuf::from(studio_config(&ui).output_path.to_string());
            match open_image(&path) {
                Ok(()) => set_status_text(&ui, "Opened rendered image"),
                Err(error) => set_status_text(&ui, error),
            }
        });
    }

    ui.run()?;
    Ok(())
}

fn apply_config(ui: &MainWindow, config: &Config) {
    update_studio_config(ui, |state| {
        state.page_width = config.page_width as f32;
        state.page_height = config.page_height as f32;
        state.margin_top = config.margin_top as f32;
        state.margin_bottom = config.margin_bottom as f32;
        state.margin_left = config.margin_left as f32;
        state.margin_right = config.margin_right as f32;
        state.letter_spacing = config.letter_spacing;
        state.line_spacing = config.line_spacing;
        state.font_size = config.font_size;
        state.heading_size = config.heading_size;
        state.stamp_size = config.stamp_size;
        state.jitter_px = config.jitter_px;
        state.blur_sigma = config.blur_sigma;
        state.ink_opacity = config.ink_opacity;
        state.default_font = config.default_font.display().to_string().into();
        state.heading_font = config.heading_font.display().to_string().into();
        state.stamp_font = config.stamp_font.display().to_string().into();
        state.paper_albedo = config.paper_albedo.display().to_string().into();
        state.paper_normal = config.paper_normal.display().to_string().into();
        state.paper_roughness = config.paper_roughness.display().to_string().into();
        state.redaction_marker = config.redaction_marker.clone().into();
        state.stamp_marker = config.stamp_marker.clone().into();
        true
    });
}

fn config_from_ui(ui: &MainWindow) -> Result<Config, String> {
    let state = studio_config(ui);
    let page_width = state.page_width.round().max(1.0) as u32;
    let page_height = state.page_height.round().max(1.0) as u32;
    let margin_top = state.margin_top.round().max(0.0) as u32;
    let margin_bottom = state.margin_bottom.round().max(0.0) as u32;
    let margin_left = state.margin_left.round().max(0.0) as u32;
    let margin_right = state.margin_right.round().max(0.0) as u32;
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
        letter_spacing: state.letter_spacing.max(1.0),
        line_spacing: state.line_spacing.max(1.0),
        default_font: state.default_font.to_string().into(),
        heading_font: state.heading_font.to_string().into(),
        stamp_font: state.stamp_font.to_string().into(),
        font_size: state.font_size.max(1.0),
        heading_size: state.heading_size.max(1.0),
        stamp_size: state.stamp_size.max(1.0),
        jitter_px: state.jitter_px.max(0.0),
        blur_sigma: state.blur_sigma.max(0.1),
        ink_opacity: state.ink_opacity.clamp(0.0, 1.0),
        redaction_marker: state.redaction_marker.to_string(),
        stamp_marker: state.stamp_marker.to_string(),
        paper_albedo: state.paper_albedo.to_string().into(),
        paper_normal: state.paper_normal.to_string().into(),
        paper_roughness: state.paper_roughness.to_string().into(),
    })
}

fn refresh_previews(ui: &MainWindow) {
    let state = studio_config(ui);
    let albedo_preview = load_image_preview(state.paper_albedo.as_ref());
    let normal_preview = load_image_preview(state.paper_normal.as_ref());
    let roughness_preview = load_image_preview(state.paper_roughness.as_ref());
    let default_font_preview = load_font_preview(state.default_font.as_ref());
    let heading_font_preview = load_font_preview(state.heading_font.as_ref());
    let stamp_font_preview = load_font_preview(state.stamp_font.as_ref());
    update_studio_view_state(ui, |view| {
        view.albedo_preview = albedo_preview;
        view.normal_preview = normal_preview;
        view.roughness_preview = roughness_preview;
        view.default_font_preview = default_font_preview;
        view.heading_font_preview = heading_font_preview;
        view.stamp_font_preview = stamp_font_preview;
    });
}

fn studio_config(ui: &MainWindow) -> StudioConfig {
    ui.global::<StudioState>().get_config()
}

fn studio_view_state(ui: &MainWindow) -> StudioViewState {
    ui.global::<StudioState>().get_view()
}

fn update_studio_view_state(ui: &MainWindow, update: impl FnOnce(&mut StudioViewState)) {
    let state = ui.global::<StudioState>();
    let mut view = state.get_view();
    update(&mut view);
    state.set_view(view);
}

fn set_status_text(ui: &MainWindow, text: impl Into<slint::SharedString>) {
    update_studio_view_state(ui, |state| state.status_text = text.into());
}

fn update_studio_config(ui: &MainWindow, update: impl FnOnce(&mut StudioConfig) -> bool) -> bool {
    let state = ui.global::<StudioState>();
    let mut config = state.get_config();
    if !update(&mut config) {
        return false;
    }
    state.set_config(config);
    true
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
        "ABCDEFG abcdefg 0123456789",
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
            set_status_text(&ui, error);
            return;
        }
    };
    let state = studio_config(&ui);
    let input_path = PathBuf::from(state.input_path.to_string());
    let output_path = PathBuf::from(state.output_path.to_string());
    update_studio_view_state(&ui, |state| state.working = true);
    set_status_text(
        &ui,
        if preview {
            "Rendering reduced preview..."
        } else {
            "Rendering full-size image..."
        },
    );

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
            update_studio_view_state(&ui, |state| state.working = false);
            match result {
                Ok((pages, status)) => {
                    if let Ok(mut stored_pages) = preview_pages.lock() {
                        *stored_pages = pages;
                    }
                    show_preview_page(&ui, &preview_pages, 0);
                    update_studio_view_state(&ui, |state| state.has_preview = true);
                    set_status_text(&ui, status);
                }
                Err(error) => set_status_text(&ui, format!("Render failed: {error}")),
            }
        });
    });
}

fn show_preview_page(ui: &MainWindow, pages: &Arc<Mutex<Vec<RgbaImage>>>, index: usize) {
    let Ok(pages) = pages.lock() else { return };
    let Some(page) = pages.get(index) else { return };
    let preview = image_to_slint(page);
    let page_number = index as i32 + 1;
    let page_count = pages.len() as i32;
    update_studio_view_state(ui, |state| {
        state.result_preview = preview;
        state.preview_page_number = page_number;
        state.preview_page_count = page_count;
        state.preview_page_label = format!("Page {} / {}", page_number, page_count).into();
    });
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
