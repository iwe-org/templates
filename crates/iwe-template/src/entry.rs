use std::fmt;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use crate::limits::{GITHUB_PREFIX, MAX_FILES, MAX_FILE_BYTES, MAX_TOTAL_BYTES};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    File,
    Symlink,
    HardLink,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateEntry {
    pub path: String,
    pub kind: EntryKind,
    pub bytes: Vec<u8>,
}

impl TemplateEntry {
    pub fn file(path: impl Into<String>, bytes: impl Into<Vec<u8>>) -> Self {
        TemplateEntry {
            path: path.into(),
            kind: EntryKind::File,
            bytes: bytes.into(),
        }
    }
}

#[derive(Debug)]
pub enum ReadError {
    NotADirectory(PathBuf),
    Io { path: PathBuf, source: io::Error },
    NonUtf8Path(PathBuf),
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReadError::NotADirectory(path) => write!(f, "{} is not a directory", path.display()),
            ReadError::Io { path, source } => write!(f, "{}: {source}", path.display()),
            ReadError::NonUtf8Path(path) => {
                write!(f, "{} is not a UTF-8 path", path.display())
            }
        }
    }
}

impl std::error::Error for ReadError {}

#[derive(Debug, Clone)]
pub(crate) struct Source {
    pub path: String,
    pub kind: EntryKind,
    pub size: u64,
    pub bytes: Vec<u8>,
}

impl Source {
    pub fn from_entry(entry: &TemplateEntry) -> Self {
        Source {
            path: entry.path.clone(),
            kind: entry.kind,
            size: entry.bytes.len() as u64,
            bytes: entry.bytes.clone(),
        }
    }

    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.bytes).into_owned()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IgnoredPaths {
    files: Vec<String>,
    folders: Vec<String>,
}

impl IgnoredPaths {
    pub fn from_git_listing(listing: &str) -> Self {
        let mut ignored = IgnoredPaths::default();
        for line in listing.split('\0') {
            match line.strip_suffix('/') {
                Some(folder) if !folder.is_empty() && folder != "." => {
                    ignored.folders.push(folder.to_string())
                }
                Some(_) => {}
                None if !line.is_empty() => ignored.files.push(line.to_string()),
                None => {}
            }
        }
        ignored
    }

    pub fn len(&self) -> usize {
        self.files.len() + self.folders.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub(crate) fn covers(&self, path: &str) -> bool {
        self.files.iter().any(|file| file == path)
            || self.folders.iter().any(|folder| {
                path.strip_prefix(folder.as_str())
                    .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
            })
    }
}

pub fn read_dir(dir: &Path) -> Result<Vec<TemplateEntry>, ReadError> {
    Ok(read_sources(dir)?
        .into_iter()
        .map(|source| TemplateEntry {
            path: source.path,
            kind: source.kind,
            bytes: source.bytes,
        })
        .collect())
}

pub(crate) fn read_sources(dir: &Path) -> Result<Vec<Source>, ReadError> {
    read_sources_except(dir, &IgnoredPaths::default())
}

pub(crate) fn read_sources_except(
    dir: &Path,
    ignored: &IgnoredPaths,
) -> Result<Vec<Source>, ReadError> {
    if !dir.is_dir() {
        return Err(ReadError::NotADirectory(dir.to_path_buf()));
    }
    let mut walk = Walk {
        ignored,
        sources: Vec::new(),
        loaded_bytes: 0,
    };
    walk.visit(dir, "")?;
    walk.sources.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(walk.sources)
}

struct Walk<'a> {
    ignored: &'a IgnoredPaths,
    sources: Vec<Source>,
    loaded_bytes: u64,
}

impl Walk<'_> {
    fn visit(&mut self, dir: &Path, prefix: &str) -> Result<(), ReadError> {
        let io_error = |path: &Path, source: io::Error| ReadError::Io {
            path: path.to_path_buf(),
            source,
        };
        let listing = fs::read_dir(dir).map_err(|source| io_error(dir, source))?;
        for item in listing {
            let item = item.map_err(|source| io_error(dir, source))?;
            let full = item.path();
            let name = item
                .file_name()
                .into_string()
                .map_err(|_| ReadError::NonUtf8Path(full.clone()))?;
            let path = format!("{prefix}{name}");
            if is_outside_template(&path) || self.ignored.covers(&path) {
                continue;
            }
            let metadata = fs::symlink_metadata(&full).map_err(|source| io_error(&full, source))?;
            let file_type = metadata.file_type();
            if file_type.is_dir() {
                self.visit(&full, &format!("{path}/"))?;
                continue;
            }
            let kind = if file_type.is_symlink() {
                EntryKind::Symlink
            } else if !file_type.is_file() {
                EntryKind::Other
            } else if link_count(&metadata) > 1 {
                EntryKind::HardLink
            } else {
                EntryKind::File
            };
            let size = if kind == EntryKind::Symlink {
                0
            } else {
                metadata.len()
            };
            let bytes = if self.should_load(kind, size) {
                let mut buffer = Vec::with_capacity(size as usize);
                fs::File::open(&full)
                    .and_then(|file| file.take(MAX_FILE_BYTES).read_to_end(&mut buffer))
                    .map_err(|source| io_error(&full, source))?;
                self.loaded_bytes += buffer.len() as u64;
                buffer
            } else {
                Vec::new()
            };
            self.sources.push(Source {
                path,
                kind,
                size,
                bytes,
            });
        }
        Ok(())
    }

    fn should_load(&self, kind: EntryKind, size: u64) -> bool {
        matches!(kind, EntryKind::File | EntryKind::HardLink)
            && size <= MAX_FILE_BYTES
            && self.sources.len() < MAX_FILES
            && self.loaded_bytes + size <= MAX_TOTAL_BYTES
    }
}

pub(crate) fn is_outside_template(path: &str) -> bool {
    path == ".git" || path.starts_with(".git/") || is_github_path(path)
}

pub(crate) fn is_github_path(path: &str) -> bool {
    path == ".github" || path.starts_with(GITHUB_PREFIX)
}

#[cfg(unix)]
fn link_count(metadata: &fs::Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;
    metadata.nlink()
}

#[cfg(not(unix))]
fn link_count(_metadata: &fs::Metadata) -> u64 {
    1
}
