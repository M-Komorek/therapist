mod action;
mod app;
mod cli;
mod keymap;
mod log_file;
mod logging;
mod session;
mod tabs;
mod terminal;
mod ui;

use std::{io, process::ExitCode};

use clap::Parser;

use crate::{
    app::{App, EventEffect},
    cli::Cli,
    session::Session,
    terminal::Terminal,
};

#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error(transparent)]
    Logging(#[from] logging::LoggingError),
    #[error(transparent)]
    LogFile(#[from] log_file::LogFileError),
    #[error("terminal error: {0}")]
    Terminal(#[from] io::Error),
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            tracing::error!("{error}");
            eprintln!("therapist: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<(), AppError> {
    logging::init(&cli.log_file)?;
    tracing::info!(file = %cli.file.display(), "starting");
    let session = Session::open(&cli.file)?;
    tracing::info!(line_count = session.log_file.line_count(), "file indexed");
    terminal::install_panic_hook();
    let mut terminal = Terminal::enter()?;
    let mut app = App::new(session, terminal.size()?);
    run_loop(&mut terminal, &mut app)?;
    tracing::info!("exiting");
    Ok(())
}

fn run_loop(terminal: &mut Terminal, app: &mut App) -> io::Result<()> {
    draw(terminal, app)?;
    loop {
        match handle_waiting_events(terminal, app)? {
            EventEffect::Nothing => {}
            EventEffect::Redraw => draw(terminal, app)?,
            EventEffect::Suspend => {
                terminal.suspend()?;
                draw(terminal, app)?;
            }
            EventEffect::Quit => return Ok(()),
        }
    }
}

/// Handles the next event and every event already waiting behind it, so keys that arrive faster
/// than the screen is drawn (a held-down key) cost one draw instead of a backlog of draws.
fn handle_waiting_events(terminal: &Terminal, app: &mut App) -> io::Result<EventEffect> {
    let mut batch_effect = EventEffect::Nothing;
    loop {
        let effect = terminal
            .next_event()?
            .map_or(EventEffect::Nothing, |event| app.handle(event));
        match effect {
            EventEffect::Nothing => {}
            EventEffect::Redraw => batch_effect = EventEffect::Redraw,
            EventEffect::Suspend | EventEffect::Quit => return Ok(effect),
        }
        if !terminal.has_waiting_event()? {
            return Ok(batch_effect);
        }
    }
}

fn draw(terminal: &mut Terminal, app: &App) -> io::Result<()> {
    terminal.draw(|frame| ui::render(frame, app))
}
