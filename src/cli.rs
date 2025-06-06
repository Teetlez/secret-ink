use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Top Secret Document Generator
#[derive(Parser, Debug)]
#[command(
    name = "docgen",
    version = "1.0",
    author = "Your Name",
    about = "Generate classified-style documents from Markdown"
)]
pub struct Cli {
    /// Activate verbose mode
    #[arg(short, long)]
    pub verbose: bool,

    /// Subcommands
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Generate a PDF from a Markdown file
    Generate {
        /// Input Markdown file
        #[arg(short, long, value_name = "FILE")]
        input: PathBuf,

        /// Output PDF file
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,

        /// Apply redactions to the document
        #[arg(long)]
        redact: bool,

        /// Simulate scan artifacts
        #[arg(long)]
        simulate_scan: bool,
    },
    /// Preview the parsed Markdown content
    Preview {
        /// Input Markdown file
        #[arg(short, long, value_name = "FILE")]
        input: PathBuf,
    },
}
