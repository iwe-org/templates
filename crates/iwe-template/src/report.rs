use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::path::Path;

use crate::entry::{is_outside_template, read_dir, EntryKind, ReadError, TemplateEntry};
use crate::manifest::Manifest;
use crate::paths::{extension, is_root_agent_instruction, skill_name};
use crate::scripts::{attribute_values, scripts_in, Script};

const BINARY_EXTENSIONS: &[&str] = &["png", "jpg", "gif", "webp"];
const PROMPT_EXTENSIONS: &[&str] = &["md", "html", "js", "svg"];
const PROMPT_ATTRIBUTE: &str = "data-prompt";
const PROMPT_CALL: &str = "iwe.prompt(";
const URL_SCHEMES: &[&str] = &["https://", "http://"];

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReportOptions {
    pub compare_url: Option<String>,
}

pub fn compare_url(repository: &str, previous_rev: &str, rev: &str) -> String {
    format!(
        "{}/compare/{previous_rev}...{rev}",
        repository.trim_end_matches('/')
    )
}

pub fn report_dir(
    dir: &Path,
    since: Option<&Path>,
    options: &ReportOptions,
) -> Result<String, ReadError> {
    let current = read_dir(dir)?;
    let previous = since.map(read_dir).transpose()?;
    Ok(report(&current, previous.as_deref(), options))
}

pub fn report(
    current: &[TemplateEntry],
    previous: Option<&[TemplateEntry]>,
    options: &ReportOptions,
) -> String {
    let current = Tree::new(current);
    let previous = previous.map(Tree::new);
    let mut out = String::new();
    write_heading(&mut out, &current, previous.as_ref(), options);
    write_files(&mut out, &current, previous.as_ref());
    write_agent_content(&mut out, &current, previous.as_ref());
    write_prompts(&mut out, &current, previous.as_ref());
    write_scripts(&mut out, &current, previous.as_ref());
    write_urls(&mut out, &current, previous.as_ref());
    out
}

struct Tree<'a> {
    files: BTreeMap<&'a str, &'a TemplateEntry>,
}

impl<'a> Tree<'a> {
    fn new(entries: &'a [TemplateEntry]) -> Self {
        Tree {
            files: entries
                .iter()
                .filter(|entry| !is_outside_template(&entry.path))
                .map(|entry| (entry.path.as_str(), entry))
                .collect(),
        }
    }

    fn manifest(&self) -> Option<Manifest> {
        let entries: Vec<TemplateEntry> =
            self.files.values().map(|entry| (*entry).clone()).collect();
        Manifest::from_entries(&entries).ok()
    }

