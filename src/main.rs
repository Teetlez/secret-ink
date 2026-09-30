mod app;
mod cli;
mod config;
mod layout;
mod page;
mod parser;
mod pipeline;
mod renderer;

use clap::Parser;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    match cli::Cli::parse().command {
        Some(cli::Command::Render(args)) => cli::render(args),
        None => app::run(),
    }
}
