use std::{fs::File, io, path::PathBuf};

use clap::{
    Parser,
    builder::{PathBufValueParser, TypedValueParser},
};

/// Browse and search large log files.
#[derive(Debug, Parser)]
#[command(version, about)]
pub struct Cli {
    /// The log file to open.
    #[arg(value_name = "FILE", value_parser = PathBufValueParser::new().try_map(readable_file))]
    pub file: PathBuf,

    /// Where the app writes its own log.
    #[arg(long, value_name = "PATH", default_value_os_t = default_log_file())]
    pub log_file: PathBuf,
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

fn default_log_file() -> PathBuf {
    std::env::temp_dir().join("therapist.log")
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
    fn given_no_log_file_option_when_parsing_then_log_goes_to_temp_dir() {
        let file = manifest_dir().join("Cargo.toml");

        let cli = Cli::try_parse_from([Path::new("therapist"), &file]).unwrap();

        assert_eq!(cli.log_file, std::env::temp_dir().join("therapist.log"));
    }

    #[test]
    fn given_log_file_option_when_parsing_then_it_overrides_the_default() {
        let file = manifest_dir().join("Cargo.toml");
        let arguments = [
            Path::new("therapist"),
            &file,
            Path::new("--log-file"),
            Path::new("/tmp/custom.log"),
        ];

        let cli = Cli::try_parse_from(arguments).unwrap();

        assert_eq!(cli.log_file, PathBuf::from("/tmp/custom.log"));
    }
}
