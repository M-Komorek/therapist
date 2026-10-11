use std::{
    fs,
    path::{Path, PathBuf},
};

/// The crate modules each layer may use besides itself. `main` may use anything, so it isn't
/// listed.
const MAY_USE: &[(&str, &[&str])] = &[
    ("domain", &[]),
    ("action", &["domain"]),
    ("platform", &[]),
    ("control", &["domain", "action"]),
    ("jobs", &["domain", "action"]),
    ("app", &["domain", "action", "control", "jobs"]),
    ("ui", &["domain", "action", "control", "jobs", "app"]),
    (
        "runtime",
        &[
            "domain", "action", "control", "jobs", "app", "ui", "platform",
        ],
    ),
];

const UI_CRATES: [&str; 2] = ["ratatui::", "crossterm::"];

#[test]
fn given_layer_sources_when_reading_their_crate_paths_then_each_uses_only_the_layers_it_may() {
    for (layer, allowed_layers) in MAY_USE {
        for source_path in layer_sources(layer) {
            let source = fs::read_to_string(&source_path).unwrap();
            for used_layer in crate_modules_used(&source) {
                assert!(
                    used_layer == *layer || allowed_layers.contains(&used_layer),
                    "{} uses crate::{used_layer}",
                    source_path.display()
                );
            }
        }
    }
}

#[test]
fn given_domain_sources_when_reading_them_then_none_use_ratatui_or_crossterm() {
    for source_path in layer_sources("domain") {
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

#[test]
fn given_grouped_nested_and_inline_crate_paths_when_reading_them_then_their_top_modules_are_found()
{
    let source = "use crate::{\n    app::{self, Model},\n    ui,\n};\nlet x = crate::domain::f();";

    assert_eq!(crate_modules_used(source), ["app", "ui", "domain"]);
}

fn layer_sources(layer: &str) -> Vec<PathBuf> {
    let source_directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut source_paths = vec![source_directory.join(format!("{layer}.rs"))];
    let layer_directory = source_directory.join(layer);
    if layer_directory.is_dir() {
        source_paths.extend(files_in(&layer_directory));
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

fn crate_modules_used(source: &str) -> Vec<&str> {
    source
        .split("crate::")
        .skip(1)
        .flat_map(top_level_modules)
        .filter(|module| !module.is_empty())
        .collect()
}

fn top_level_modules(path_after_crate: &str) -> Vec<&str> {
    match path_after_crate.strip_prefix('{') {
        Some(group) => group_items(group).into_iter().map(first_segment).collect(),
        None => vec![first_segment(path_after_crate)],
    }
}

fn group_items(group: &str) -> Vec<&str> {
    let mut items = Vec::new();
    let mut depth = 0;
    let mut item_start = 0;

    for (offset, character) in group.char_indices() {
        match character {
            '{' => depth += 1,
            '}' if depth == 0 => {
                items.push(&group[item_start..offset]);
                break;
            }
            '}' => depth -= 1,
            ',' if depth == 0 => {
                items.push(&group[item_start..offset]);
                item_start = offset + 1;
            }
            _ => {}
        }
    }
    items
}

fn first_segment(path: &str) -> &str {
    let path = path.trim_start();
    let segment_end = path
        .find(|character: char| !(character.is_alphanumeric() || character == '_'))
        .unwrap_or(path.len());
    &path[..segment_end]
}
