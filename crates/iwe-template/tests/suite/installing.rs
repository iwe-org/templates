use iwe_template::{agent_info, installable_files, is_installed, AgentInfo, Manifest};

use super::fixture::{valid_template, with_manifest_line, without};

#[test]
fn everything_installs_except_the_named_files() {
    let entries = valid_template();
    let manifest = Manifest::from_entries(&entries).unwrap();
    let installed: Vec<&str> = installable_files(&entries, &manifest)
        .iter()
        .map(|entry| entry.path.as_str())
        .collect();
    assert_eq!(
        installed,
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
        ]
    );
}

#[test]
fn license_files_and_excluded_folders_are_not_installed() {
    let manifest = Manifest::from_entries(&valid_template()).unwrap();
    for path in [
        "LICENSE",
        "LICENSE.md",
        "LICENSE.txt",
        "template.toml",
        "preview.png",
        ".github/workflows/x.yml",
        "README.md",
        "drafts/a.md",
        "drafts/deep/b.md",
    ] {
        assert!(!is_installed(path, &manifest), "{path}");
    }
    for path in [
        "data/README.md",
        "data/LICENSE.md",
        "data/preview.png",
        "data/drafts.md",
    ] {
        assert!(is_installed(path, &manifest), "{path}");
    }
}

#[test]
fn agent_info_names_instructions_and_skill_folders() {
    let entries = valid_template();
    let manifest = Manifest::from_entries(&entries).unwrap();
    assert_eq!(
        agent_info(&installable_files(&entries, &manifest)),
        AgentInfo {
            instructions: true,
            skills: vec!["plan".to_string(), "review".to_string()],
        }
    );
    let excluding = with_manifest_line(
        without(without(valid_template(), "AGENTS.md"), "CLAUDE.md"),
        "exclude = [\"README.md\", \"drafts\"]",
        "exclude = [\".claude/skills/review\"]",
    );
    let manifest = Manifest::from_entries(&excluding).unwrap();
    assert_eq!(
        agent_info(&installable_files(&excluding, &manifest)),
        AgentInfo {
            instructions: false,
            skills: vec!["plan".to_string()],
        }
    );
}
