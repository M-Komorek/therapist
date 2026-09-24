mod action;
mod cli;
mod keymap;
mod logging;
mod terminal;

use std::{io, process::ExitCode};

use clap::Parser;

use crate::{
    action::{Action, AppAction},
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
        terminal.draw(|frame| frame.render_widget("q to quit, Ctrl-z to suspend", frame.area()))?;
        let Some(Event::Key(key)) = terminal.next_event()? else {
            continue;
        };
        match keymap::action_for(key) {
            Some(Action::App(AppAction::Quit)) => return Ok(()),
            Some(Action::App(AppAction::Suspend)) => terminal.suspend()?,
            #[cfg(debug_assertions)]
            Some(Action::App(AppAction::DebugPanic)) => panic!("debug panic key pressed"),
            None => {}
        }
    }
}