    fn texts(&self) -> impl Iterator<Item = (&'a str, String)> + '_ {
        self.files
            .iter()
            .filter(|(path, entry)| entry.kind == EntryKind::File && is_text(path))
            .map(|(path, entry)| (*path, String::from_utf8_lossy(&entry.bytes).into_owned()))
    }

    fn differs(&self, path: &str, other: Option<&Tree>) -> Option<Change> {
        let Some(other) = other else {
            return Some(Change::Added);
        };
        match (self.files.get(path), other.files.get(path)) {
            (Some(_), None) => Some(Change::Added),
            (Some(now), Some(before)) if now.bytes != before.bytes || now.kind != before.kind => {
                Some(Change::Changed)
            }
            _ => None,
        }
    }

    fn prompts(&self) -> Vec<Prompt> {
        self.texts()
            .filter(|(path, _)| extension(path).is_some_and(|ext| PROMPT_EXTENSIONS.contains(&ext)))
            .flat_map(|(path, text)| prompts_in(path, &text))
            .collect()
    }

    fn scripts(&self) -> Vec<Script> {
        self.texts()
            .flat_map(|(path, text)| scripts_in(path, &text))
            .collect()
    }

    fn urls(&self) -> BTreeMap<String, BTreeSet<&'a str>> {
        let mut urls: BTreeMap<String, BTreeSet<&'a str>> = BTreeMap::new();
        for (path, text) in self.texts() {
            for url in urls_in(&text) {
                urls.entry(url).or_default().insert(path);
            }
        }
        urls
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Change {
    Added,
    Changed,
}

impl Change {
    fn label(self) -> &'static str {
        match self {
            Change::Added => "added",
            Change::Changed => "changed",
        }
    }
}

struct Prompt {
    path: String,
    kind: &'static str,
    text: String,
}

fn is_text(path: &str) -> bool {
    !extension(path).is_some_and(|ext| BINARY_EXTENSIONS.contains(&ext))
}

fn write_heading(
    out: &mut String,
    current: &Tree,
    previous: Option<&Tree>,
    options: &ReportOptions,
) {
    match current.manifest() {
        Some(manifest) => {
            let _ = writeln!(
                out,
                "# Template report: {} {}\n",
                code(&manifest.id),
                code(&manifest.version)
            );
            let _ = writeln!(out, "- Name: {}", code(&manifest.name));
            let _ = writeln!(out, "- Description: {}", code(&manifest.description));
            let _ = writeln!(out, "- Authors: {}", code(&manifest.authors.join(", ")));
            let _ = writeln!(out, "- License: {}", code(&manifest.license));
            if !manifest.exclude.is_empty() {
                let _ = writeln!(
                    out,
                    "- Not installed: {}",
                    code(&manifest.exclude.join(", "))
                );
            }
        }
        None => {
            let _ = writeln!(out, "# Template report\n\n- No readable `template.toml`");
        }
    }
    match previous.map(Tree::manifest) {
        Some(Some(before)) => {
            let _ = writeln!(out, "- Update from version {}", code(&before.version));
        }
        Some(None) => {
            let _ = writeln!(
                out,
                "- Update from a revision without a readable `template.toml`"
            );
        }
        None => {
            let _ = writeln!(out, "- New template: read every file");
        }
    }
    if let Some(url) = &options.compare_url {
        let _ = writeln!(out, "- Compare: <{url}>");
    }
}

fn write_files(out: &mut String, current: &Tree, previous: Option<&Tree>) {
    let added: Vec<&str> = current
        .files
        .keys()
        .filter(|path| current.differs(path, previous) == Some(Change::Added))
        .copied()
        .collect();
    let changed: Vec<&str> = current
        .files
        .keys()
        .filter(|path| current.differs(path, previous) == Some(Change::Changed))
        .copied()
        .collect();
    let removed: Vec<&str> = previous
        .map(|before| {
            before
                .files
                .keys()
                .filter(|path| !current.files.contains_key(*path))
                .copied()
                .collect()
        })
        .unwrap_or_default();
    let _ = writeln!(
        out,
        "\n## Files\n\n{} added, {} changed, {} removed.",
        added.len(),
        changed.len(),
        removed.len()
    );
    write_path_list(out, "Added", &added);
    write_path_list(out, "Changed", &changed);
    write_path_list(out, "Removed", &removed);
}

fn write_path_list(out: &mut String, title: &str, paths: &[&str]) {
    if paths.is_empty() {
        return;
    }
    let _ = writeln!(out, "\n### {title}\n");
    for path in paths {
        let _ = writeln!(out, "- {}", code(path));
    }
}

fn write_agent_content(out: &mut String, current: &Tree, previous: Option<&Tree>) {
    let _ = writeln!(out, "\n## Agent instructions and skills\n");
    let touched: Vec<(&str, Change)> = current
        .files
        .keys()
        .filter(|path| is_root_agent_instruction(path) || skill_name(path).is_some())
        .filter_map(|path| {
            current
                .differs(path, previous)
                .map(|change| (*path, change))
        })
        .collect();
    let removed: Vec<&str> = previous
        .map(|before| {
            before
                .files
                .keys()
                .filter(|path| is_root_agent_instruction(path) || skill_name(path).is_some())
                .filter(|path| !current.files.contains_key(*path))
                .copied()
                .collect()
        })
        .unwrap_or_default();
    if touched.is_empty() && removed.is_empty() {
        let _ = writeln!(out, "None added or changed.");
        return;
    }
    for (path, change) in &touched {
        let _ = writeln!(out, "- {} ({})", agent_label(path), change.label());
    }
    for path in &removed {
        let _ = writeln!(out, "- {} (removed)", agent_label(path));
    }
    for (path, change) in &touched {
        let text = String::from_utf8_lossy(&current.files[path].bytes).into_owned();
        write_details(
            out,
            &format!("{path} ({})", change.label()),
            &text,
            "markdown",
        );
    }
}

fn agent_label(path: &str) -> String {
    match skill_name(path) {
        Some(skill) => format!("skill {}: {}", code(skill), code(path)),
        None => code(path),
    }
}

fn write_prompts(out: &mut String, current: &Tree, previous: Option<&Tree>) {
    let _ = writeln!(out, "\n## Prompts\n");
    let prompts = current.prompts();
    if prompts.is_empty() {
        let _ = writeln!(out, "None.");
        return;
    }
    let known: Option<BTreeSet<String>> = previous.map(|before| {
        before
            .prompts()
            .into_iter()
            .map(|prompt| prompt.text)
            .collect()
    });
    for prompt in prompts {
        let marker = match &known {
            Some(known) if !known.contains(&prompt.text) => " (new)",
            _ => "",
        };
        let _ = writeln!(
            out,
            "- {} in {}{marker}\n",
            code(prompt.kind),
            code(&prompt.path)
        );
        let _ = writeln!(out, "{}\n", indented(&fenced(&prompt.text, "text")));
    }
}

fn write_scripts(out: &mut String, current: &Tree, previous: Option<&Tree>) {
    let _ = writeln!(out, "\n## Scripts\n");
    let scripts = current.scripts();
    if scripts.is_empty() {
        let _ = writeln!(out, "None.");
        return;
    }
    let known: Option<BTreeSet<(String, String)>> = previous.map(|before| {
        before
            .scripts()
            .into_iter()
            .map(|script| (script.path, script.source))
            .collect()
    });
    let is_unchanged = |script: &Script| {
        known
            .as_ref()
            .is_some_and(|known| known.contains(&(script.path.clone(), script.source.clone())))
    };
    for script in &scripts {
        let _ = writeln!(
            out,
            "- {}",
            script_label(script, is_unchanged(script), known.is_some())
        );
    }
    for script in scripts.iter().filter(|script| !is_unchanged(script)) {
        if script.source.trim().is_empty() {
            continue;
        }
        write_details(
            out,
            &format!("{} line {}", script.path, script.line),
            &script.source,
            "js",
        );
    }
}

fn script_label(script: &Script, unchanged: bool, is_update: bool) -> String {
    let mut label = if script.inline {
        format!("{} line {}, inline", code(&script.path), script.line)
    } else {
        format!("{}, file", code(&script.path))
    };
    let line_count = script.source.trim().lines().count();
    let noun = if line_count == 1 { "line" } else { "lines" };
    let _ = write!(label, ", {line_count} {noun}");
    if let Some(src) = &script.src {
        let _ = write!(label, ", loads {}", code(src));
    }
    if is_update {
        label.push_str(if unchanged {
            " (unchanged)"
        } else {
            " (new or changed)"
        });
    }
    label
}

fn write_urls(out: &mut String, current: &Tree, previous: Option<&Tree>) {
    let _ = writeln!(out, "\n## External URLs\n");
    let urls = current.urls();
    if urls.is_empty() {
        let _ = writeln!(out, "None.");
        return;
    }
    let known = previous.map(Tree::urls);
    for (url, paths) in &urls {
        let marker = match &known {
            Some(known) if !known.contains_key(url) => " (new)",
            _ => "",
        };
        let places: Vec<String> = paths.iter().map(|path| code(path)).collect();
        let _ = writeln!(out, "- {}{marker}: {}", code(url), places.join(", "));
    }
}

fn write_details(out: &mut String, summary: &str, body: &str, language: &str) {
    let _ = writeln!(
        out,
        "\n<details>\n<summary>{}</summary>\n\n{}\n\n</details>",
        escape_html(summary),
        fenced(body, language)
    );
}

fn prompts_in(path: &str, text: &str) -> Vec<Prompt> {
    let declared = attribute_values(text, PROMPT_ATTRIBUTE)
        .into_iter()
        .map(|value| Prompt {
            path: path.to_string(),
            kind: PROMPT_ATTRIBUTE,
            text: value,
        });
    let called = prompt_calls(text).into_iter().map(|argument| Prompt {
        path: path.to_string(),
        kind: "iwe.prompt",
        text: argument,
    });
    declared.chain(called).collect()
}

fn prompt_calls(text: &str) -> Vec<String> {
    let mut calls = Vec::new();
    let mut cursor = 0;
    while let Some(found) = text[cursor..].find(PROMPT_CALL) {
        let start = cursor + found + PROMPT_CALL.len();
        let end = end_of_call(text, start);
        calls.push(text[start..end].trim().to_string());
        cursor = start;
    }
    calls
}

fn end_of_call(text: &str, start: usize) -> usize {
    let mut depth = 1usize;
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for (offset, c) in text[start..].char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match quote {
            Some(_) if c == '\\' => escaped = true,
            Some(open) if c == open => quote = None,
            Some(_) => {}
            None => match c {
                '"' | '\'' | '`' => quote = Some(c),
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        return start + offset;
                    }
                }
                _ => {}
            },
        }
    }
    text[start..]
        .find('\n')
        .map_or(text.len(), |offset| start + offset)
}

