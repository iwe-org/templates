use std::io::Write;

use flate2::write::GzEncoder;
use flate2::Compression;
use iwe_template::{
    pack_entries, sha256_hex, unpack, validate_entries, AgentInfo, Manifest, Origin, PackError,
    TemplateEntry, UnpackError, Violation, MAX_ARCHIVE_BYTES, MAX_FILES, MAX_FILE_BYTES,
};

use super::fixture::{valid_template, with};

const BLOCK: usize = 512;

fn origin() -> Origin {
    Origin {
        repository: "https://github.com/example/field-notes".to_string(),
        rev: "0123456789abcdef0123456789abcdef01234567".to_string(),
        published: "2026-11-11".to_string(),
    }
}

fn raw_header(name: &str, size: u64, entry_type: u8, link: &str) -> [u8; BLOCK] {
    let mut block = [0u8; BLOCK];
    block[..name.len()].copy_from_slice(name.as_bytes());
    block[100..108].copy_from_slice(b"0000644\0");
    block[108..116].copy_from_slice(b"0000000\0");
    block[116..124].copy_from_slice(b"0000000\0");
    block[124..136].copy_from_slice(format!("{size:011o}\0").as_bytes());
    block[136..148].copy_from_slice(b"00000000000\0");
    block[156] = entry_type;
    block[157..157 + link.len()].copy_from_slice(link.as_bytes());
    block[257..263].copy_from_slice(b"ustar\0");
    block[263..265].copy_from_slice(b"00");
    block[148..156].copy_from_slice(b"        ");
    let checksum: u32 = block.iter().map(|byte| u32::from(*byte)).sum();
    block[148..156].copy_from_slice(format!("{checksum:06o}\0 ").as_bytes());
    block
}

fn raw_entry(name: &str, entry_type: u8, link: &str, data: &[u8]) -> Vec<u8> {
    let mut bytes = raw_header(name, data.len() as u64, entry_type, link).to_vec();
    bytes.extend_from_slice(data);
    bytes.resize(bytes.len().div_ceil(BLOCK) * BLOCK, 0);
    bytes
}

fn gzip(tar: &[u8]) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(tar).unwrap();
    encoder.write_all(&[0u8; 2 * BLOCK]).unwrap();
    encoder.finish().unwrap()
}

#[test]
fn packing_is_deterministic() {
    let first = pack_entries(&valid_template(), &origin()).unwrap();
    let mut shuffled = valid_template();
    shuffled.reverse();
    let second = pack_entries(&shuffled, &origin()).unwrap();
    assert_eq!(first.archive, second.archive);
    assert_eq!(first.record.sha256, sha256_hex(&second.archive));
    assert_eq!(first.record.sha256.len(), 64);
}

#[test]
fn a_packed_template_unpacks_and_validates() {
    let entries = valid_template();
    let packed = pack_entries(&entries, &origin()).unwrap();
    let unpacked = unpack(&packed.archive).unwrap();
    assert_eq!(validate_entries(&unpacked, None), vec![]);
    let paths: Vec<&str> = unpacked.iter().map(|entry| entry.path.as_str()).collect();
    assert_eq!(
        paths,
        vec![
            ".claude/skills/plan/SKILL.md",
            ".claude/skills/plan/reference/steps.md",
            ".claude/skills/review/SKILL.md",
            ".gitignore",
            ".iwe/config.toml",
            ".iwe/schemas/note.yaml",
            "AGENTS.md",
            "CLAUDE.md",
            "data/assets/app.js",
            "data/assets/style.css",
            "data/board.html",
            "data/index.md",
            "data/notes/first.md",
            "template.toml",
        ]
    );
    for entry in &unpacked {
        let original = entries
            .iter()
            .find(|other| other.path == entry.path)
            .unwrap();
        assert_eq!(entry.bytes, original.bytes);
    }
    let manifest = Manifest::from_entries(&unpacked).unwrap();
    assert_eq!(manifest.id, "field-notes");
}

#[test]
fn the_record_describes_the_packed_template() {
    let packed = pack_entries(&valid_template(), &origin()).unwrap();
    let record = &packed.record;
    assert_eq!(record.id, "field-notes");
    assert_eq!(record.format, 1);
    assert_eq!(record.version, "1.2.0");
    assert_eq!(record.files, 13);
    assert!(record.preview);
    assert_eq!(
        packed.preview.as_deref(),
        Some("\u{89}PNG fixture".as_bytes())
    );
    assert_eq!(record.repository, origin().repository);
    assert_eq!(record.rev, origin().rev);
    assert_eq!(record.published, "2026-11-11");
    assert_eq!(
        record.agent,
        AgentInfo {
            instructions: true,
            skills: vec!["plan".to_string(), "review".to_string()],
        }
    );
}

#[test]
fn a_long_path_survives_the_round_trip() {
    let long = format!("data/{}/note.md", "deep/".repeat(40).trim_end_matches('/'));
    let packed = pack_entries(&with(valid_template(), &long, "# Deep\n"), &origin()).unwrap();
    let unpacked = unpack(&packed.archive).unwrap();
    assert!(unpacked.iter().any(|entry| entry.path == long));
}

#[test]
fn an_invalid_template_is_not_packed() {
    let result = pack_entries(&with(valid_template(), ".mcp.json", "{}"), &origin());
    assert!(matches!(
        result,
        Err(PackError::Invalid(violations))
            if violations == vec![Violation::DotPathNotAllowed { path: ".mcp.json".to_string() }]
    ));
}

