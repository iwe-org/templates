use iwe_template::{
    validate_entries, EntryKind, Expected, TemplateEntry, Violation, MAX_FILES, MAX_FILE_BYTES,
    MAX_PREVIEW_BYTES, MAX_TOTAL_BYTES,
};

use super::fixture::{valid_template, with, with_config, with_manifest_line, without, CONFIG};

fn violations_of(entries: &[TemplateEntry]) -> Vec<Violation> {
    validate_entries(entries, None)
}

fn assert_only(entries: &[TemplateEntry], expected: Violation) {
    assert_eq!(violations_of(entries), vec![expected]);
}

fn assert_path_fails(path: &str, expected: fn(String) -> Violation) {
    assert_only(
        &with(valid_template(), path, "x\n"),
        expected(path.to_string()),
    );
}

fn dot_path(path: String) -> Violation {
    Violation::DotPathNotAllowed { path }
}

fn extension(path: String) -> Violation {
    Violation::ExtensionNotAllowed { path }
}

fn agent_instructions(path: String) -> Violation {
    Violation::AgentInstructionsOutsideRoot { path }
}

fn config_with(extra: &str) -> Vec<TemplateEntry> {
    with_config(valid_template(), &format!("{CONFIG}\n{extra}"))
}

#[test]
fn the_fixture_template_passes() {
    assert_eq!(violations_of(&valid_template()), vec![]);
}

#[test]
fn every_allowed_extension_passes() {
    let mut entries = valid_template();
    for extension in iwe_template::ALLOWED_EXTENSIONS {
        entries = with(entries, &format!("data/file.{extension}"), "x\n");
    }
    assert_eq!(violations_of(&entries), vec![]);
}

#[test]
fn a_file_type_outside_the_list_fails() {
    assert_path_fails("data/run.sh", extension);
    assert_path_fails("data/notes.txt", extension);
    assert_path_fails("data/archive.zip", extension);
    assert_path_fails("data/Makefile", extension);
    assert_path_fails("data/PHOTO.PNG", extension);
    assert_path_fails("data/template.toml", extension);
    assert_path_fails("data/LICENSE", extension);
    assert_path_fails("data/schema.yaml", extension);
}

#[test]
fn dot_paths_outside_the_named_ones_fail() {
    assert_path_fails(".claude/settings.json", dot_path);
    assert_path_fails(".claude/settings.local.json", dot_path);
    assert_path_fails(".claude/hooks/start.md", dot_path);
    assert_path_fails(".claude/skills/plan/run.sh", dot_path);
    assert_path_fails(".claude/skills/plan/.hidden/notes.md", dot_path);
    assert_path_fails(".mcp.json", dot_path);
    assert_path_fails(".iwe/app/objects.db", dot_path);
    assert_path_fails(".iwe/schemas/nested/note.yaml", dot_path);
    assert_path_fails(".iwe/schemas/note.yml", dot_path);
    assert_path_fails(".iwe/notes.md", dot_path);
    assert_path_fails(".env", dot_path);
    assert_path_fails("data/.gitignore", dot_path);
    assert_path_fails("data/.hidden.md", dot_path);
    assert_path_fails(".GitHub/workflows/ci.yml", dot_path);
    assert_path_fails(".Claude/skills/other/SKILL.md", dot_path);
}

#[test]
fn the_github_folder_is_ignored() {
    let entries = with(valid_template(), ".github/scripts/build.sh", "exit 1\n");
    assert_eq!(violations_of(&entries), vec![]);
}

#[test]
fn agent_instructions_outside_the_root_fail_in_any_case() {
    assert_path_fails("data/AGENTS.md", agent_instructions);
    assert_path_fails("data/notes/CLAUDE.md", agent_instructions);
    assert_path_fails("data/claude.md", agent_instructions);
    assert_path_fails("data/Agents.MD", agent_instructions);
    assert_path_fails(".claude/skills/plan/CLAUDE.md", agent_instructions);
    assert_only(
        &with(without(valid_template(), "AGENTS.md"), "agents.md", "x\n"),
        agent_instructions("agents.md".to_string()),
    );
}

#[test]
fn symlinks_hard_links_and_special_files_fail() {
    let linked = |kind| {
        let mut entries = valid_template();
        entries.push(TemplateEntry {
            path: "data/link.md".to_string(),
            kind,
            bytes: Vec::new(),
        });
        entries
    };
    let path = || "data/link.md".to_string();
    assert_only(
        &linked(EntryKind::Symlink),
        Violation::Symlink { path: path() },
    );
    assert_only(
        &linked(EntryKind::HardLink),
        Violation::HardLink { path: path() },
    );
    assert_only(
        &linked(EntryKind::Other),
        Violation::UnsupportedFileKind { path: path() },
    );
}

