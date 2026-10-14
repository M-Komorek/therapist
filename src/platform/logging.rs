use std::{
    fs::{DirBuilder, File},
    io,
    os::unix::fs::DirBuilderExt,
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
    #[error("cannot create the therapist log's directory {}: {source}", path.display())]
    CannotCreateDirectory { path: PathBuf, source: io::Error },
    #[error("cannot create the therapist log {}: {source}", path.display())]
    CannotCreateFile { path: PathBuf, source: io::Error },
    #[error("cannot install the logger: {0}")]
    CannotInstall(#[from] SetGlobalDefaultError),
}

pub fn init(therapist_log_path: &Path) -> Result<(), LoggingError> {
    let filter = parse_filter(&std::env::var(FILTER_VARIABLE).unwrap_or_default())?;
    create_parent_directory(therapist_log_path)?;
    let therapist_log_file =
        File::create(therapist_log_path).map_err(|source| LoggingError::CannotCreateFile {
            path: therapist_log_path.to_path_buf(),
            source,
        })?;
    let subscriber = tracing_subscriber::fmt()
        .with_writer(Mutex::new(therapist_log_file))
        .with_ansi(false)
        .with_env_filter(filter)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}

fn create_parent_directory(therapist_log_path: &Path) -> Result<(), LoggingError> {
    let Some(directory) = therapist_log_path.parent() else {
        return Ok(());
    };

    DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(directory)
        .map_err(|source| LoggingError::CannotCreateDirectory {
            path: directory.to_path_buf(),
            source,
        })
}

fn parse_filter(directives: &str) -> Result<EnvFilter, ParseError> {
    EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .parse(directives)
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;

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

    #[test]
    fn given_missing_directories_when_creating_parent_directory_then_they_are_created_private() {
        let temp_directory = tempfile::tempdir().unwrap();
        let state_directory = temp_directory.path().join("state");
        let therapist_directory = state_directory.join("therapist");

        create_parent_directory(&therapist_directory.join("therapist.log")).unwrap();

        for directory in [state_directory, therapist_directory] {
            let permissions = directory.metadata().unwrap().permissions();
            assert_eq!(permissions.mode() & 0o777, 0o700);
        }
    }

    #[test]
    fn given_existing_directory_when_creating_parent_directory_then_it_succeeds() {
        let temp_directory = tempfile::tempdir().unwrap();

        let result = create_parent_directory(&temp_directory.path().join("therapist.log"));

        assert!(result.is_ok());
    }

    #[test]
    fn given_bare_file_name_when_creating_parent_directory_then_it_succeeds() {
        assert!(create_parent_directory(Path::new("therapist.log")).is_ok());
    }
}
