use std::collections::HashSet;
use std::path::Path;

use crate::config::config_violations;
use crate::entry::{
    is_outside_template, read_sources, EntryKind, ReadError, Source, TemplateEntry,
};
use crate::limits::{
    ACCEPTED_LICENSES, CONFIG_PATH, FORMAT, MANIFEST_PATH, MAX_DESCRIPTION_CHARS, MAX_FILES,
    MAX_FILE_BYTES, MAX_ID_CHARS, MAX_NAME_CHARS, MAX_PREVIEW_BYTES, MAX_SCRIPT_LINE_CHARS,
    MAX_TOTAL_BYTES, MIN_ID_CHARS, PREVIEW_PATH,
};
use crate::manifest::{exclude_glob, ExcludeSet, Manifest};
use crate::paths::{classify, is_safe_relative_path, PathClass};
use crate::scripts::{long_lines, scripts_in};
use crate::violation::Violation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expected {
    pub id: String,
    pub version: String,
    pub previous_version: Option<String>,
}

pub fn validate_dir(dir: &Path, expected: Option<&Expected>) -> Result<Vec<Violation>, ReadError> {
    Ok(validate_sources(&read_sources(dir)?, expected))
}

pub fn validate_entries(entries: &[TemplateEntry], expected: Option<&Expected>) -> Vec<Violation> {
    let sources: Vec<Source> = entries
        .iter()
        .filter(|entry| !is_outside_template(&entry.path))
        .map(Source::from_entry)
        .collect();
    validate_sources(&sources, expected)
}

pub(crate) fn validate_sources(sources: &[Source], expected: Option<&Expected>) -> Vec<Violation> {
    let mut violations = Vec::new();
    let mut seen = HashSet::new();
    for source in sources {
        violations.extend(file_violations(source, &mut seen));
    }
    if sources.len() > MAX_FILES {
        violations.push(Violation::TooManyFiles {
            count: sources.len(),
        });
    }
    let total: u64 = sources.iter().map(|source| source.size).sum();
    if total > MAX_TOTAL_BYTES {
        violations.push(Violation::TotalTooLarge { size: total });
    }
    violations.extend(config_violations(sources));
    violations.extend(manifest_violations(sources, expected));
    violations
}

fn file_violations(source: &Source, seen: &mut HashSet<String>) -> Vec<Violation> {
    let path = source.path.clone();
    if !is_safe_relative_path(&path) {
        return vec![Violation::EscapingPath { path }];
    }
    match source.kind {
        EntryKind::Symlink => return vec![Violation::Symlink { path }],
        EntryKind::HardLink => return vec![Violation::HardLink { path }],
        EntryKind::Other => return vec![Violation::UnsupportedFileKind { path }],
        EntryKind::File => {}
    }
    let mut violations = Vec::new();
    if !seen.insert(path.to_lowercase()) {
        violations.push(Violation::DuplicatePath { path: path.clone() });
    }
    match classify(&path) {
        PathClass::Allowed => {}
        PathClass::DotPathNotAllowed => {
            violations.push(Violation::DotPathNotAllowed { path: path.clone() })
        }
        PathClass::ExtensionNotAllowed => {
            violations.push(Violation::ExtensionNotAllowed { path: path.clone() })
        }
        PathClass::AgentInstructionsOutsideRoot => {
            violations.push(Violation::AgentInstructionsOutsideRoot { path: path.clone() })
        }
    }
    if source.size > MAX_FILE_BYTES {
        violations.push(Violation::FileTooLarge {
            path: path.clone(),
            size: source.size,
        });
    } else if path == PREVIEW_PATH && source.size > MAX_PREVIEW_BYTES {
        violations.push(Violation::PreviewTooLarge { size: source.size });
    }
    for script in scripts_in(&path, &source.text()) {
        for (line, length) in long_lines(&script, MAX_SCRIPT_LINE_CHARS) {
            violations.push(Violation::ScriptLineTooLong {
                path: path.clone(),
                line,
                length,
            });
        }
    }
    violations
}

