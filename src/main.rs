use clap::Parser;
use cli::Cli;

mod action;
mod app;
mod cli;
mod config;
mod data;
mod domain;
mod infra;
mod tui;
mod ui;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    infra::errors::init()?;
    infra::logging::init()?;

    let args = Cli::parse();
    args.validate_file_path()?;

    let mut app = app::App::new(args.tick_rate, args.frame_rate, args.file_path)?;
    app.run().await?;

    Ok(())
}
