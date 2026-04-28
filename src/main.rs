use clap::Parser;
use cli::Cli;

use crate::app::App;

mod action;
mod app;
mod cli;
mod component;
mod components;
mod config;
mod domain;
mod errors;
mod layout;
mod logging;
mod tui;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    crate::errors::init()?;
    crate::logging::init()?;

    let args = Cli::parse();
    args.validate_file_path()?;

    let mut app = App::new(args.tick_rate, args.frame_rate, args.file_path)?;
    app.run().await?;

    Ok(())
}