#[test]
fn paths_that_leave_the_template_fail() {
    for path in [
        "../outside.md",
        "data/../../outside.md",
        "/etc/passwd.md",
        "data//x.md",
        "./x.md",
        "data\\x.md",
        "",
    ] {
        assert_only(
            &with(valid_template(), path, "x\n"),
            Violation::EscapingPath {
                path: path.to_string(),
            },
        );
    }
}

#[test]
fn a_path_repeated_in_another_letter_case_fails() {
    let mut entries = valid_template();
    entries.push(TemplateEntry::file("data/Index.md", "# Other\n"));
    assert_only(
        &entries,
        Violation::DuplicatePath {
            path: "data/Index.md".to_string(),
        },
    );
}

#[test]
fn more_than_the_file_limit_fails() {
    let mut entries = valid_template();
    let missing = MAX_FILES - (entries.len() - 1);
    for number in 0..missing {
        entries.push(TemplateEntry::file(format!("data/bulk/{number}.md"), "x\n"));
    }
    assert_eq!(violations_of(&entries), vec![]);
    entries.push(TemplateEntry::file("data/bulk/last.md", "x\n"));
    assert_only(
        &entries,
        Violation::TooManyFiles {
            count: MAX_FILES + 1,
        },
    );
}

#[test]
fn a_file_over_the_limit_fails() {
    let at_limit = vec![b'a'; MAX_FILE_BYTES as usize];
    assert_eq!(
        violations_of(&with(valid_template(), "data/big.md", at_limit)),
        vec![]
    );
    let over = vec![b'a'; MAX_FILE_BYTES as usize + 1];
    assert_only(
        &with(valid_template(), "data/big.md", over),
        Violation::FileTooLarge {
            path: "data/big.md".to_string(),
            size: MAX_FILE_BYTES + 1,
        },
    );
}

#[test]
fn a_total_over_the_limit_fails() {
    let mut entries = valid_template();
    for number in 0..5 {
        entries.push(TemplateEntry::file(
            format!("data/bulk/{number}.md"),
            vec![b'a'; MAX_FILE_BYTES as usize],
        ));
    }
    let total: u64 = entries
        .iter()
        .filter(|entry| !entry.path.starts_with(".github/"))
        .map(|entry| entry.bytes.len() as u64)
        .sum();
    assert!(total > MAX_TOTAL_BYTES);
    assert_only(&entries, Violation::TotalTooLarge { size: total });
}

#[test]
fn a_preview_over_the_limit_fails() {
    let at_limit = vec![0u8; MAX_PREVIEW_BYTES as usize];
    assert_eq!(
        violations_of(&with(valid_template(), "preview.png", at_limit)),
        vec![]
    );
    let over = vec![0u8; MAX_PREVIEW_BYTES as usize + 1];
    assert_only(
        &with(valid_template(), "preview.png", over),
        Violation::PreviewTooLarge {
            size: MAX_PREVIEW_BYTES + 1,
        },
    );
}

#[test]
fn a_long_line_in_a_script_file_fails() {
    let long = format!("const ok = 1\nconst data = \"{}\"\n", "a".repeat(500));
    assert_only(
        &with(valid_template(), "data/assets/app.js", long),
        Violation::ScriptLineTooLong {
            path: "data/assets/app.js".to_string(),
            line: 2,
            length: 515,
        },
    );
    let at_limit = format!("{}\n", "a".repeat(500));
    assert_eq!(
        violations_of(&with(valid_template(), "data/assets/app.js", at_limit)),
        vec![]
    );
}

#[test]
fn a_long_line_in_a_script_element_of_a_page_fails() {
    let page = format!(
        "<html>\n<body>\n<p>{}</p>\n<SCRIPT type=\"module\">\nlet a = 1\n{}\n</SCRIPT>\n</body>\n</html>\n",
        "prose ".repeat(200),
        "x".repeat(501)
    );
    assert_only(
        &with(valid_template(), "data/board.html", page),
        Violation::ScriptLineTooLong {
            path: "data/board.html".to_string(),
            line: 6,
            length: 501,
        },
    );
}

#[test]
fn a_long_line_in_a_script_element_of_an_html_block_fails() {
    let document = format!(
        "# Page\n\n{}\n\n``` html\n<div></div>\n<script>{}</script>\n```\n\n``` js\n{}\n```\n",
        "prose ".repeat(200),
        "y".repeat(600),
        "z".repeat(600)
    );
    assert_only(
        &with(valid_template(), "data/index.md", document),
        Violation::ScriptLineTooLong {
            path: "data/index.md".to_string(),
            line: 7,
            length: 600,
        },
    );
}

