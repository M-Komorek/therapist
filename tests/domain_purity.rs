use std::{
    fs,
    path::{Path, PathBuf},
};

const UI_CRATES: [&str; 2] = ["ratatui::", "crossterm::"];

#[test]
fn given_domain_sources_when_reading_them_then_none_use_ratatui_or_crossterm() {
    for source_path in domain_sources() {
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

fn domain_sources() -> Vec<PathBuf> {
    let source_directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut source_paths = vec![source_directory.join("domain.rs")];
    source_paths.extend(files_in(&source_directory.join("domain")));
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
