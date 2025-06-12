use crate::config::Config;

#[derive(Debug)]
pub enum Block {
    Heading { level: u32, text: String },
    Paragraph(String),
    Redaction(String),
    Stamp(String),
}

pub fn parse_document(text: &str, cfg: &Config) -> Vec<Block> {
    let mut blocks = Vec::new();

    for line in text.lines() {
        if let Some(stripped) = line.strip_prefix('#') {
            // count #s for level
            let level = line.chars().take_while(|c| *c == '#').count() as u32;
            let text = stripped.trim().to_string();
            blocks.push(Block::Heading { level, text });
        } else if line.contains(&cfg.redaction_marker) {
            // capture redaction content
            let inner = line.trim_matches('=').to_string();
            blocks.push(Block::Redaction(inner));
        } else if line.contains(&cfg.stamp_marker) {
            let inner = line.trim_matches('!').to_string();
            blocks.push(Block::Stamp(inner));
        } else {
            blocks.push(Block::Paragraph(line.to_string()));
        }
    }

    blocks
}
