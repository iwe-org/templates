use std::cell::Cell;
use std::fmt;
use std::io::{self, Read};
use std::rc::Rc;

use flate2::read::GzDecoder;

use crate::entry::TemplateEntry;
use crate::limits::{
    MAX_ARCHIVE_BYTES, MAX_FILES, MAX_FILE_BYTES, MAX_TOTAL_BYTES, MAX_UNPACKED_STREAM_BYTES,
};
use crate::paths::is_safe_relative_path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnpackError {
    ArchiveTooLarge { size: u64 },
    StreamTooLarge,
    Malformed { message: String },
    TooManyFiles,
    EntryTooLarge { path: String, size: u64 },
    TotalTooLarge,
    Symlink { path: String },
    HardLink { path: String },
    UnsupportedEntry { path: String },
    EscapingPath { path: String },
    DuplicatePath { path: String },
}

impl fmt::Display for UnpackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnpackError::ArchiveTooLarge { size } => {
                write!(
                    f,
                    "the archive is {size} bytes, more than {MAX_ARCHIVE_BYTES}"
                )
            }
            UnpackError::StreamTooLarge => write!(
                f,
                "the archive decompresses to more than {MAX_UNPACKED_STREAM_BYTES} bytes"
            ),
            UnpackError::Malformed { message } => write!(f, "the archive is malformed: {message}"),
            UnpackError::TooManyFiles => write!(f, "the archive has more than {MAX_FILES} files"),
            UnpackError::EntryTooLarge { path, size } => {
                write!(f, "{path}: {size} bytes, more than {MAX_FILE_BYTES}")
            }
            UnpackError::TotalTooLarge => {
                write!(
                    f,
                    "the archive holds more than {MAX_TOTAL_BYTES} bytes in total"
                )
            }
            UnpackError::Symlink { path } => write!(f, "{path}: symlinks are not allowed"),
            UnpackError::HardLink { path } => write!(f, "{path}: hard links are not allowed"),
            UnpackError::UnsupportedEntry { path } => {
                write!(f, "{path}: only regular files are allowed")
            }
            UnpackError::EscapingPath { path } => {
                write!(f, "{path:?}: the path leaves the template")
            }
            UnpackError::DuplicatePath { path } => {
                write!(f, "{path}: the path appears more than once")
            }
        }
    }
}

impl std::error::Error for UnpackError {}

struct BoundedReader<R> {
    inner: R,
    remaining: u64,
    exceeded: Rc<Cell<bool>>,
}

impl<R: Read> Read for BoundedReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        if self.remaining == 0 {
            let mut probe = [0u8; 1];
            if self.inner.read(&mut probe)? == 0 {
                return Ok(0);
            }
            self.exceeded.set(true);
            return Err(io::Error::other("decompressed stream limit exceeded"));
        }
        let allowed = buffer.len().min(self.remaining as usize);
        let read = self.inner.read(&mut buffer[..allowed])?;
        self.remaining -= read as u64;
        Ok(read)
    }
}

pub fn unpack(archive: &[u8]) -> Result<Vec<TemplateEntry>, UnpackError> {
    if archive.len() as u64 > MAX_ARCHIVE_BYTES {
        return Err(UnpackError::ArchiveTooLarge {
            size: archive.len() as u64,
        });
    }
    let exceeded = Rc::new(Cell::new(false));
    let reader = BoundedReader {
        inner: GzDecoder::new(archive),
        remaining: MAX_UNPACKED_STREAM_BYTES,
        exceeded: Rc::clone(&exceeded),
    };
    let failure = |error: io::Error| {
        if exceeded.get() {
            UnpackError::StreamTooLarge
        } else {
            UnpackError::Malformed {
                message: error.to_string(),
            }
        }
    };
    let mut tar = tar::Archive::new(reader);
    let mut entries: Vec<TemplateEntry> = Vec::new();
    let mut total: u64 = 0;
    for item in tar.entries().map_err(&failure)? {
        let mut item = item.map_err(&failure)?;
        let path = String::from_utf8(item.path_bytes().into_owned()).map_err(|error| {
            UnpackError::EscapingPath {
                path: String::from_utf8_lossy(error.as_bytes()).into_owned(),
            }
        })?;
        let entry_type = item.header().entry_type();
        if entry_type.is_dir() {
            let folder = path.trim_end_matches('/');
            if !is_safe_relative_path(folder) {
                return Err(UnpackError::EscapingPath { path });
            }
            continue;
        }
        if !is_safe_relative_path(&path) {
            return Err(UnpackError::EscapingPath { path });
        }
        if entry_type.is_symlink() {
            return Err(UnpackError::Symlink { path });
        }
        if entry_type.is_hard_link() {
            return Err(UnpackError::HardLink { path });
        }
        if !entry_type.is_file() {
            return Err(UnpackError::UnsupportedEntry { path });
        }
        if entries.len() >= MAX_FILES {
            return Err(UnpackError::TooManyFiles);
        }
        let size = item.size();
        if size > MAX_FILE_BYTES {
            return Err(UnpackError::EntryTooLarge { path, size });
        }
        total += size;
        if total > MAX_TOTAL_BYTES {
            return Err(UnpackError::TotalTooLarge);
        }
        if entries.iter().any(|entry| entry.path == path) {
            return Err(UnpackError::DuplicatePath { path });
        }
        let mut bytes = Vec::with_capacity(size as usize);
        (&mut item)
            .take(MAX_FILE_BYTES)
            .read_to_end(&mut bytes)
            .map_err(&failure)?;
        entries.push(TemplateEntry::file(path, bytes));
    }
    Ok(entries)
}
