use std::fmt;
use std::io::{self, Write};
use std::path::Path;

use flate2::{Compression, GzBuilder};
use sha2::{Digest, Sha256};

use crate::entry::{read_sources, EntryKind, ReadError, TemplateEntry};
use crate::index::EntryRecord;
use crate::install::{agent_info, installable_files};
use crate::limits::{MANIFEST_PATH, PREVIEW_PATH};
use crate::manifest::{Manifest, ManifestError};
use crate::unpack::{unpack, UnpackError};
use crate::validate::{validate_entries, validate_sources};
use crate::violation::Violation;

const ARCHIVED_FILE_MODE: u32 = 0o644;
const GZIP_UNKNOWN_OPERATING_SYSTEM: u8 = 255;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Origin {
    pub repository: String,
    pub rev: String,
    pub published: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Packed {
    pub archive: Vec<u8>,
    pub preview: Option<Vec<u8>>,
    pub record: EntryRecord,
}

#[derive(Debug)]
pub enum PackError {
    Read(ReadError),
    Invalid(Vec<Violation>),
    Manifest(ManifestError),
    Io(io::Error),
    Unpack(UnpackError),
}

impl fmt::Display for PackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PackError::Read(error) => write!(f, "{error}"),
            PackError::Invalid(violations) => {
                write!(f, "the template breaks {} rule(s)", violations.len())?;
                for violation in violations {
                    write!(f, "\n{violation}")?;
                }
                Ok(())
            }
            PackError::Manifest(error) => write!(f, "{error}"),
            PackError::Io(error) => write!(f, "{error}"),
            PackError::Unpack(error) => write!(f, "the packed archive does not unpack: {error}"),
        }
    }
}

impl std::error::Error for PackError {}

pub fn pack(dir: &Path, origin: &Origin) -> Result<Packed, PackError> {
    let sources = read_sources(dir).map_err(PackError::Read)?;
    let violations = validate_sources(&sources, None);
    if !violations.is_empty() {
        return Err(PackError::Invalid(violations));
    }
    let entries: Vec<TemplateEntry> = sources
        .into_iter()
        .map(|source| TemplateEntry {
            path: source.path,
            kind: source.kind,
            bytes: source.bytes,
        })
        .collect();
    pack_entries(&entries, origin)
}

pub fn pack_entries(entries: &[TemplateEntry], origin: &Origin) -> Result<Packed, PackError> {
    let violations = validate_entries(entries, None);
    if !violations.is_empty() {
        return Err(PackError::Invalid(violations));
    }
    let manifest = Manifest::from_entries(entries).map_err(PackError::Manifest)?;
    let installed = installable_files(entries, &manifest);
    let manifest_entry = entries
        .iter()
        .find(|entry| entry.path == MANIFEST_PATH && entry.kind == EntryKind::File)
        .ok_or(PackError::Manifest(ManifestError::Missing))?;
    let mut archived: Vec<&TemplateEntry> = installed.clone();
    archived.push(manifest_entry);
    archived.sort_by(|a, b| a.path.cmp(&b.path));
    let archive = write_archive(&archived).map_err(PackError::Io)?;
    let unpacked = unpack(&archive).map_err(PackError::Unpack)?;
    let packed_violations = validate_entries(&unpacked, None);
    if !packed_violations.is_empty() {
        return Err(PackError::Invalid(packed_violations));
    }
    let preview = entries
        .iter()
        .find(|entry| entry.path == PREVIEW_PATH && entry.kind == EntryKind::File)
        .map(|entry| entry.bytes.clone());
    let record = EntryRecord {
        id: manifest.id,
        format: manifest.format,
        name: manifest.name,
        description: manifest.description,
        version: manifest.version,
        authors: manifest.authors,
        license: manifest.license,
        repository: origin.repository.clone(),
        rev: origin.rev.clone(),
        sha256: sha256_hex(&archive),
        preview: preview.is_some(),
        files: installed.len() as u32,
        agent: agent_info(&installed),
        published: origin.published.clone(),
    };
    Ok(Packed {
        archive,
        preview,
        record,
    })
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn write_archive(files: &[&TemplateEntry]) -> io::Result<Vec<u8>> {
    let encoder = GzBuilder::new()
        .mtime(0)
        .operating_system(GZIP_UNKNOWN_OPERATING_SYSTEM)
        .write(Vec::new(), Compression::new(9));
    let mut builder = tar::Builder::new(encoder);
    builder.mode(tar::HeaderMode::Deterministic);
    for file in files {
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::Regular);
        header.set_size(file.bytes.len() as u64);
        header.set_mode(ARCHIVED_FILE_MODE);
        header.set_uid(0);
        header.set_gid(0);
        header.set_mtime(0);
        builder.append_data(&mut header, &file.path, file.bytes.as_slice())?;
    }
    let mut encoder = builder.into_inner()?;
    encoder.flush()?;
    encoder.finish()
}
