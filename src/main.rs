mod app;
mod config;
mod layout;
mod page;
mod parser;
mod pipeline;
mod renderer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    app::run()
}
