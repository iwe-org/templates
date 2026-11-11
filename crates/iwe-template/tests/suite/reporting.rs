use iwe_template::{compare_url, report, ReportOptions, TemplateEntry};

use super::fixture::{assert_snapshot, valid_template, with, without, APP_SCRIPT, BOARD_PAGE};

fn updated_template() -> Vec<TemplateEntry> {
    let entries = without(valid_template(), "data/notes/first.md");
    let entries = with(
        entries,
        "data/notes/second.md",
        "# Second\n\nRead <https://example.org/second>.\n",
    );
    let entries = with(
        entries,
        ".claude/skills/plan/SKILL.md",
        "# Plan\n\nWrite a plan, then push it to the remote.\n",
    );
    let entries = with(
        entries,
        ".claude/skills/ship/SKILL.md",
        "# Ship\n\nTag the release.\n",
    );
    let entries = with(
        entries,
        "AGENTS.md",
        "# Agents\n\nKeep notes short and dated.\n",
    );
    let entries = with(
        entries,
        "data/assets/app.js",
        format!("{APP_SCRIPT}render.version = 2\n"),
    );
    let entries = with(
        entries,
        "data/board.html",
        BOARD_PAGE.replace(
            "Triage the new notes",
            "Archive every note older than a year",
        ),
    );
    with(
        entries,
        "template.toml",
        super::fixture::MANIFEST.replace("1.2.0", "1.3.0"),
    )
}

#[test]
fn the_report_for_a_new_template() {
    let text = report(&valid_template(), None, &ReportOptions::default());
    assert_snapshot("report-new.md", &text);
}

#[test]
fn the_report_for_an_update() {
    let options = ReportOptions {
        compare_url: Some(compare_url(
            "https://github.com/example/field-notes",
            "0123456789abcdef0123456789abcdef01234567",
            "89abcdef0123456789abcdef0123456789abcdef",
        )),
    };
    let text = report(&updated_template(), Some(&valid_template()), &options);
    assert_snapshot("report-update.md", &text);
}

#[test]
fn the_report_names_what_a_reviewer_must_read() {
    let text = report(&valid_template(), None, &ReportOptions::default());
    for expected in [
        "`AGENTS.md` (added)",
        "`CLAUDE.md` (added)",
        "skill `plan`: `.claude/skills/plan/SKILL.md` (added)",
        "Summarize this week's notes",
        "Triage the new notes",
        "`Review ${key} and list what is missing`",
        "`data/assets/app.js`, file",
        "`data/board.html` line 5, inline, 0 lines, loads `assets/app.js`",
        "`data/index.md` line 7, inline",
        "`https://example.com/guide`: `data/index.md`",
    ] {
        assert!(text.contains(expected), "missing {expected:?} in\n{text}");
    }
    assert!(!text.contains(".github"));
}

#[test]
fn the_update_report_marks_what_is_new() {
    let text = report(
        &updated_template(),
        Some(&valid_template()),
        &ReportOptions::default(),
    );
    for expected in [
        "2 added, 5 changed, 1 removed",
        "skill `ship`: `.claude/skills/ship/SKILL.md` (added)",
        "skill `plan`: `.claude/skills/plan/SKILL.md` (changed)",
        "`AGENTS.md` (changed)",
        "`data-prompt` in `data/board.html` (new)",
        "`data/assets/app.js`, file, 4 lines (new or changed)",
        "(unchanged)",
        "`https://example.org/second` (new): `data/notes/second.md`",
        "Update from version `1.2.0`",
    ] {
        assert!(text.contains(expected), "missing {expected:?} in\n{text}");
    }
    assert!(!text.contains("`CLAUDE.md` ("));
}
