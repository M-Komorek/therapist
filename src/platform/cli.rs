use std::{fs::File, io, path::PathBuf};

use clap::{
    Parser,
    builder::{PathBufValueParser, TypedValueParser},
};

use super::paths;

/// Browse and search large log files.
#[derive(Debug, Parser)]
#[command(version, about)]
pub struct Cli {
    /// The log file to browse.
    #[arg(value_name = "FILE", value_parser = PathBufValueParser::new().try_map(readable_file))]
    pub browsed_file_path: PathBuf,

    /// Where therapist writes its own log.
    #[arg(
        long = "therapist-log",
        value_name = "PATH",
        default_value_os_t = paths::default_therapist_log_path(|name| std::env::var_os(name))
    )]
    pub therapist_log_path: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error("cannot open the file: {0}")]
    CannotOpen(io::Error),
    #[error("not a regular file")]
    NotARegularFile,
}

fn readable_file(path: PathBuf) -> Result<PathBuf, CliError> {
    let file = File::open(&path).map_err(CliError::CannotOpen)?;
    let metadata = file.metadata().map_err(CliError::CannotOpen)?;
    if !metadata.is_file() {
        return Err(CliError::NotARegularFile);
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use clap::{CommandFactory, error::ErrorKind};

    use super::*;

    fn manifest_dir() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn given_cli_definition_when_checked_then_it_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn given_existing_file_when_validating_then_it_is_accepted() {
        let path = manifest_dir().join("Cargo.toml");

        assert_eq!(readable_file(path.clone()).unwrap(), path);
    }

    #[test]
    fn given_missing_file_when_validating_then_it_cannot_be_opened() {
        let result = readable_file(PathBuf::from("/nope"));

        assert!(matches!(result, Err(CliError::CannotOpen(_))));
    }

    #[test]
    fn given_directory_when_validating_then_it_is_not_a_regular_file() {
        let result = readable_file(manifest_dir().to_path_buf());

        assert!(matches!(result, Err(CliError::NotARegularFile)));
    }

    #[test]
    fn given_missing_file_when_parsing_then_clap_reports_invalid_value() {
        let error = Cli::try_parse_from(["therapist", "/nope"]).unwrap_err();

        assert_eq!(error.kind(), ErrorKind::ValueValidation);
    }

    #[test]
    fn given_no_therapist_log_option_when_parsing_then_the_default_therapist_log_path_is_used() {
        let browsed_file_path = manifest_dir().join("Cargo.toml");

        let cli = Cli::try_parse_from([Path::new("therapist"), &browsed_file_path]).unwrap();

        assert_eq!(
            cli.therapist_log_path,
            paths::default_therapist_log_path(|name| std::env::var_os(name))
        );
    }

    #[test]
    fn given_therapist_log_option_when_parsing_then_it_overrides_the_default() {
        let browsed_file_path = manifest_dir().join("Cargo.toml");
        let arguments = [
            Path::new("therapist"),
            &browsed_file_path,
            Path::new("--therapist-log"),
            Path::new("/tmp/custom.log"),
        ];

        let cli = Cli::try_parse_from(arguments).unwrap();

        assert_eq!(cli.therapist_log_path, PathBuf::from("/tmp/custom.log"));
    }
}
