use crate::config::Config;

#[derive(Debug, Clone)]
pub enum Block {
    Heading { level: u32, text: String },
    Paragraph(String),
    Stamp(String),
}

pub fn parse_document(text: &str, cfg: &Config) -> Vec<Block> {
    let mut blocks = Vec::new();

    for line in text.lines() {
        if line.trim().starts_with('#') {
            // count #s for level
            let level = line.chars().take_while(|c| *c == '#').count() as u32;
            let text = line.trim_start_matches('#').to_string();
            blocks.push(Block::Heading { level, text });
        } else if line.contains(&cfg.stamp_marker) {
            let inner = line.trim_matches('!').to_string();
            blocks.push(Block::Stamp(inner));
        } else {
            let inner = if line.contains(&cfg.redaction_marker) {
                line.replace("==", "\u{20D2}").to_string()
            } else {
                line.to_string()
            };
            blocks.push(Block::Paragraph(inner.to_string()));
        }
    }

    blocks
}
