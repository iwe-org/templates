use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use iwe_template::{sha256_hex, EntryRecord, Index};

use super::fixture::{valid_template, with, write_tree};

fn run(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_iwe-template"))
        .args(arguments)
        .output()
        .unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn path_text(path: &Path) -> &str {
    path.to_str().unwrap()
}

#[test]
fn check_passes_a_valid_template_and_fails_an_invalid_one() {
    let root = tempfile::tempdir().unwrap();
    write_tree(root.path(), &valid_template());
    let passing = run(&[
        "check",
        path_text(root.path()),
        "--id",
        "field-notes",
        "--version",
        "1.2.0",
    ]);
    assert_eq!(passing.status.code(), Some(0), "{}", text(&passing.stdout));

    let mismatched = run(&[
        "check",
        path_text(root.path()),
        "--id",
        "field-notes",
        "--version",
        "2.0.0",
    ]);
    assert_eq!(mismatched.status.code(), Some(1));
    assert!(text(&mismatched.stdout).contains("the registry entry says `2.0.0`"));

    let stale = run(&[
        "check",
        path_text(root.path()),
        "--id",
        "field-notes",
        "--version",
        "1.2.0",
        "--previous-version",
        "1.2.0",
    ]);
    assert_eq!(stale.status.code(), Some(1));

    fs::write(root.path().join(".mcp.json"), "{}").unwrap();
    let failing = run(&["check", path_text(root.path())]);
    assert_eq!(failing.status.code(), Some(1));
    assert!(text(&failing.stdout).starts_with(".mcp.json: "));
}

#[test]
fn check_on_a_missing_folder_exits_with_two() {
    assert_eq!(
        run(&["check", "/nonexistent/template"]).status.code(),
        Some(2)
    );
}

#[test]
fn report_prints_markdown_with_a_compare_link() {
    let old = tempfile::tempdir().unwrap();
    let new = tempfile::tempdir().unwrap();
    write_tree(old.path(), &valid_template());
    write_tree(
        new.path(),
        &with(valid_template(), "data/notes/second.md", "# Second\n"),
    );
    let output = run(&[
        "report",
        path_text(new.path()),
        "--since",
        path_text(old.path()),
        "--repository",
        "https://github.com/example/field-notes",
        "--since-rev",
        "0123456789abcdef0123456789abcdef01234567",
        "--rev",
        "89abcdef0123456789abcdef0123456789abcdef",
    ]);
    assert_eq!(output.status.code(), Some(0));
    let markdown = text(&output.stdout);
    assert!(markdown.contains("1 added, 0 changed, 0 removed"));
    assert!(markdown.contains(
        "<https://github.com/example/field-notes/compare/0123456789abcdef0123456789abcdef01234567...89abcdef0123456789abcdef0123456789abcdef>"
    ));
}

#[test]
fn pack_then_index_publishes_a_template() {
    let root = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    write_tree(root.path(), &valid_template());
    let rev = "0123456789abcdef0123456789abcdef01234567";
    let packed = run(&[
        "pack",
        path_text(root.path()),
        "--out",
        path_text(out.path()),
        "--repository",
        "https://github.com/example/field-notes",
        "--rev",
        rev,
        "--published",
        "2026-11-11",
    ]);
    assert_eq!(packed.status.code(), Some(0), "{}", text(&packed.stderr));
    let archive = fs::read(out.path().join("field-notes-1.2.0.tar.gz")).unwrap();
    let record: EntryRecord =
        serde_json::from_str(&fs::read_to_string(out.path().join("entry.json")).unwrap()).unwrap();
    assert_eq!(record.sha256, sha256_hex(&archive));
    assert!(out.path().join("preview.png").exists());

    let registry = out.path().join("templates.toml");
    fs::write(
        &registry,
        format!("[field-notes]\nrepository = \"https://github.com/example/field-notes\"\nrev = \"{rev}\"\nversion = \"1.2.0\"\n"),
    )
    .unwrap();
    let stars = out.path().join("stars.json");
    fs::write(&stars, r#"{"field-notes": 7}"#).unwrap();
    let index_path = out.path().join("index.json");
    let indexed = run(&[
        "index",
        "--registry",
        path_text(&registry),
        "--entries",
        path_text(out.path()),
        "--stars",
        path_text(&stars),
        "--generated",
        "2026-11-12T00:00:00Z",
        "--out",
        path_text(&index_path),
    ]);
    assert_eq!(indexed.status.code(), Some(0), "{}", text(&indexed.stderr));
    let index: Index = serde_json::from_str(&fs::read_to_string(&index_path).unwrap()).unwrap();
    assert_eq!(index.generated, "2026-11-12T00:00:00Z");
    let entry = &index.templates[0];
    assert_eq!(entry.stars, 7);
    assert_eq!(
        entry.archive,
        "https://github.com/iwe-org/templates/releases/download/field-notes-1.2.0/field-notes-1.2.0.tar.gz"
    );
    assert_eq!(
        entry.preview.as_deref(),
        Some(
            "https://github.com/iwe-org/templates/releases/download/field-notes-1.2.0/preview.png"
        )
    );
    assert_eq!(entry.sha256, record.sha256);

    let entries = run(&["entries", path_text(&registry)]);
    assert_eq!(entries.status.code(), Some(0));
    let listed: serde_json::Value = serde_json::from_slice(&entries.stdout).unwrap();
    assert_eq!(listed[0]["id"], "field-notes");
    assert_eq!(listed[0]["rev"], rev);
    assert_eq!(listed[0]["previous"], serde_json::Value::Null);
}

#[test]
fn pack_refuses_an_invalid_template() {
    let root = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    write_tree(
        root.path(),
        &with(valid_template(), "data/run.sh", "exit 1\n"),
    );
    let packed = run(&[
        "pack",
        path_text(root.path()),
        "--out",
        path_text(out.path()),
    ]);
    assert_eq!(packed.status.code(), Some(1));
    assert!(fs::read_dir(out.path()).unwrap().next().is_none());
}

#[test]
fn entries_rejects_a_branch_name_as_rev() {
    let folder = tempfile::tempdir().unwrap();
    let registry = folder.path().join("templates.toml");
    fs::write(
        &registry,
        "[field-notes]\nrepository = \"https://github.com/example/field-notes\"\nrev = \"main\"\nversion = \"1.2.0\"\n",
    )
    .unwrap();
    let output = run(&["entries", path_text(&registry)]);
    assert_eq!(output.status.code(), Some(2));
    assert!(text(&output.stderr).contains("must be a full 40-character lowercase commit SHA"));
}
