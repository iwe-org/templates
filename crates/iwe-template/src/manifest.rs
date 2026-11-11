use std::fmt;

use globset::{Glob, GlobBuilder, GlobMatcher};
use serde::{Deserialize, Serialize};

use crate::entry::{EntryKind, TemplateEntry};
use crate::limits::MANIFEST_PATH;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub format: u32,
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub authors: Vec<String>,
    pub license: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestError {
    Missing,
    Invalid(String),
}

impl fmt::Display for ManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ManifestError::Missing => write!(f, "template.toml is missing"),
            ManifestError::Invalid(message) => write!(f, "template.toml does not parse: {message}"),
        }
    }
}

impl std::error::Error for ManifestError {}

impl Manifest {
    pub fn parse(text: &str) -> Result<Manifest, ManifestError> {
        toml::from_str(text).map_err(|error| ManifestError::Invalid(error.message().to_string()))
    }

    pub fn from_entries(entries: &[TemplateEntry]) -> Result<Manifest, ManifestError> {
        let entry = entries
            .iter()
            .find(|entry| entry.path == MANIFEST_PATH && entry.kind == EntryKind::File)
            .ok_or(ManifestError::Missing)?;
        let text = std::str::from_utf8(&entry.bytes)
            .map_err(|error| ManifestError::Invalid(error.to_string()))?;
        Manifest::parse(text)
    }

    pub fn excludes(&self, path: &str) -> bool {
        ExcludeSet::new(&self.exclude).matches(path)
    }
}

pub(crate) struct ExcludeSet {
    matchers: Vec<GlobMatcher>,
}

impl ExcludeSet {
    pub fn new(patterns: &[String]) -> Self {
        ExcludeSet {
            matchers: patterns
                .iter()
                .filter_map(|pattern| exclude_glob(pattern))
                .map(|glob| glob.compile_matcher())
                .collect(),
        }
    }

    pub fn matches(&self, path: &str) -> bool {
        ancestors_and_self(path).any(|candidate| {
            self.matchers
                .iter()
                .any(|matcher| matcher.is_match(candidate))
        })
    }
}

pub(crate) fn exclude_glob(pattern: &str) -> Option<Glob> {
    let trimmed = pattern.strip_suffix('/').unwrap_or(pattern);
    if !crate::paths::is_safe_relative_path(trimmed) {
        return None;
    }
    GlobBuilder::new(trimmed)
        .literal_separator(true)
        .build()
        .ok()
}

fn ancestors_and_self(path: &str) -> impl Iterator<Item = &str> {
    path.match_indices('/')
        .map(move |(index, _)| &path[..index])
        .chain(std::iter::once(path))
}
