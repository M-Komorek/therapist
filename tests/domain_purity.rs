use std::{
    fs,
    path::{Path, PathBuf},
};

const DOMAIN_MODULES: [&str; 3] = ["session", "log_file", "tabs"];
const UI_CRATES: [&str; 2] = ["ratatui::", "crossterm::"];

#[test]
fn given_domain_modules_when_reading_their_sources_then_none_use_ratatui_or_crossterm() {
    for source_path in DOMAIN_MODULES.into_iter().flat_map(module_sources) {
        let source = fs::read_to_string(&source_path).unwrap();
        for ui_crate in UI_CRATES {
            assert!(
                !source.contains(ui_crate),
                "{} uses {ui_crate}",
                source_path.display()
            );
        }
    }
}

fn module_sources(module: &str) -> Vec<PathBuf> {
    let source_directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut source_paths = vec![source_directory.join(format!("{module}.rs"))];
    let submodule_directory = source_directory.join(module);
    if submodule_directory.is_dir() {
        source_paths.extend(files_in(&submodule_directory));
    }
    source_paths
}

fn files_in(directory: &Path) -> Vec<PathBuf> {
    fs::read_dir(directory)
        .unwrap()
        .flat_map(|entry| {
            let path = entry.unwrap().path();
            if path.is_dir() {
                files_in(&path)
            } else {
                vec![path]
            }
        })
        .collect()
}
