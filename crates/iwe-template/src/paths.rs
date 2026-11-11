use crate::limits::{
    AGENT_INSTRUCTION_FILES, ALLOWED_EXTENSIONS, CONFIG_PATH, GITIGNORE_PATH, LICENSE_FILES,
    MANIFEST_PATH, SCHEMAS_PREFIX, SKILLS_PREFIX,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PathClass {
    Allowed,
    DotPathNotAllowed,
    ExtensionNotAllowed,
    AgentInstructionsOutsideRoot,
}

pub fn is_safe_relative_path(path: &str) -> bool {
    !path.is_empty()
        && !path.contains('\\')
        && !path.chars().any(char::is_control)
        && path
            .split('/')
            .all(|component| !component.is_empty() && component != "." && component != "..")
}

pub(crate) fn file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

pub(crate) fn extension(path: &str) -> Option<&str> {
    let name = file_name(path);
    match name.rsplit_once('.') {
        Some((stem, extension)) if !stem.is_empty() => Some(extension),
        _ => None,
    }
}

pub(crate) fn has_extension(path: &str, expected: &str) -> bool {
    extension(path) == Some(expected)
}

pub(crate) fn is_agent_instruction_name(name: &str) -> bool {
    AGENT_INSTRUCTION_FILES
        .iter()
        .any(|reserved| reserved.eq_ignore_ascii_case(name))
}

pub(crate) fn is_root_agent_instruction(path: &str) -> bool {
    AGENT_INSTRUCTION_FILES.contains(&path)
}

pub(crate) fn is_license_path(path: &str) -> bool {
    !path.contains('/') && (path == "LICENSE" || path.starts_with("LICENSE."))
}

pub(crate) fn skill_name(path: &str) -> Option<&str> {
    let rest = path.strip_prefix(SKILLS_PREFIX)?;
    let (folder, _) = rest.split_once('/')?;
    Some(folder)
}

pub(crate) fn classify(path: &str) -> PathClass {
    let name = file_name(path);
    let has_dot_component = path.split('/').any(|component| component.starts_with('.'));
    if has_dot_component {
        if !is_allowed_dot_path(path) {
            return PathClass::DotPathNotAllowed;
        }
        if is_agent_instruction_name(name) {
            return PathClass::AgentInstructionsOutsideRoot;
        }
        return PathClass::Allowed;
    }
    if is_agent_instruction_name(name) {
        return if is_root_agent_instruction(path) {
            PathClass::Allowed
        } else {
            PathClass::AgentInstructionsOutsideRoot
        };
    }
    if path == MANIFEST_PATH || LICENSE_FILES.contains(&path) {
        return PathClass::Allowed;
    }
    match extension(path) {
        Some(found) if ALLOWED_EXTENSIONS.contains(&found) => PathClass::Allowed,
        _ => PathClass::ExtensionNotAllowed,
    }
}

fn is_allowed_dot_path(path: &str) -> bool {
    if path == CONFIG_PATH || path == GITIGNORE_PATH {
        return true;
    }
    if let Some(rest) = path.strip_prefix(SCHEMAS_PREFIX) {
        return !rest.contains('/') && !rest.starts_with('.') && has_extension(rest, "yaml");
    }
    if let Some(rest) = path.strip_prefix(SKILLS_PREFIX) {
        return rest.split('/').all(|component| !component.starts_with('.'))
            && has_extension(rest, "md");
    }
    false
}
