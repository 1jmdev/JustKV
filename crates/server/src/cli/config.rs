use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use betterkv_server::config::Config;

use crate::cli::runtime::{parse_config_content_into, tokenize_config_line};

pub(crate) fn load_config_file_into(path: &str, config: &mut Config) -> Result<(), String> {
    let mut visited = BTreeSet::new();
    load_file_recursive(Path::new(path), &mut visited, config)
}

fn load_file_recursive(
    path: &Path,
    visited: &mut BTreeSet<PathBuf>,
    config: &mut Config,
) -> Result<(), String> {
    let canonical = std::fs::canonicalize(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    if !visited.insert(canonical.clone()) {
        return Ok(());
    }

    let content = std::fs::read_to_string(&canonical)
        .map_err(|err| format!("failed to read {}: {err}", canonical.display()))?;

    for line in content.lines() {
        let Some(tokens) = tokenize_config_line(line) else {
            continue;
        };
        if tokens[0].eq_ignore_ascii_case("include") {
            for include in &tokens[1..] {
                let include_path = if Path::new(include).is_absolute() {
                    PathBuf::from(include)
                } else {
                    canonical
                        .parent()
                        .unwrap_or_else(|| Path::new("."))
                        .join(include)
                };
                load_file_recursive(&include_path, visited, config)?;
            }
        }
    }

    parse_config_content_into(&content, config)
}
