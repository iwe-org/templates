use std::collections::BTreeSet;

use crate::entry::{is_github_path, EntryKind, TemplateEntry};
use crate::index::AgentInfo;
use crate::limits::{MANIFEST_PATH, PREVIEW_PATH};
use crate::manifest::{ExcludeSet, Manifest};
use crate::paths::{is_license_path, is_root_agent_instruction, skill_name};

pub fn is_installed(path: &str, manifest: &Manifest) -> bool {
    is_installed_with(path, &ExcludeSet::new(&manifest.exclude))
}

pub fn installable_files<'a>(
    entries: &'a [TemplateEntry],
    manifest: &Manifest,
) -> Vec<&'a TemplateEntry> {
    let excluded = ExcludeSet::new(&manifest.exclude);
    let mut files: Vec<&TemplateEntry> = entries
        .iter()
        .filter(|entry| entry.kind == EntryKind::File && is_installed_with(&entry.path, &excluded))
        .collect();
    files.sort_by(|a, b| a.path.cmp(&b.path));
    files
}

pub fn agent_info(installed: &[&TemplateEntry]) -> AgentInfo {
    let skills: BTreeSet<&str> = installed
        .iter()
        .filter_map(|entry| skill_name(&entry.path))
        .collect();
    AgentInfo {
        instructions: installed
            .iter()
            .any(|entry| is_root_agent_instruction(&entry.path)),
        skills: skills.into_iter().map(str::to_string).collect(),
    }
}

fn is_installed_with(path: &str, excluded: &ExcludeSet) -> bool {
    path != MANIFEST_PATH
        && path != PREVIEW_PATH
        && path != ".git"
        && !path.starts_with(".git/")
        && !is_github_path(path)
        && !is_license_path(path)
        && !excluded.matches(path)
}
