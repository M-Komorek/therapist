use std::path::PathBuf;

use clap::Parser;

use crate::config::{get_config_dir, get_data_dir};

#[derive(Parser, Debug)]
#[command(author, version = version(), about)]
pub struct Cli {
    /// Tick rate, i.e. number of ticks per second
    #[arg(short, long, value_name = "FLOAT", default_value_t = 4.0)]
    pub tick_rate: f64,

    /// Frame rate, i.e. number of frames per second
    #[arg(short, long, value_name = "FLOAT", default_value_t = 60.0)]
    pub frame_rate: f64,

    /// Path to the log file
    #[arg(value_name = "FILE")]
    pub file_path: PathBuf,
}

impl Cli {
    pub fn validate_file_path(&self) -> color_eyre::Result<()> {
        if !self.file_path.exists() {
            return Err(color_eyre::eyre::eyre!(
                "File not found: {}",
                self.file_path.display()
            ));
        }

        std::fs::File::open(&self.file_path).map_err(|error| {
            color_eyre::eyre::eyre!("Cannot read file '{}': {}", self.file_path.display(), error)
        })?;

        Ok(())
    }
}

const VERSION_MESSAGE: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    "-",
    env!("VERGEN_GIT_DESCRIBE"),
    " (",
    env!("VERGEN_BUILD_DATE"),
    ")"
);

pub fn version() -> String {
    let author = clap::crate_authors!();

    let config_dir_path = get_config_dir().display().to_string();
    let data_dir_path = get_data_dir().display().to_string();

    format!(
        "\
{VERSION_MESSAGE}

Authors: {author}

Config directory: {config_dir_path}
Data directory: {data_dir_path}"
    )
}
