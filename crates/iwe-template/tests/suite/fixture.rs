use std::fs;
use std::path::{Path, PathBuf};

use iwe_template::TemplateEntry;

pub const MANIFEST: &str = r#"format = 1
id = "field-notes"
name = "Field Notes"
description = "A small notebook with a board and one skill."
version = "1.2.0"
authors = ["Ada Example"]
license = "MIT"
exclude = ["README.md", "drafts"]
"#;

pub const CONFIG: &str = r#"version = 4

[markdown]
refs_extension = ".md"

[workspace]
path = "data"

[actions.sort]
type = "sort"
title = "Sort A-Z"

[schemas.note]
match = "data/**"
"#;

pub const INDEX_DOCUMENT: &str = r#"# Field Notes

See [the guide](https://example.com/guide) first.

``` html
<button data-prompt="Summarize this week's notes">This week</button>
<script>
  document.querySelector("button").addEventListener("click", () => {
    iwe.prompt(`Review ${key} and list what is missing`)
  })
</script>
```
"#;

pub const BOARD_PAGE: &str = r#"<!doctype html>
<html>
  <head>
    <link rel="stylesheet" href="assets/style.css">
    <script src="assets/app.js"></script>
  </head>
  <body>
    <button data-prompt='Triage the new notes' data-prompt-title="Triage">Triage</button>
    <script>
      render(document.body)
    </script>
  </body>
</html>
"#;

pub const APP_SCRIPT: &str = "function render(root) {\n  root.dataset.ready = \"yes\"\n}\n";

pub fn valid_template() -> Vec<TemplateEntry> {
    let files: &[(&str, &str)] = &[
        (
            ".claude/skills/plan/SKILL.md",
            "# Plan\n\nWrite a plan before any change.\n",
        ),
        (".claude/skills/plan/reference/steps.md", "# Steps\n"),
        (
            ".claude/skills/review/SKILL.md",
            "# Review\n\nRead the diff.\n",
        ),
        (".github/workflows/validate.yml", "name: validate\n"),
        (".gitignore", ".iwe/app/\n"),
        (".iwe/config.toml", CONFIG),
        (".iwe/schemas/note.yaml", "type: object\n"),
        ("AGENTS.md", "# Agents\n\nKeep notes short.\n"),
        ("CLAUDE.md", "@AGENTS.md\n"),
        ("LICENSE", "MIT License\n"),
        ("README.md", "# Field Notes template\n"),
        ("data/assets/app.js", APP_SCRIPT),
        ("data/assets/style.css", "body { margin: 0 }\n"),
        ("data/board.html", BOARD_PAGE),
        ("data/index.md", INDEX_DOCUMENT),
        ("data/notes/first.md", "# First\n"),
        ("drafts/idea.md", "# Idea\n"),
        ("preview.png", "\u{89}PNG fixture"),
        ("template.toml", MANIFEST),
    ];
    files
        .iter()
        .map(|(path, text)| TemplateEntry::file(*path, text.as_bytes()))
        .collect()
}

pub fn with(
    mut entries: Vec<TemplateEntry>,
    path: &str,
    bytes: impl Into<Vec<u8>>,
) -> Vec<TemplateEntry> {
    entries.retain(|entry| entry.path != path);
    entries.push(TemplateEntry::file(path, bytes));
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    entries
}

pub fn without(mut entries: Vec<TemplateEntry>, path: &str) -> Vec<TemplateEntry> {
    entries.retain(|entry| entry.path != path);
    entries
}

pub fn with_manifest_line(entries: Vec<TemplateEntry>, from: &str, to: &str) -> Vec<TemplateEntry> {
    assert!(MANIFEST.contains(from));
    with(entries, "template.toml", MANIFEST.replace(from, to))
}

pub fn with_config(entries: Vec<TemplateEntry>, config: &str) -> Vec<TemplateEntry> {
    with(entries, ".iwe/config.toml", config)
}

pub fn write_tree(root: &Path, entries: &[TemplateEntry]) {
    for entry in entries {
        let target = root.join(&entry.path);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(target, &entry.bytes).unwrap();
    }
}

pub fn snapshot_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/snapshots")
        .join(name)
}

pub fn assert_snapshot(name: &str, actual: &str) {
    let path = snapshot_path(name);
    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        fs::write(&path, actual).unwrap();
    }
    let expected = fs::read_to_string(&path).unwrap_or_default();
    assert_eq!(
        actual, expected,
        "snapshot {name} differs; rerun with UPDATE_SNAPSHOTS=1 to accept"
    );
}
