use std::collections::BTreeMap;

use iwe_template::{
    assemble_index, pack_entries, AgentInfo, EntryRecord, Index, IndexError, Origin, Registry,
    RegistryProblem, DEFAULT_RELEASE_BASE_URL, FORMAT,
};

use super::fixture::{assert_snapshot, valid_template, without};

const REGISTRY: &str = r#"
[field-notes]
repository = "https://github.com/example/field-notes"
rev = "0123456789abcdef0123456789abcdef01234567"
version = "1.2.0"
path = ""
featured = true

[plain-notes]
repository = "https://github.com/example/mono"
rev = "89abcdef0123456789abcdef0123456789abcdef"
version = "0.3.1"
path = "templates/plain"
"#;

fn records() -> Vec<EntryRecord> {
    let field_notes = pack_entries(
        &valid_template(),
        &Origin {
            repository: "https://github.com/example/field-notes".to_string(),
            rev: "0123456789abcdef0123456789abcdef01234567".to_string(),
            published: "2026-11-11".to_string(),
        },
    )
    .unwrap()
    .record;
    let plain_notes = EntryRecord {
        id: "plain-notes".to_string(),
        format: FORMAT,
        name: "Plain Notes".to_string(),
        description: "Notes and nothing else.".to_string(),
        version: "0.3.1".to_string(),
        authors: vec!["Bo Example".to_string(), "Cy Example".to_string()],
        license: "CC0-1.0".to_string(),
        repository: "https://github.com/example/mono".to_string(),
        rev: "89abcdef0123456789abcdef0123456789abcdef".to_string(),
        sha256: "f".repeat(64),
        preview: false,
        files: 4,
        agent: AgentInfo::default(),
        published: "2026-11-09".to_string(),
    };
    let superseded = EntryRecord {
        version: "0.3.0".to_string(),
        ..plain_notes.clone()
    };
    vec![superseded, field_notes, plain_notes]
}

fn stars() -> BTreeMap<String, u64> {
    BTreeMap::from([("field-notes".to_string(), 11)])
}

#[test]
fn the_index_for_a_registry() {
    let registry = Registry::parse(REGISTRY).unwrap();
    assert_eq!(registry.problems(), vec![]);
    let mut index = assemble_index(
        &registry,
        &records(),
        &stars(),
        DEFAULT_RELEASE_BASE_URL,
        "2026-11-10T09:00:00Z",
    )
    .unwrap();
    assert_eq!(index.templates[0].sha256.len(), 64);
    index.templates[0].sha256 = "0".repeat(64);
    let json = serde_json::to_string_pretty(&index).unwrap() + "\n";
    assert_snapshot("index.json", &json);
    assert_eq!(serde_json::from_str::<Index>(&json).unwrap(), index);
}

#[test]
fn an_empty_registry_gives_an_empty_index() {
    let registry = Registry::parse("# no entries yet\n").unwrap();
    let index = assemble_index(
        &registry,
        &[],
        &BTreeMap::new(),
        DEFAULT_RELEASE_BASE_URL,
        "now",
    )
    .unwrap();
    assert_eq!(
        serde_json::to_string(&index).unwrap(),
        r#"{"format":1,"generated":"now","templates":[]}"#
    );
}

#[test]
fn the_spec_example_parses_and_unknown_fields_are_ignored() {
    let json = r#"{
      "format": 1,
      "generated": "2026-11-10T09:00:00Z",
      "future": {"anything": true},
      "templates": [
        {
          "id": "dev-workspace",
          "format": 1,
          "name": "Dev Workspace",
          "description": "Memory for your coding agent: specs, plans, bugs and releases as a graph.",
          "version": "1.0.0",
          "authors": ["iWe"],
          "license": "MIT",
          "repository": "https://github.com/iwe-org/dev-workspace",
          "rev": "4f1c9a0e6b7d2c3a8e5f0b1d9c7a6e4f3b2d1c0a",
          "archive": "https://github.com/iwe-org/templates/releases/download/dev-workspace-1.0.0/dev-workspace-1.0.0.tar.gz",
          "sha256": "abc",
          "files": 62,
          "agent": { "instructions": true, "skills": ["explore", "implement"], "hooks": 0 },
          "stars": 11,
          "featured": false,
          "published": "2026-11-11",
          "downloads": 40
        }
      ]
    }"#;
    let index: Index = serde_json::from_str(json).unwrap();
    let entry = &index.templates[0];
    assert_eq!(entry.id, "dev-workspace");
    assert_eq!(entry.preview, None);
    assert_eq!(entry.files, 62);
    assert_eq!(entry.stars, 11);
    assert_eq!(entry.agent.skills, vec!["explore", "implement"]);
    assert!(entry.agent.instructions);
    assert!(!serde_json::to_string(&index).unwrap().contains("preview"));
}

