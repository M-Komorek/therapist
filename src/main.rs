mod cli;
mod logging;
mod terminal;

use std::{io, process::ExitCode};

use clap::Parser;
use ratatui::crossterm::event::{KeyCode, KeyModifiers};

use crate::{
    cli::Cli,
    terminal::{Event, Terminal},
};

#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error(transparent)]
    Logging(#[from] logging::LoggingError),
    #[error("terminal error: {0}")]
    Terminal(#[from] io::Error),
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            tracing::error!("{error}");
            eprintln!("therapist: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: &Cli) -> Result<(), AppError> {
    logging::init(&cli.log_file)?;
    tracing::info!(file = %cli.file.display(), "starting");
    terminal::install_panic_hook();
    let mut terminal = Terminal::enter()?;
    run_loop(&mut terminal)?;
    tracing::info!("exiting");
    Ok(())
}

// Temporary, until more complex logic came it.
fn run_loop(terminal: &mut Terminal) -> io::Result<()> {
    loop {
        terminal.draw(|frame| {
            frame.render_widget("Ctrl-z suspends, any other key quits", frame.area())
        })?;
        match terminal.next_event()? {
            Some(Event::Key(key))
                if key.code == KeyCode::Char('z') && key.modifiers == KeyModifiers::CONTROL =>
            {
                terminal.suspend()?
            }
            Some(Event::Key(_)) => return Ok(()),
            Some(Event::Resize) | None => {}
        }
    }
}
