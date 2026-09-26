mod action;
mod app;
mod cli;
mod keymap;
#[cfg_attr(not(test), expect(dead_code))]
mod log_file;
mod logging;
mod session;
#[cfg_attr(not(test), expect(dead_code))]
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
    let mut app = App::new(session);
    run_loop(&mut terminal, &mut app)?;
    tracing::info!("exiting");
    Ok(())
}

fn run_loop(terminal: &mut Terminal, app: &mut App) -> io::Result<()> {
    draw(terminal, app)?;
    loop {
        let Some(event) = terminal.next_event()? else {
            continue;
        };
        match app.handle(event) {
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

fn draw(terminal: &mut Terminal, app: &App) -> io::Result<()> {
    terminal.draw(|frame| ui::render(frame, app))
}
