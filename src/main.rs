mod cli;
mod logging;

use std::process::ExitCode;

use clap::Parser;

use crate::cli::Cli;

#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error(transparent)]
    Logging(#[from] logging::LoggingError),
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("therapist: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: &Cli) -> Result<(), AppError> {
    logging::init(&cli.log_file)?;
    tracing::info!(file = %cli.file.display(), "starting");
    tracing::info!("exiting");
    Ok(())
}
