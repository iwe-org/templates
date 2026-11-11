use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::limits::FORMAT;
use crate::registry::Registry;

pub const DEFAULT_RELEASE_BASE_URL: &str = "https://github.com/iwe-org/templates/releases/download";
pub const INDEX_URL: &str =
    "https://github.com/iwe-org/templates/releases/download/index/index.json";
pub const ARCHIVE_EXTENSION: &str = "tar.gz";
pub const ENTRY_RECORD_FILE: &str = "entry.json";
pub const PREVIEW_FILE: &str = "preview.png";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Index {
    pub format: u32,
    pub generated: String,
    #[serde(default)]
    pub templates: Vec<IndexEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexEntry {
    pub id: String,
    pub format: u32,
    pub name: String,
    pub description: String,
    pub version: String,
    pub authors: Vec<String>,
    pub license: String,
    pub repository: String,
    pub rev: String,
    pub archive: String,
    pub sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview: Option<String>,
    pub files: u32,
    #[serde(default)]
    pub agent: AgentInfo,
    #[serde(default)]
    pub stars: u64,
    #[serde(default)]
    pub featured: bool,
    pub published: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentInfo {
    #[serde(default)]
    pub instructions: bool,
    #[serde(default)]
    pub skills: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntryRecord {
    pub id: String,
    pub format: u32,
    pub name: String,
    pub description: String,
    pub version: String,
    pub authors: Vec<String>,
    pub license: String,
    pub repository: String,
    pub rev: String,
    pub sha256: String,
    #[serde(default)]
    pub preview: bool,
    pub files: u32,
    #[serde(default)]
    pub agent: AgentInfo,
    pub published: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndexError {
    MissingRecord {
        id: String,
        version: String,
    },
    RevMismatch {
        id: String,
        version: String,
        registry: String,
        published: String,
    },
}

impl fmt::Display for IndexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IndexError::MissingRecord { id, version } => {
                write!(f, "no published entry record for {id} {version}")
            }
            IndexError::RevMismatch { id, version, registry, published } => write!(
                f,
                "{id} {version} was published at {published} but templates.toml pins {registry}; a new revision needs a new version"
            ),
        }
    }
}

impl std::error::Error for IndexError {}

pub fn release_name(id: &str, version: &str) -> String {
    format!("{id}-{version}")
}

pub fn archive_file_name(id: &str, version: &str) -> String {
    format!("{id}-{version}.{ARCHIVE_EXTENSION}")
}

pub fn assemble_index(
    registry: &Registry,
    records: &[EntryRecord],
    stars: &BTreeMap<String, u64>,
    release_base_url: &str,
    generated: &str,
) -> Result<Index, IndexError> {
    let base = release_base_url.trim_end_matches('/');
    let mut templates = Vec::with_capacity(registry.entries.len());
    for (id, entry) in &registry.entries {
        let record = records
            .iter()
            .find(|record| &record.id == id && record.version == entry.version)
            .ok_or_else(|| IndexError::MissingRecord {
                id: id.clone(),
                version: entry.version.clone(),
            })?;
        if record.rev != entry.rev {
            return Err(IndexError::RevMismatch {
                id: id.clone(),
                version: entry.version.clone(),
                registry: entry.rev.clone(),
                published: record.rev.clone(),
            });
        }
        let release = release_name(id, &entry.version);
        templates.push(IndexEntry {
            id: id.clone(),
            format: record.format,
            name: record.name.clone(),
            description: record.description.clone(),
            version: record.version.clone(),
            authors: record.authors.clone(),
            license: record.license.clone(),
            repository: entry.repository.clone(),
            rev: entry.rev.clone(),
            archive: format!("{base}/{release}/{}", archive_file_name(id, &entry.version)),
            sha256: record.sha256.clone(),
            preview: record
                .preview
                .then(|| format!("{base}/{release}/{PREVIEW_FILE}")),
            files: record.files,
            agent: record.agent.clone(),
            stars: stars.get(id).copied().unwrap_or(0),
            featured: entry.featured,
            published: record.published.clone(),
        });
    }
    Ok(Index {
        format: FORMAT,
        generated: generated.to_string(),
        templates,
    })
}
