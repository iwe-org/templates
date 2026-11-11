use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::limits::REV_CHARS;
use crate::paths::is_safe_relative_path;
use crate::validate::is_valid_id;

pub const REPOSITORY_PREFIX: &str = "https://github.com/";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistryEntry {
    pub repository: String,
    pub rev: String,
    pub version: String,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub featured: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Registry {
    pub entries: BTreeMap<String, RegistryEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EntryChange {
    pub id: String,
    #[serde(flatten)]
    pub entry: RegistryEntry,
    pub previous: Option<RegistryEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    Invalid(String),
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RegistryError::Invalid(message) => {
                write!(f, "templates.toml does not parse: {message}")
            }
        }
    }
}

impl std::error::Error for RegistryError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryProblem {
    Id { id: String },
    Repository { id: String, repository: String },
    Rev { id: String, rev: String },
    Version { id: String, version: String },
    Path { id: String, path: String },
}

impl fmt::Display for RegistryProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RegistryProblem::Id { id } => write!(
                f,
                "[{id}]: the id must be lowercase kebab-case, 3 to 40 characters"
            ),
            RegistryProblem::Repository { id, repository } => write!(
                f,
                "[{id}]: repository `{repository}` must be https://github.com/<owner>/<name>"
            ),
            RegistryProblem::Rev { id, rev } => write!(
                f,
                "[{id}]: rev `{rev}` must be a full 40-character lowercase commit SHA, not a branch or a tag"
            ),
            RegistryProblem::Version { id, version } => {
                write!(f, "[{id}]: version `{version}` is not a semver version")
            }
            RegistryProblem::Path { id, path } => write!(
                f,
                "[{id}]: path `{path}` must be empty or a relative path without `..`"
            ),
        }
    }
}

impl Registry {
    pub fn parse(text: &str) -> Result<Registry, RegistryError> {
        toml::from_str(text).map_err(|error| RegistryError::Invalid(error.message().to_string()))
    }

    pub fn problems(&self) -> Vec<RegistryProblem> {
        let mut problems = Vec::new();
        for (id, entry) in &self.entries {
            if !is_valid_id(id) {
                problems.push(RegistryProblem::Id { id: id.clone() });
            }
            if !is_github_repository(&entry.repository) {
                problems.push(RegistryProblem::Repository {
                    id: id.clone(),
                    repository: entry.repository.clone(),
                });
            }
            if !is_full_rev(&entry.rev) {
                problems.push(RegistryProblem::Rev {
                    id: id.clone(),
                    rev: entry.rev.clone(),
                });
            }
            if semver::Version::parse(&entry.version).is_err() {
                problems.push(RegistryProblem::Version {
                    id: id.clone(),
                    version: entry.version.clone(),
                });
            }
            if !entry.path.is_empty() && !is_safe_relative_path(&entry.path) {
                problems.push(RegistryProblem::Path {
                    id: id.clone(),
                    path: entry.path.clone(),
                });
            }
        }
        problems
    }

    pub fn changes_since(&self, base: &Registry) -> Vec<EntryChange> {
        self.entries
            .iter()
            .filter(|(id, entry)| base.entries.get(*id) != Some(entry))
            .map(|(id, entry)| EntryChange {
                id: id.clone(),
                entry: entry.clone(),
                previous: base.entries.get(id).cloned(),
            })
            .collect()
    }

    pub fn all_entries(&self) -> Vec<EntryChange> {
        self.changes_since(&Registry::default())
    }
}

pub fn is_full_rev(rev: &str) -> bool {
    rev.len() == REV_CHARS
        && rev
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub fn is_github_repository(repository: &str) -> bool {
    let Some(rest) = repository.strip_prefix(REPOSITORY_PREFIX) else {
        return false;
    };
    let mut parts = rest.split('/');
    let (Some(owner), Some(name), None) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    let is_plain = |part: &str| {
        !part.is_empty()
            && !part.starts_with('.')
            && !part.starts_with('-')
            && part
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    };
    is_plain(owner) && is_plain(name) && !name.ends_with(".git")
}