fn manifest_violations(sources: &[Source], expected: Option<&Expected>) -> Vec<Violation> {
    let Some(source) = sources
        .iter()
        .find(|source| source.path == MANIFEST_PATH && source.kind == EntryKind::File)
    else {
        return vec![Violation::ManifestMissing];
    };
    let manifest = match std::str::from_utf8(&source.bytes)
        .map_err(|error| error.to_string())
        .and_then(|text| {
            Manifest::parse(text).map_err(|error| match error {
                crate::manifest::ManifestError::Invalid(message) => message,
                crate::manifest::ManifestError::Missing => String::new(),
            })
        }) {
        Ok(manifest) => manifest,
        Err(message) => return vec![Violation::ManifestInvalid { message }],
    };
    let mut violations = Vec::new();
    if manifest.format != FORMAT {
        violations.push(Violation::ManifestFormat {
            found: manifest.format,
        });
    }
    if !is_valid_id(&manifest.id) {
        violations.push(Violation::ManifestId {
            id: manifest.id.clone(),
        });
    }
    if !is_single_line(&manifest.name) || manifest.name.chars().count() > MAX_NAME_CHARS {
        violations.push(Violation::ManifestName {
            name: manifest.name.clone(),
        });
    }
    if !is_single_line(&manifest.description)
        || manifest.description.chars().count() > MAX_DESCRIPTION_CHARS
    {
        violations.push(Violation::ManifestDescription {
            description: manifest.description.clone(),
        });
    }
    let version = parse_version(&manifest.version);
    if version.is_none() {
        violations.push(Violation::ManifestVersion {
            version: manifest.version.clone(),
        });
    }
    if manifest.authors.is_empty()
        || manifest
            .authors
            .iter()
            .any(|author| !is_single_line(author))
    {
        violations.push(Violation::ManifestAuthors);
    }
    if !ACCEPTED_LICENSES.contains(&manifest.license.as_str()) {
        violations.push(Violation::ManifestLicense {
            license: manifest.license.clone(),
        });
    }
    for pattern in &manifest.exclude {
        let keeps_config = exclude_glob(pattern).is_some()
            && !ExcludeSet::new(std::slice::from_ref(pattern)).matches(CONFIG_PATH);
        if !keeps_config {
            violations.push(Violation::ManifestExclude {
                pattern: pattern.clone(),
            });
        }
    }
    if let Some(expected) = expected {
        if manifest.id != expected.id {
            violations.push(Violation::RegistryIdMismatch {
                expected: expected.id.clone(),
                found: manifest.id.clone(),
            });
        }
        if manifest.version != expected.version {
            violations.push(Violation::RegistryVersionMismatch {
                expected: expected.version.clone(),
                found: manifest.version.clone(),
            });
        }
        let previous = expected.previous_version.as_deref();
        if let (Some(previous), Some(found)) = (previous, &version) {
            let has_increased = parse_version(previous).is_some_and(|previous| *found > previous);
            if !has_increased {
                violations.push(Violation::VersionNotIncreased {
                    previous: previous.to_string(),
                    found: manifest.version.clone(),
                });
            }
        }
    }
    violations
}

pub fn is_valid_id(id: &str) -> bool {
    (MIN_ID_CHARS..=MAX_ID_CHARS).contains(&id.len())
        && id.split('-').all(|word| {
            !word.is_empty()
                && word
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        })
        && id.starts_with(|c: char| c.is_ascii_lowercase())
}

fn is_single_line(text: &str) -> bool {
    !text.trim().is_empty() && text.trim() == text && !text.chars().any(char::is_control)
}

fn parse_version(version: &str) -> Option<semver::Version> {
    semver::Version::parse(version)
        .ok()
        .filter(|version| version.build.is_empty())
}
