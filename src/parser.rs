pub mod parser {
    use pulldown_cmark::{CowStr, Event, Parser, Tag};
    use std::path::PathBuf;

    /// A high-level representation of a document, as a sequence of elements.
    #[derive(Debug)]
    pub struct Document {
        pub elements: Vec<DocumentElement>,
    }

    /// Each element can be plain text, a redaction span, or an embedded image.
    #[derive(Debug)]
    pub enum DocumentElement {
        /// A run of normal text.
        Text(String),
        /// A span of text to redact (black box).
        Redaction(String),
        /// An embedded image with a path.
        Image(PathBuf),
        /// A paragraph break.
        ParagraphBreak,
    }

    /// Parses a Markdown-like string into our Document model.
    ///
    /// Custom syntax:
    ///   - Surround text to redact with `==like this==`
    ///   - Image syntax uses standard Markdown: `![alt](path/to/file.png)`
    pub fn parse_markdown(input: &str) -> Document {
        let mut elements = Vec::new();
        // Use pulldown_cmark to iterate over events
        let parser = Parser::new(input);

        // We accumulate text until a break or special event
        let mut text_buffer = String::new();

        for event in parser {
            match event {
                Event::Start(Tag::Image(_link_type, src, _title)) => {
                    // Flush any buffered text
                    if !text_buffer.trim().is_empty() {
                        elements.push(DocumentElement::Text(text_buffer.clone()));
                        text_buffer.clear();
                    }
                    // Treat the image src as an embedded image
                    elements.push(DocumentElement::Image(PathBuf::from(src.to_string())));
                    // After an image, we consider a paragraph break
                    elements.push(DocumentElement::ParagraphBreak);
                }
                Event::Text(text) => {
                    // Look for custom redaction markers: ==...==
                    let parts: Vec<&str> = text.split("==").collect();
                    for (i, part) in parts.iter().enumerate() {
                        if i % 2 == 0 {
                            // even index: normal text
                            text_buffer.push_str(part);
                        } else {
                            // odd index: this is a redaction span
                            if !text_buffer.trim().is_empty() {
                                elements.push(DocumentElement::Text(text_buffer.clone()));
                                text_buffer.clear();
                            }
                            elements.push(DocumentElement::Redaction(part.to_string()));
                        }
                    }
                }
                Event::SoftBreak | Event::HardBreak => {
                    if !text_buffer.trim().is_empty() {
                        elements.push(DocumentElement::Text(text_buffer.clone()));
                        text_buffer.clear();
                    }
                    elements.push(DocumentElement::ParagraphBreak);
                }
                _ => {
                    // We ignore other events (emphasis, headings, etc.) for now
                }
            }
        }

        // Flush leftover text
        if !text_buffer.trim().is_empty() {
            elements.push(DocumentElement::Text(text_buffer));
        }

        Document { elements }
    }
}