#[test]
fn a_version_without_a_published_record_fails() {
    let registry = Registry::parse(REGISTRY).unwrap();
    let only_field_notes: Vec<EntryRecord> = records()
        .into_iter()
        .filter(|record| record.id == "field-notes")
        .collect();
    assert_eq!(
        assemble_index(
            &registry,
            &only_field_notes,
            &stars(),
            DEFAULT_RELEASE_BASE_URL,
            "now"
        ),
        Err(IndexError::MissingRecord {
            id: "plain-notes".to_string(),
            version: "0.3.1".to_string(),
        })
    );
}

#[test]
fn a_revision_moved_without_a_new_version_fails() {
    let registry = Registry::parse(
        &REGISTRY.replace("0123456789abcdef0123456789abcdef01234567", &"a".repeat(40)),
    )
    .unwrap();
    assert!(matches!(
        assemble_index(&registry, &records(), &stars(), DEFAULT_RELEASE_BASE_URL, "now"),
        Err(IndexError::RevMismatch { id, .. }) if id == "field-notes"
    ));
}

#[test]
fn a_template_without_a_preview_has_no_preview_url() {
    let record = pack_entries(
        &without(valid_template(), "preview.png"),
        &Origin::default(),
    )
    .unwrap()
    .record;
    assert!(!record.preview);
}

#[test]
fn registry_problems_are_reported_per_entry() {
    let registry = Registry::parse(
        r#"
[Bad_Id]
repository = "https://gitlab.com/example/notes"
rev = "main"
version = "one"
path = "../up"

[short-rev]
repository = "https://github.com/example/notes.git"
rev = "0123456"
version = "1.0.0"

[upper-rev]
repository = "https://github.com/example/notes/tree/main"
rev = "0123456789ABCDEF0123456789ABCDEF01234567"
version = "1.0.0"
"#,
    )
    .unwrap();
    let field = |value: &str| value.to_string();
    assert_eq!(
        registry.problems(),
        vec![
            RegistryProblem::Id {
                id: field("Bad_Id")
            },
            RegistryProblem::Repository {
                id: field("Bad_Id"),
                repository: field("https://gitlab.com/example/notes")
            },
            RegistryProblem::Rev {
                id: field("Bad_Id"),
                rev: field("main")
            },
            RegistryProblem::Version {
                id: field("Bad_Id"),
                version: field("one")
            },
            RegistryProblem::Path {
                id: field("Bad_Id"),
                path: field("../up")
            },
            RegistryProblem::Repository {
                id: field("short-rev"),
                repository: field("https://github.com/example/notes.git")
            },
            RegistryProblem::Rev {
                id: field("short-rev"),
                rev: field("0123456")
            },
            RegistryProblem::Repository {
                id: field("upper-rev"),
                repository: field("https://github.com/example/notes/tree/main")
            },
            RegistryProblem::Rev {
                id: field("upper-rev"),
                rev: field("0123456789ABCDEF0123456789ABCDEF01234567")
            },
        ]
    );
    assert!(Registry::parse("[a-b-c]\nrepository = 1\n").is_err());
    assert!(Registry::parse(
        "[a-b-c]\nrepository = \"x\"\nrev = \"y\"\nversion = \"1.0.0\"\nhooks = true\n"
    )
    .is_err());
}

#[test]
fn changes_since_a_base_name_added_and_moved_entries() {
    let base = Registry::parse(REGISTRY).unwrap();
    let moved = REGISTRY
        .replace("89abcdef0123456789abcdef0123456789abcdef", &"b".repeat(40))
        .replace("0.3.1", "0.4.0");
    let current = Registry::parse(&format!(
        "{moved}\n[new-notes]\nrepository = \"https://github.com/example/new\"\nrev = \"{}\"\nversion = \"1.0.0\"\n",
        "c".repeat(40)
    ))
    .unwrap();
    let changes = current.changes_since(&base);
    let summary: Vec<(&str, Option<&str>)> = changes
        .iter()
        .map(|change| {
            (
                change.id.as_str(),
                change
                    .previous
                    .as_ref()
                    .map(|previous| previous.version.as_str()),
            )
        })
        .collect();
    assert_eq!(
        summary,
        vec![("new-notes", None), ("plain-notes", Some("0.3.1"))]
    );
    assert_eq!(current.all_entries().len(), 3);
    assert_eq!(base.changes_since(&base), vec![]);
}
