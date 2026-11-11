use toml::{Table, Value};

use crate::entry::Source;
use crate::limits::{ALLOWED_CONFIG_KEYS, CONFIG_PATH, MIN_CONFIG_VERSION};
use crate::paths::is_safe_relative_path;
use crate::violation::Violation;

pub(crate) fn config_violations(sources: &[Source]) -> Vec<Violation> {
    let Some(config) = sources.iter().find(|source| source.path == CONFIG_PATH) else {
        return vec![Violation::ConfigMissing];
    };
    let table = match std::str::from_utf8(&config.bytes)
        .map_err(|error| error.to_string())
        .and_then(|text| {
            text.parse::<Table>()
                .map_err(|error| error.message().to_string())
        }) {
        Ok(table) => table,
        Err(message) => return vec![Violation::ConfigInvalid { message }],
    };
    let mut violations = Vec::new();
    for key in table.keys() {
        match key.as_str() {
            "library" => violations.push(Violation::ConfigLibraryTable),
            "commands" => violations.push(Violation::ConfigCommands),
            allowed if ALLOWED_CONFIG_KEYS.contains(&allowed) => {}
            _ => violations.push(Violation::ConfigKeyNotAllowed { key: key.clone() }),
        }
    }
    match table.get("version").and_then(Value::as_integer) {
        Some(version) if version >= MIN_CONFIG_VERSION => {}
        found => violations.push(Violation::ConfigVersion { found }),
    }
    violations.extend(action_violations(table.get("actions")));
    violations.extend(workspace_path_violation(table.get("workspace"), sources));
    violations
}

fn action_violations(actions: Option<&Value>) -> Vec<Violation> {
    let Some(actions) = actions else {
        return Vec::new();
    };
    let Some(actions) = actions.as_table() else {
        return vec![Violation::ConfigInvalid {
            message: "actions must be a table".to_string(),
        }];
    };
    let mut violations = Vec::new();
    for (name, action) in actions {
        let Some(action) = action.as_table() else {
            violations.push(Violation::ConfigInvalid {
                message: format!("actions.{name} must be a table"),
            });
            continue;
        };
        if action.get("type").and_then(Value::as_str) == Some("transform") {
            violations.push(Violation::ConfigTransformAction {
                action: name.clone(),
            });
        } else if action.contains_key("command") {
            violations.push(Violation::ConfigActionCommand {
                action: name.clone(),
            });
        }
    }
    violations
}

fn workspace_path_violation(workspace: Option<&Value>, sources: &[Source]) -> Option<Violation> {
    let workspace = workspace?;
    let Some(workspace) = workspace.as_table() else {
        return Some(Violation::ConfigInvalid {
            message: "workspace must be a table".to_string(),
        });
    };
    let path = workspace.get("path")?;
    let Some(path) = path.as_str() else {
        return Some(Violation::ConfigWorkspacePath {
            path: path.to_string(),
        });
    };
    if path.is_empty() {
        return None;
    }
    let folder = path.strip_suffix('/').unwrap_or(path);
    let prefix = format!("{folder}/");
    let is_contained = is_safe_relative_path(folder)
        && sources
            .iter()
            .any(|source| source.path.starts_with(&prefix));
    (!is_contained).then(|| Violation::ConfigWorkspacePath {
        path: path.to_string(),
    })
}