#[test]
fn a_missing_config_fails() {
    assert_only(
        &without(valid_template(), ".iwe/config.toml"),
        Violation::ConfigMissing,
    );
}

#[test]
fn a_config_that_does_not_parse_fails() {
    let violations = violations_of(&with_config(valid_template(), "version = = 4\n"));
    assert!(matches!(
        violations.as_slice(),
        [Violation::ConfigInvalid { .. }]
    ));
}

#[test]
fn a_config_key_outside_the_list_fails() {
    assert_only(
        &config_with("[plugins]\nname = \"x\"\n"),
        Violation::ConfigKeyNotAllowed {
            key: "plugins".to_string(),
        },
    );
}

#[test]
fn every_allowed_config_key_passes() {
    let config = "version = 5\nformat = \"markdown\"\n[markdown]\n[djot]\n[workspace]\n[completion]\n[search]\n[templates.default]\nkey_template = \"{{slug}}\"\n[actions]\n[schemas]\n";
    assert_eq!(
        violations_of(&with_config(valid_template(), config)),
        vec![]
    );
}

#[test]
fn a_config_version_below_four_fails() {
    assert_only(
        &with_config(
            valid_template(),
            &CONFIG.replace("version = 4", "version = 3"),
        ),
        Violation::ConfigVersion { found: Some(3) },
    );
    assert_only(
        &with_config(valid_template(), &CONFIG.replace("version = 4\n", "")),
        Violation::ConfigVersion { found: None },
    );
}

#[test]
fn a_library_table_fails() {
    assert_only(
        &config_with("[library]\npath = \"\"\n"),
        Violation::ConfigLibraryTable,
    );
}

#[test]
fn a_commands_table_fails() {
    assert_only(
        &config_with("[commands.claude]\nrun = \"claude -p\"\n"),
        Violation::ConfigCommands,
    );
}

#[test]
fn a_transform_action_fails() {
    assert_only(
        &config_with("[actions.rewrite]\ntype = \"transform\"\ntitle = \"Rewrite\"\ncommand = \"claude\"\ninput_template = \"{{context}}\"\n"),
        Violation::ConfigTransformAction { action: "rewrite".to_string() },
    );
}

#[test]
fn an_action_carrying_a_command_fails() {
    assert_only(
        &config_with("[actions.link]\ntype = \"link\"\ntitle = \"Link\"\ncommand = \"claude\"\n"),
        Violation::ConfigActionCommand {
            action: "link".to_string(),
        },
    );
}

#[test]
fn a_workspace_path_that_is_not_a_contained_folder_fails() {
    for path in [
        "missing",
        "../data",
        "/data",
        "data/../data",
        "data/index.md",
    ] {
        assert_only(
            &with_config(
                valid_template(),
                &CONFIG.replace("path = \"data\"", &format!("path = \"{path}\"")),
            ),
            Violation::ConfigWorkspacePath {
                path: path.to_string(),
            },
        );
    }
    for path in ["", "data/", "data/notes"] {
        let config = CONFIG.replace("path = \"data\"", &format!("path = \"{path}\""));
        assert_eq!(
            violations_of(&with_config(valid_template(), &config)),
            vec![]
        );
    }
}

#[test]
fn a_missing_manifest_fails() {
    assert_only(
        &without(valid_template(), "template.toml"),
        Violation::ManifestMissing,
    );
}

#[test]
fn a_manifest_that_does_not_parse_fails() {
    for broken in [
        "format = ",
        "format = 1\n",
        &format!("{}\nhooks = true\n", super::fixture::MANIFEST),
    ] {
        let violations = violations_of(&with(valid_template(), "template.toml", broken));
        assert!(
            matches!(violations.as_slice(), [Violation::ManifestInvalid { .. }]),
            "{broken}"
        );
    }
}

#[test]
fn a_manifest_format_other_than_one_fails() {
    assert_only(
        &with_manifest_line(valid_template(), "format = 1", "format = 2"),
        Violation::ManifestFormat { found: 2 },
    );
}

#[test]
fn a_manifest_id_that_is_not_short_kebab_case_fails() {
    let too_long = "a".repeat(41);
    for id in [
        "ab",
        "Field-Notes",
        "field_notes",
        "field--notes",
        "-field",
        "field-",
        "9lives",
        too_long.as_str(),
    ] {
        assert_only(
            &with_manifest_line(
                valid_template(),
                "id = \"field-notes\"",
                &format!("id = \"{id}\""),
            ),
            Violation::ManifestId { id: id.to_string() },
        );
    }
}

