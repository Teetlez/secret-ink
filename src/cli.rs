use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};

use crate::{config::Config, pipeline};

#[derive(Parser)]
#[command(name = "secret-ink", about = "Generate a secret-ink document")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Render a Markdown document to an image.
    Render(RenderArgs),
}

#[derive(clap::Args)]
pub struct RenderArgs {
    /// TOML profile; relative asset paths are resolved from its directory.
    #[arg(long, default_value = "profile.toml")]
    profile: PathBuf,

    /// Markdown input document.
    #[arg(long, default_value = "input.md")]
    input: PathBuf,

    /// Rendered image destination.
    #[arg(long, default_value = "output.png")]
    output: PathBuf,
}

pub fn render(args: RenderArgs) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = Config::load_from(&args.profile)?;
    config.resolve_paths(args.profile.parent().unwrap_or(Path::new(".")));

    let image = pipeline::render_document(&config, &args.input, false)?;
    image.save(&args.output)?;
    let (width, height) = image.dimensions();
    println!(
        "Rendered {width} x {height} px to {}",
        args.output.display()
    );
    Ok(())
}