fn urls_in(text: &str) -> BTreeSet<String> {
    let mut urls = BTreeSet::new();
    for scheme in URL_SCHEMES {
        let mut cursor = 0;
        while let Some(found) = text[cursor..].find(scheme) {
            let start = cursor + found;
            let length = text[start..]
                .find(|c: char| {
                    c.is_whitespace() || c.is_control() || "\"'<>()[]{}`\\|^".contains(c)
                })
                .unwrap_or(text.len() - start);
            let url =
                text[start..start + length].trim_end_matches(['.', ',', ';', ':', '!', '?', '*']);
            if url.len() > scheme.len() {
                urls.insert(url.to_string());
            }
            cursor = start + scheme.len();
        }
    }
    urls
}

fn code(text: &str) -> String {
    let cleaned: String = text
        .chars()
        .map(|c| if c == '`' || c.is_control() { ' ' } else { c })
        .collect();
    format!("`{cleaned}`")
}

fn fenced(body: &str, language: &str) -> String {
    let longest_run = body
        .split(|c: char| c != '`')
        .map(str::len)
        .max()
        .unwrap_or(0);
    let fence = "`".repeat(longest_run.max(2) + 1);
    format!("{fence}{language}\n{}\n{fence}", body.trim_matches('\n'))
}

fn indented(block: &str) -> String {
    block
        .lines()
        .map(|line| format!("  {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
