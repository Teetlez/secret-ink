pub mod renderer {
    use super::parser::{Document, DocumentElement};
    use std::path::Path;

    use image::{Rgb, RgbImage};
    use imageproc::drawing::{draw_filled_rect_mut, draw_text_mut};
    use imageproc::rect::Rect;
    use rand::Rng;
    use rusttype::{Font, Scale, point};

    /// Parameters that govern layout and styling
    pub struct RenderParams {
        pub page_width: u32,
        pub page_height: u32,
        pub margin: u32,
        pub font_path: String,
        pub font_size: f32,
        pub line_spacing: f32,
        pub redaction_color: [u8; 3],
        pub text_color: [u8; 3],
        pub background_color: [u8; 3],
        pub jitter_max: i32, // maximum pixel jitter per glyph (±)
    }

    pub struct Renderer {
        params: RenderParams,
        font: Font<'static>,
    }

    impl Renderer {
        /// Initialize the renderer by loading the font
        pub fn new(params: RenderParams) -> Self {
            let font_data = std::fs::read(&params.font_path).expect("Failed to read font file");
            let font = Font::try_from_vec(font_data).expect("Error loading font data");

            Renderer { params, font }
        }

        /// Renders a complete Document into an RgbImage buffer
        pub fn render_document(&mut self, doc: &Document) -> RgbImage {
            // 1. Create a blank page
            let mut image = RgbImage::from_pixel(
                self.params.page_width,
                self.params.page_height,
                Rgb(self.params.background_color),
            );

            // 2. Set up text layout state
            let scale = Scale::uniform(self.params.font_size);
            let v_metrics = self.font.v_metrics(scale);
            let line_height = (v_metrics.ascent - v_metrics.descent) * self.params.line_spacing;
            let mut cursor_x = self.params.margin as i32;
            let mut cursor_y = (self.params.margin as f32 + v_metrics.ascent) as i32;

            // 3. Draw each element
            for elem in &doc.elements {
                match elem {
                    DocumentElement::Text(text) => {
                        self.draw_text_wrapped(
                            &mut image,
                            text,
                            scale,
                            &mut cursor_x,
                            &mut cursor_y,
                            line_height as i32,
                        );
                    }
                    DocumentElement::Redaction(redtext) => {
                        self.draw_redaction(
                            &mut image,
                            redtext,
                            scale,
                            &mut cursor_x,
                            &mut cursor_y,
                            line_height as i32,
                        );
                    }
                    DocumentElement::Image(path_buf) => {
                        self.draw_embedded_image(&mut image, path_buf, cursor_x, cursor_y);
                        // Advance cursor below image (assuming a fixed height or reading actual image height)
                        if let Ok(img) = image::open(path_buf) {
                            let (_w, h) = img.dimensions();
                            cursor_y += h as i32 + 20; // some padding
                            cursor_x = self.params.margin as i32;
                        }
                    }
                    DocumentElement::ParagraphBreak => {
                        // Move cursor down by one line
                        cursor_x = self.params.margin as i32;
                        cursor_y += line_height as i32;
                    }
                }

                // If we exceed page height, we could (TODO) start a new page or stop.
                if cursor_y as u32 > self.params.page_height - self.params.margin {
                    break;
                }
            }

            image
        }

        /// Draws a run of text, wrapping words at right margin, with per-glyph jitter
        fn draw_text_wrapped(
            &self,
            image: &mut RgbImage,
            text: &str,
            scale: Scale,
            cursor_x: &mut i32,
            cursor_y: &mut i32,
            line_height: i32,
        ) {
            let max_x = (self.params.page_width - self.params.margin) as i32;
            let space_width = self.font.glyph(' ').scaled(scale).h_metrics().advance_width as i32;

            // Break text into words
            for word in text.split_whitespace() {
                let mut word_width = 0;
                // Measure word width (approximate by summing glyph advances)
                for ch in word.chars() {
                    let glyph = self.font.glyph(ch).scaled(scale);
                    let h_metrics = glyph.h_metrics();
                    word_width += h_metrics.advance_width as i32;
                }
                // If word doesn't fit on this line, wrap
                if *cursor_x + word_width >= max_x {
                    *cursor_x = self.params.margin as i32;
                    *cursor_y += line_height;
                }

                // Draw each character with random jitter
                for ch in word.chars() {
                    let mut rng = rand::thread_rng();
                    let jitter_x = rng.gen_range(-self.params.jitter_max..=self.params.jitter_max);
                    let jitter_y = rng.gen_range(-self.params.jitter_max..=self.params.jitter_max);
                    let x = *cursor_x + jitter_x;
                    let y = *cursor_y + jitter_y;

                    draw_text_mut(
                        image,
                        Rgb(self.params.text_color),
                        x,
                        y - (scale.y as i32), // adjust baseline
                        scale,
                        &self.font,
                        &ch.to_string(),
                    );

                    // Advance cursor by glyph width
                    let glyph = self.font.glyph(ch).scaled(scale);
                    let advance = glyph.h_metrics().advance_width as i32;
                    *cursor_x += advance;
                }
                // After a word, add a space
                *cursor_x += space_width;
            }
        }

        /// Draws a redaction box covering the given text span
        fn draw_redaction(
            &self,
            image: &mut RgbImage,
            redaction_text: &str,
            scale: Scale,
            cursor_x: &mut i32,
            cursor_y: &mut i32,
            line_height: i32,
        ) {
            let max_x = (self.params.page_width - self.params.margin) as i32;

            // Measure text width to size the redaction box
            let mut total_width = 0;
            for ch in redaction_text.chars() {
                let glyph = self.font.glyph(ch).scaled(scale);
                total_width += glyph.h_metrics().advance_width as i32;
            }
            // If it doesn't fit on this line, wrap
            if *cursor_x + total_width >= max_x {
                *cursor_x = self.params.margin as i32;
                *cursor_y += line_height;
            }

            // Draw a filled black rectangle over the area
            let rect = Rect::at(*cursor_x, *cursor_y - (scale.y as i32))
                .of_size(total_width as u32, (scale.y as u32 + 4));
            draw_filled_rect_mut(image, rect, Rgb(self.params.redaction_color));

            // Advance cursor past the redacted span
            *cursor_x += total_width;
        }

        /// Embeds an image at the current cursor position
        fn draw_embedded_image(
            &self,
            image: &mut RgbImage,
            path_buf: &std::path::PathBuf,
            cursor_x: i32,
            cursor_y: i32,
        ) {
            if let Ok(subimg) = image::open(path_buf) {
                let subimg = subimg.to_rgb8();
                let (w, h) = subimg.dimensions();
                // Simple: copy pixels directly
                image::GenericImage::copy_from(
                    &mut image,
                    &subimg,
                    cursor_x as u32,
                    cursor_y as u32,
                )
                .unwrap_or(());
            }
        }
    }
}