#[test]
fn an_escaping_path_is_refused() {
    for name in ["../evil.md", "data/../../evil.md", "/etc/evil.md"] {
        let archive = gzip(&raw_entry(name, b'0', "", b"x"));
        assert_eq!(
            unpack(&archive),
            Err(UnpackError::EscapingPath {
                path: name.to_string()
            })
        );
    }
}

#[test]
fn a_symlink_is_refused() {
    let archive = gzip(&raw_entry("data/link.md", b'2', "/etc/passwd", b""));
    assert_eq!(
        unpack(&archive),
        Err(UnpackError::Symlink {
            path: "data/link.md".to_string()
        })
    );
}

#[test]
fn a_hard_link_is_refused() {
    let archive = gzip(&raw_entry("data/link.md", b'1', "data/index.md", b""));
    assert_eq!(
        unpack(&archive),
        Err(UnpackError::HardLink {
            path: "data/link.md".to_string()
        })
    );
}

#[test]
fn a_device_or_fifo_is_refused() {
    for entry_type in *b"346" {
        let archive = gzip(&raw_entry("data/special.md", entry_type, "", b""));
        assert_eq!(
            unpack(&archive),
            Err(UnpackError::UnsupportedEntry {
                path: "data/special.md".to_string()
            })
        );
    }
}

#[test]
fn an_oversized_entry_is_refused_before_it_is_read() {
    let mut tar = raw_header("data/big.md", 4 * 1024 * 1024 * 1024, b'0', "").to_vec();
    tar.extend_from_slice(&[b'a'; BLOCK]);
    assert_eq!(
        unpack(&gzip(&tar)),
        Err(UnpackError::EntryTooLarge {
            path: "data/big.md".to_string(),
            size: 4 * 1024 * 1024 * 1024,
        })
    );
    let over = vec![b'a'; MAX_FILE_BYTES as usize + 1];
    assert_eq!(
        unpack(&gzip(&raw_entry("data/big.md", b'0', "", &over))),
        Err(UnpackError::EntryTooLarge {
            path: "data/big.md".to_string(),
            size: MAX_FILE_BYTES + 1,
        })
    );
}

#[test]
fn too_many_entries_are_refused() {
    let mut tar = Vec::new();
    for number in 0..=MAX_FILES {
        tar.extend(raw_entry(&format!("data/{number}.md"), b'0', "", b"x"));
    }
    assert_eq!(unpack(&gzip(&tar)), Err(UnpackError::TooManyFiles));
}

#[test]
fn a_total_over_the_limit_is_refused() {
    let megabyte = vec![b'a'; MAX_FILE_BYTES as usize];
    let mut tar = Vec::new();
    for number in 0..6 {
        tar.extend(raw_entry(&format!("data/{number}.md"), b'0', "", &megabyte));
    }
    assert_eq!(unpack(&gzip(&tar)), Err(UnpackError::TotalTooLarge));
}

#[test]
fn a_repeated_path_is_refused() {
    let mut tar = raw_entry("data/a.md", b'0', "", b"one");
    tar.extend(raw_entry("data/a.md", b'0', "", b"two"));
    assert_eq!(
        unpack(&gzip(&tar)),
        Err(UnpackError::DuplicatePath {
            path: "data/a.md".to_string()
        })
    );
}

#[test]
fn a_decompression_bomb_is_refused() {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
    encoder
        .write_all(&raw_header("././@LongLink", 64 * 1024 * 1024, b'L', ""))
        .unwrap();
    let chunk = vec![b'a'; 1024 * 1024];
    for _ in 0..64 {
        encoder.write_all(&chunk).unwrap();
    }
    let bomb = encoder.finish().unwrap();
    assert!((bomb.len() as u64) < MAX_ARCHIVE_BYTES);
    assert_eq!(unpack(&bomb), Err(UnpackError::StreamTooLarge));

    let mut padded = GzEncoder::new(Vec::new(), Compression::best());
    for number in 0..40_000 {
        padded
            .write_all(&raw_entry(&format!("folder-{number}/"), b'5', "", b""))
            .unwrap();
    }
    assert_eq!(
        unpack(&padded.finish().unwrap()),
        Err(UnpackError::StreamTooLarge)
    );
}

#[test]
fn an_archive_over_the_download_limit_is_refused() {
    let archive = vec![0u8; MAX_ARCHIVE_BYTES as usize + 1];
    assert_eq!(
        unpack(&archive),
        Err(UnpackError::ArchiveTooLarge {
            size: MAX_ARCHIVE_BYTES + 1
        })
    );
}

#[test]
fn bytes_that_are_not_an_archive_are_refused() {
    assert!(matches!(
        unpack(b"not a gzip stream"),
        Err(UnpackError::Malformed { .. })
    ));
    let truncated = pack_entries(&valid_template(), &origin()).unwrap().archive;
    assert!(matches!(
        unpack(&truncated[..truncated.len() / 2]),
        Err(UnpackError::Malformed { .. })
    ));
}

#[test]
fn unpacked_entries_are_plain_files() {
    let archive = gzip(&raw_entry("data/a.md", b'0', "", b"one"));
    assert_eq!(
        unpack(&archive),
        Ok(vec![TemplateEntry::file("data/a.md", "one")])
    );
}
