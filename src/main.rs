mod action;
mod app;
mod control;
mod domain;
mod jobs;
mod platform;
mod runtime;
mod ui;

use std::{io, process::ExitCode};

use clap::Parser;

use crate::{
    app::Model,
    domain::log_file::{LogFile, LogFileError},
    platform::{
        cli::Cli,
        logging::{self, LoggingError},
        terminal::{self, Terminal},
    },
};

#[derive(Debug, thiserror::Error)]
enum TherapistError {
    #[error(transparent)]
    Logging(#[from] LoggingError),
    #[error(transparent)]
    LogFile(#[from] LogFileError),
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

fn run(cli: Cli) -> Result<(), TherapistError> {
    logging::init(&cli.therapist_log_path)?;
    terminal::install_panic_hook();
    tracing::info!(browsed_file = %cli.browsed_file_path.display(), "starting");

    let log_file = LogFile::open(&cli.browsed_file_path)?;
    let mut terminal = Terminal::enter()?;
    runtime::run(&mut terminal, Model::new(log_file))?;

    tracing::info!("exiting");
    Ok(())
}