#[test]
fn a_manifest_name_over_forty_characters_fails() {
    let name = "n".repeat(41);
    assert_only(
        &with_manifest_line(
            valid_template(),
            "name = \"Field Notes\"",
            &format!("name = \"{name}\""),
        ),
        Violation::ManifestName { name },
    );
    assert_only(
        &with_manifest_line(valid_template(), "name = \"Field Notes\"", "name = \"\""),
        Violation::ManifestName {
            name: String::new(),
        },
    );
}

#[test]
fn a_manifest_description_over_the_limit_or_on_two_lines_fails() {
    let long = "d".repeat(141);
    let from = "description = \"A small notebook with a board and one skill.\"";
    assert_only(
        &with_manifest_line(valid_template(), from, &format!("description = \"{long}\"")),
        Violation::ManifestDescription { description: long },
    );
    assert_only(
        &with_manifest_line(valid_template(), from, "description = \"One.\\nTwo.\""),
        Violation::ManifestDescription {
            description: "One.\nTwo.".to_string(),
        },
    );
}

#[test]
fn a_manifest_version_that_is_not_semver_fails() {
    for version in ["1.2", "v1.2.0", "latest", "1.2.0+build"] {
        assert_only(
            &with_manifest_line(
                valid_template(),
                "version = \"1.2.0\"",
                &format!("version = \"{version}\""),
            ),
            Violation::ManifestVersion {
                version: version.to_string(),
            },
        );
    }
}

#[test]
fn a_manifest_without_an_author_fails() {
    for authors in ["[]", "[\"\"]"] {
        assert_only(
            &with_manifest_line(
                valid_template(),
                "authors = [\"Ada Example\"]",
                &format!("authors = {authors}"),
            ),
            Violation::ManifestAuthors,
        );
    }
}

#[test]
fn a_license_outside_the_accepted_list_fails() {
    assert_only(
        &with_manifest_line(
            valid_template(),
            "license = \"MIT\"",
            "license = \"GPL-3.0\"",
        ),
        Violation::ManifestLicense {
            license: "GPL-3.0".to_string(),
        },
    );
    for license in iwe_template::ACCEPTED_LICENSES {
        let entries = with_manifest_line(
            valid_template(),
            "license = \"MIT\"",
            &format!("license = \"{license}\""),
        );
        assert_eq!(violations_of(&entries), vec![]);
    }
}

#[test]
fn an_exclude_pattern_that_is_invalid_or_drops_the_config_fails() {
    for pattern in ["../x", "/abs", "a[", ".iwe", ".iwe/*.toml", "**"] {
        assert_only(
            &with_manifest_line(
                valid_template(),
                "exclude = [\"README.md\", \"drafts\"]",
                &format!("exclude = [\"{pattern}\"]"),
            ),
            Violation::ManifestExclude {
                pattern: pattern.to_string(),
            },
        );
    }
}

#[test]
fn an_id_or_version_other_than_the_registry_entry_fails() {
    let expected = |id: &str, version: &str, previous: Option<&str>| Expected {
        id: id.to_string(),
        version: version.to_string(),
        previous_version: previous.map(str::to_string),
    };
    let entries = valid_template();
    assert_eq!(
        validate_entries(&entries, Some(&expected("field-notes", "1.2.0", None))),
        vec![]
    );
    assert_eq!(
        validate_entries(&entries, Some(&expected("other-notes", "1.2.0", None))),
        vec![Violation::RegistryIdMismatch {
            expected: "other-notes".to_string(),
            found: "field-notes".to_string(),
        }]
    );
    assert_eq!(
        validate_entries(&entries, Some(&expected("field-notes", "1.3.0", None))),
        vec![Violation::RegistryVersionMismatch {
            expected: "1.3.0".to_string(),
            found: "1.2.0".to_string(),
        }]
    );
}

#[test]
fn a_version_that_does_not_increase_fails() {
    let expected = |previous: &str| Expected {
        id: "field-notes".to_string(),
        version: "1.2.0".to_string(),
        previous_version: Some(previous.to_string()),
    };
    let entries = valid_template();
    assert_eq!(validate_entries(&entries, Some(&expected("1.1.9"))), vec![]);
    assert_eq!(
        validate_entries(&entries, Some(&expected("1.2.0-rc.1"))),
        vec![]
    );
    for previous in ["1.2.0", "1.10.0", "2.0.0"] {
        assert_eq!(
            validate_entries(&entries, Some(&expected(previous))),
            vec![Violation::VersionNotIncreased {
                previous: previous.to_string(),
                found: "1.2.0".to_string(),
            }]
        );
    }
}

#[test]
fn every_violation_has_a_message() {
    for entries in [
        with(valid_template(), ".mcp.json", "{}"),
        without(valid_template(), ".iwe/config.toml"),
        without(valid_template(), "template.toml"),
    ] {
        for violation in violations_of(&entries) {
            assert!(!violation.to_string().is_empty());
        }
    }
}
