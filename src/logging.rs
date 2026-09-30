use std::{
    fs::File,
    io,
    path::{Path, PathBuf},
    sync::Mutex,
};

use tracing::subscriber::SetGlobalDefaultError;
use tracing_subscriber::{
    EnvFilter,
    filter::{LevelFilter, ParseError},
};

const FILTER_VARIABLE: &str = "THERAPIST_LOG";

#[derive(Debug, thiserror::Error)]
pub enum LoggingError {
    #[error("invalid {FILTER_VARIABLE}: {0}")]
    InvalidFilter(#[from] ParseError),
    #[error("cannot create the log file {}: {source}", path.display())]
    CannotCreateFile { path: PathBuf, source: io::Error },
    #[error("cannot install the logger: {0}")]
    CannotInstall(#[from] SetGlobalDefaultError),
}

/// Sends tracing events to `log_path`, filtered by `THERAPIST_LOG` (EnvFilter syntax, `info` by default).
pub fn init(log_path: &Path) -> Result<(), LoggingError> {
    let filter = parse_filter(&std::env::var(FILTER_VARIABLE).unwrap_or_default())?;
    let log_file = File::create(log_path).map_err(|source| LoggingError::CannotCreateFile {
        path: log_path.to_path_buf(),
        source,
    })?;
    let subscriber = tracing_subscriber::fmt()
        .with_writer(Mutex::new(log_file))
        .with_ansi(false)
        .with_env_filter(filter)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}

fn parse_filter(directives: &str) -> Result<EnvFilter, ParseError> {
    EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .parse(directives)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_no_directives_when_parsing_filter_then_level_is_info() {
        let filter = parse_filter("").unwrap();

        assert_eq!(filter.max_level_hint(), Some(LevelFilter::INFO));
    }

    #[test]
    fn given_level_directive_when_parsing_filter_then_it_is_used() {
        let filter = parse_filter("debug").unwrap();

        assert_eq!(filter.max_level_hint(), Some(LevelFilter::DEBUG));
    }

    #[test]
    fn given_invalid_directive_when_parsing_filter_then_it_is_an_error() {
        assert!(parse_filter("therapist=loud").is_err());
    }
}
