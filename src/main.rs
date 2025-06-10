use cosmic_text::BorrowedWithFontSystem;
use cosmic_text::CacheKeyFlags;
use cosmic_text::Color;
use cosmic_text::Editor;
use cosmic_text::FeatureTag;
use cosmic_text::FontFeatures;
use cosmic_text::Shaping;
use cosmic_text::Style;
use cosmic_text::{
    Action, Attrs, Buffer, Edit, Family, FontSystem, Metrics, Motion, SwashCache, Weight,
};
use std::fs;
use std::io;
use std::path::Path;
use std::{num::NonZeroU32, rc::Rc, slice};
use tiny_skia::{Paint, PixmapMut, Rect, Transform};
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, Event, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
    window::WindowBuilder,
};

fn set_buffer_text(buffer: &mut BorrowedWithFontSystem<'_, Buffer>) {
    let features = FontFeatures::new()
        .enable(FeatureTag::DISCRETIONARY_LIGATURES)
        .enable(FeatureTag::CONTEXTUAL_LIGATURES)
        .enable(FeatureTag::STANDARD_LIGATURES)
        .enable(FeatureTag::CONTEXTUAL_ALTERNATES)
        .clone();
    let attrs = Attrs::new().font_features(features);

    let spans: &[(&str, Attrs)] = &[
        (
            "Rollerscript-Smooth: The Quick Brown Fox Jumps Over the Lazy Dog\n",
            attrs.clone().family(Family::Name("Rollerscript-Smooth")),
        ),
        (
            "Rollerscript-Rough: The Quick Brown Fox Jumps Over the Lazy Dog\n",
            attrs.clone().family(Family::Name("Rollerscript-Rough")),
        ),
        (
            "Olicana-Fine: The Quick Brown Fox Jumps Over the Lazy Dog\n",
            attrs.clone().family(Family::Name("Olicana-Fine")),
        ),
        (
            "Olicana-Smooth: The Quick Brown Fox Jumps Over the Lazy Dog\n",
            attrs.clone().family(Family::Name("Olicana-Smooth")),
        ),
        (
            "Olicana-Rough: The Quick Brown Fox Jumps Over the Lazy Dog\n",
            attrs.clone().family(Family::Name("Olicana-Rough")),
        ),
        (
            "Pecita: The Quick Brown Fox Jumps Over the Lazy Dog\n",
            attrs.clone().family(Family::Name("Pecita")),
        ),
        (
            "Darkwoman: The Quick Brown Fox Jumps Over the Lazy Dog\n",
            attrs.clone().family(Family::Name("Darkwoman")),
        ),
        (
            "Evas Signatures: The Quick Brown Fox Jumps Over the Lazy Dog\n",
            attrs.clone().family(Family::Name("Evas Signatures")),
        ),
        (
            "tudy1311: The Quick Brown Fox Jumps Over the Lazy Dog\n",
            attrs.clone().family(Family::Name("tudy1311")),
        ),
        (
            "Thi: The Quick Brown Fox Jumps Over the Lazy Dog\n",
            attrs.clone().family(Family::Name("Thi")),
        ),
        (
            "Segoe Script: The Quick Brown Fox Jumps Over the Lazy Dog\n",
            attrs.clone().family(Family::Name("Segoe Script")),
        ),
        (
            "Trixie-Plain: The Quick Brown Fox Jumps Over the Lazy Dog\n",
            attrs.clone().family(Family::Name("Trixie-Plain")),
        ),
        (
            "Caslon Antique: The Quick Brown Fox Jumps Over the Lazy Dog\n",
            attrs.clone().family(Family::Name("Caslon Antique")),
        ),
        (
            "Courier New: The Quick Brown Fox Jumps Over the Lazy Dog\n",
            attrs.clone().family(Family::Name("Courier New")),
        ),
        (
            "Wormhole Type: The Quick Brown Fox Jumps Over the Lazy Dog\n",
            attrs.clone().family(Family::Name("Wormhole Type")),
        ),
        (
            "Pica: The Quick Brown Fox Jumps Over the Lazy Dog\n",
            attrs.clone().family(Family::Name("Pica")),
        ),
        (
            "JMH Typewriter: The Quick Brown Fox Jumps Over the Lazy Dog\n",
            attrs.clone().family(Family::Name("JMH Typewriter")),
        ),
        (
            "ELEGANT TYPEWRITER: The Quick Brown Fox Jumps Over the Lazy Dog\n",
            attrs.clone().family(Family::Name("ELEGANT TYPEWRITER")),
        ),
        (
            "Mom´sTypewriter: The Quick Brown Fox Jumps Over the Lazy Dog\n",
            attrs.clone().family(Family::Name("Mom´sTypewriter")),
        ),
        (
            "Silk Remington-SBold: The Quick Brown Fox Jumps Over the Lazy Dog\n",
            attrs.clone().family(Family::Name("Silk RemingtonSBold")),
        ),
        (
            "STAMPWRITER-KIT: The Quick Brown Fox Jumps Over the Lazy Dog\n",
            attrs.clone().family(Family::Name("STAMPWRITER-KIT")),
        ),
        (
            "Chomsky: The Quick Brown Fox Jumps Over the Lazy Dog\n",
            attrs.clone().family(Family::Name("Chomsky")),
        ),
        (
            "Lodeh Regular: The Quick Brown Fox Jumps Over the Lazy Dog\n",
            attrs.clone().family(Family::Name("Lodeh Regular")),
        ),
        (
            "OldNewspaperTypes: The Quick Brown Fox Jumps Over the Lazy Dog\n",
            attrs.clone().family(Family::Name("OldNewspaperTypes")),
        ),
        (
            "Palatino Linotype: The Quick Brown Fox Jumps Over the Lazy Dog\n",
            attrs.clone().family(Family::Name("Palatino Linotype")),
        ),
    ];

    buffer.set_rich_text(
        spans.iter().map(|(text, attrs)| (*text, attrs.clone())),
        &attrs,
        Shaping::Advanced,
        None,
    );
}

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    let window = Rc::new(WindowBuilder::new().build(&event_loop).unwrap());
    let context = softbuffer::Context::new(window.clone()).unwrap();
    let mut surface = softbuffer::Surface::new(&context, window.clone()).unwrap();

    let fonts = load_fonts_in_dir(&Path::new("./fonts")).unwrap();
    let mut font_system = FontSystem::new_with_fonts(fonts);
    let mut swash_cache = SwashCache::new();
    font_system.db().faces().for_each(|f| {
        println!(
            "Name: {:?} [{:?}] [{:?}]",
            f.post_script_name, f.families, f.source
        )
    });
    let mut display_scale = window.scale_factor() as f32;
    let metrics = Metrics::new(32.0, 44.0);
    let mut editor = Editor::new(Buffer::new_empty(metrics.scale(display_scale)));
    let mut editor = editor.borrow_with(&mut font_system);
    editor.with_buffer_mut(|buffer| {
        buffer.set_size(
            Some(window.inner_size().width as f32),
            Some(window.inner_size().height as f32),
        )
    });
    editor.with_buffer_mut(set_buffer_text);

    let mut ctrl_pressed = false;
    let mut mouse_x = 0.0;
    let mut mouse_y = 0.0;
    let mut mouse_left = ElementState::Released;
    let mut unapplied_scroll_delta = 0.0;
    //#F6EFE4
    let bg_color = tiny_skia::Color::from_rgba8(0xE4, 0xEF, 0xF6, 0xFF);
    let font_color = Color::rgb(0x10, 0x10, 0x10);
    let cursor_color = Color::rgb(0x10, 0x10, 0x10);
    let selection_color = Color::rgba(0x10, 0x10, 0x10, 0x33);
    let selected_text_color = Color::rgb(0xA0, 0xA0, 0xFF);

    event_loop
        .run(|event, elwt| {
            elwt.set_control_flow(ControlFlow::Wait);

            let Event::WindowEvent { window_id, event } = event else {
                return;
            };

            match event {
                WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                    log::info!("Updated scale factor for {window_id:?}");

                    display_scale = scale_factor as f32;
                    editor
                        .with_buffer_mut(|buffer| buffer.set_metrics(metrics.scale(display_scale)));

                    window.request_redraw();
                }
                WindowEvent::RedrawRequested => {
                    let (width, height) = {
                        let size = window.inner_size();
                        (size.width, size.height)
                    };

                    surface
                        .resize(
                            NonZeroU32::new(width).unwrap(),
                            NonZeroU32::new(height).unwrap(),
                        )
                        .unwrap();

                    let mut surface_buffer = surface.buffer_mut().unwrap();
                    let surface_buffer_u8 = unsafe {
                        slice::from_raw_parts_mut(
                            surface_buffer.as_mut_ptr() as *mut u8,
                            surface_buffer.len() * 4,
                        )
                    };
                    let mut pixmap =
                        PixmapMut::from_bytes(surface_buffer_u8, width, height).unwrap();
                    pixmap.fill(bg_color);

                    editor.with_buffer_mut(|buffer| {
                        buffer.set_size(Some(width as f32), Some(height as f32))
                    });

                    let mut paint = Paint {
                        anti_alias: true,
                        ..Default::default()
                    };
                    editor.shape_as_needed(true);

                    editor.draw(
                        &mut swash_cache,
                        font_color,
                        cursor_color,
                        selection_color,
                        selected_text_color,
                        |x, y, w, h, color| {
                            // Note: due to softbuffer and tiny_skia having incompatible internal color representations we swap
                            // the red and blue channels here
                            let jx = x as f32; // + (fastrand::f32());
                            let jy = y as f32; // + (fastrand::f32());
                            let jw = w as f32 + (fastrand::f32() * 0.5);
                            let jh = h as f32 + (fastrand::f32() * 0.5);
                            paint.set_color_rgba8(
                                color.b(),
                                color.g(),
                                color.r(),
                                fastrand::u8(((color.a() as f32 * 0.65) as u8)..=color.a()),
                            );
                            pixmap.fill_rect(
                                Rect::from_xywh(jx, jy, jw, jh).unwrap(),
                                &paint,
                                Transform::identity(),
                                None,
                            );
                        },
                    );

                    surface_buffer.present().unwrap();
                }
                WindowEvent::ModifiersChanged(modifiers) => {
                    ctrl_pressed = modifiers.state().control_key()
                }
                WindowEvent::KeyboardInput { event, .. } => {
                    let KeyEvent {
                        logical_key, state, ..
                    } = event;

                    if state.is_pressed() {
                        match logical_key {
                            Key::Named(NamedKey::ArrowLeft) => {
                                editor.action(Action::Motion(Motion::Left))
                            }
                            Key::Named(NamedKey::ArrowRight) => {
                                editor.action(Action::Motion(Motion::Right))
                            }
                            Key::Named(NamedKey::ArrowUp) => {
                                editor.action(Action::Motion(Motion::Up))
                            }
                            Key::Named(NamedKey::ArrowDown) => {
                                editor.action(Action::Motion(Motion::Down))
                            }
                            Key::Named(NamedKey::Home) => {
                                editor.action(Action::Motion(Motion::Home))
                            }
                            Key::Named(NamedKey::End) => editor.action(Action::Motion(Motion::End)),
                            Key::Named(NamedKey::PageUp) => {
                                editor.action(Action::Motion(Motion::PageUp))
                            }
                            Key::Named(NamedKey::PageDown) => {
                                editor.action(Action::Motion(Motion::PageDown))
                            }
                            Key::Named(NamedKey::Escape) => editor.action(Action::Escape),
                            Key::Named(NamedKey::Enter) => editor.action(Action::Enter),
                            Key::Named(NamedKey::Backspace) => editor.action(Action::Backspace),
                            Key::Named(NamedKey::Delete) => editor.action(Action::Delete),
                            Key::Named(key) => {
                                if let Some(text) = key.to_text() {
                                    for c in text.chars() {
                                        editor.action(Action::Insert(c));
                                    }
                                }
                            }
                            Key::Character(text) => {
                                if !ctrl_pressed {
                                    for c in text.chars() {
                                        editor.action(Action::Insert(c));
                                    }
                                }
                            }
                            _ => {}
                        }
                        window.request_redraw();
                    }
                }
                WindowEvent::CursorMoved {
                    device_id: _,
                    position,
                } => {
                    // Update saved mouse position for use when handling click events
                    mouse_x = position.x;
                    mouse_y = position.y;

                    // Implement dragging
                    if mouse_left.is_pressed() {
                        // Execute Drag editor action (update selection)
                        editor.action(Action::Drag {
                            x: position.x as i32,
                            y: position.y as i32,
                        });

                        // Scroll if cursor is near edge of window while dragging
                        if mouse_y <= 5.0 {
                            editor.action(Action::Scroll { lines: -1 });
                        } else if mouse_y - 5.0 >= window.inner_size().height as f64 {
                            editor.action(Action::Scroll { lines: 1 });
                        }

                        window.request_redraw();
                    }
                }
                WindowEvent::MouseInput {
                    device_id: _,
                    state,
                    button,
                } => {
                    if button == MouseButton::Left {
                        if state == ElementState::Pressed && mouse_left == ElementState::Released {
                            editor.action(Action::Click {
                                x: mouse_x /*- line_x*/ as i32,
                                y: mouse_y as i32,
                            });
                            window.request_redraw();
                        }
                        mouse_left = state;
                    }
                }
                WindowEvent::MouseWheel {
                    device_id: _,
                    delta,
                    phase: _,
                } => {
                    let line_delta = match delta {
                        MouseScrollDelta::LineDelta(_x, y) => y as i32,
                        MouseScrollDelta::PixelDelta(PhysicalPosition { x: _, y }) => {
                            unapplied_scroll_delta += y;
                            let line_delta = (unapplied_scroll_delta / 20.0).floor();
                            unapplied_scroll_delta -= line_delta * 20.0;
                            line_delta as i32
                        }
                    };
                    if line_delta != 0 {
                        editor.action(Action::Scroll { lines: -line_delta });
                    }
                    window.request_redraw();
                }
                WindowEvent::CloseRequested => {
                    //TODO: just close one window
                    elwt.exit();
                }
                _ => {}
            }
        })
        .unwrap();
}

/// Recursively walk `dir`, loading `.otf` fonts if present in each folder,
/// otherwise falling back to `.ttf` fonts.
///
/// # Arguments
///
/// * `db`  – the font database to populate
/// * `dir` – the directory to scan
fn load_fonts_in_dir(dir: &Path) -> io::Result<Vec<cosmic_text::fontdb::Source>> {
    // First, collect any .otf files in this directory
    let mut font_paths = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if ext.eq_ignore_ascii_case("otf") {
                    println!("Found OTF font: {}", path.display());
                    font_paths.push(cosmic_text::fontdb::Source::File(path));
                }
            }
        }
    }

    if font_paths.is_empty() {
        // Fallback: collect and load .ttf files if no .otf were found
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if ext.eq_ignore_ascii_case("ttf") {
                        println!("Found TTF font (fallback): {}", path.display());
                        font_paths.push(cosmic_text::fontdb::Source::File(path));
                    }
                }
            }
        }
    }

    // Recurse into subdirectories
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            load_fonts_in_dir(&path)?;
        }
    }

    Ok(font_paths)
}
