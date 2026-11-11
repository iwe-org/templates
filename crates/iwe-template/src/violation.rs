use std::fmt;

use crate::limits::{
    ACCEPTED_LICENSES, FORMAT, MAX_DESCRIPTION_CHARS, MAX_FILES, MAX_FILE_BYTES, MAX_ID_CHARS,
    MAX_NAME_CHARS, MAX_PREVIEW_BYTES, MAX_SCRIPT_LINE_CHARS, MAX_TOTAL_BYTES, MIN_CONFIG_VERSION,
    MIN_ID_CHARS,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Violation {
    EscapingPath {
        path: String,
    },
    DuplicatePath {
        path: String,
    },
    Symlink {
        path: String,
    },
    HardLink {
        path: String,
    },
    UnsupportedFileKind {
        path: String,
    },
    ExtensionNotAllowed {
        path: String,
    },
    DotPathNotAllowed {
        path: String,
    },
    AgentInstructionsOutsideRoot {
        path: String,
    },
    TooManyFiles {
        count: usize,
    },
    FileTooLarge {
        path: String,
        size: u64,
    },
    TotalTooLarge {
        size: u64,
    },
    PreviewTooLarge {
        size: u64,
    },
    ScriptLineTooLong {
        path: String,
        line: usize,
        length: usize,
    },
    ConfigMissing,
    ConfigInvalid {
        message: String,
    },
    ConfigKeyNotAllowed {
        key: String,
    },
    ConfigVersion {
        found: Option<i64>,
    },
    ConfigLibraryTable,
    ConfigCommands,
    ConfigTransformAction {
        action: String,
    },
    ConfigActionCommand {
        action: String,
    },
    ConfigWorkspacePath {
        path: String,
    },
    ManifestMissing,
    ManifestInvalid {
        message: String,
    },
    ManifestFormat {
        found: u32,
    },
    ManifestId {
        id: String,
    },
    ManifestName {
        name: String,
    },
    ManifestDescription {
        description: String,
    },
    ManifestVersion {
        version: String,
    },
    ManifestAuthors,
    ManifestLicense {
        license: String,
    },
    ManifestExclude {
        pattern: String,
    },
    RegistryIdMismatch {
        expected: String,
        found: String,
    },
    RegistryVersionMismatch {
        expected: String,
        found: String,
    },
    VersionNotIncreased {
        previous: String,
        found: String,
    },
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Violation::EscapingPath { path } => {
                write!(f, "{path:?}: the path leaves the template or is not a plain relative path")
            }
            Violation::DuplicatePath { path } => {
                write!(f, "{path}: the path appears more than once, ignoring letter case")
            }
            Violation::Symlink { path } => write!(f, "{path}: symlinks are not allowed"),
            Violation::HardLink { path } => write!(f, "{path}: hard links are not allowed"),
            Violation::UnsupportedFileKind { path } => {
                write!(f, "{path}: only regular files are allowed")
            }
            Violation::ExtensionNotAllowed { path } => {
                write!(f, "{path}: this file type is not allowed in a template")
            }
            Violation::DotPathNotAllowed { path } => write!(
                f,
                "{path}: the only dot paths a template may hold are .iwe/config.toml, .iwe/schemas/*.yaml, .claude/skills/**/*.md and .gitignore"
            ),
            Violation::AgentInstructionsOutsideRoot { path } => write!(
                f,
                "{path}: AGENTS.md and CLAUDE.md are allowed at the template root only, spelled exactly so"
            ),
            Violation::TooManyFiles { count } => {
                write!(f, "the template has {count} files, more than {MAX_FILES}")
            }
            Violation::FileTooLarge { path, size } => {
                write!(f, "{path}: {size} bytes, more than {MAX_FILE_BYTES}")
            }
            Violation::TotalTooLarge { size } => {
                write!(f, "the template is {size} bytes in total, more than {MAX_TOTAL_BYTES}")
            }
            Violation::PreviewTooLarge { size } => {
                write!(f, "preview.png: {size} bytes, more than {MAX_PREVIEW_BYTES}")
            }
            Violation::ScriptLineTooLong { path, line, length } => write!(
                f,
                "{path}:{line}: a script line of {length} characters, more than {MAX_SCRIPT_LINE_CHARS}; minified or bundled JavaScript is not allowed"
            ),
            Violation::ConfigMissing => write!(f, ".iwe/config.toml is required"),
            Violation::ConfigInvalid { message } => {
                write!(f, ".iwe/config.toml does not parse: {message}")
            }
            Violation::ConfigKeyNotAllowed { key } => {
                write!(f, ".iwe/config.toml: the top-level key `{key}` is not allowed")
            }
            Violation::ConfigVersion { found: Some(found) } => write!(
                f,
                ".iwe/config.toml: version is {found}, it must be {MIN_CONFIG_VERSION} or later"
            ),
            Violation::ConfigVersion { found: None } => write!(
                f,
                ".iwe/config.toml: version must be an integer, {MIN_CONFIG_VERSION} or later"
            ),
            Violation::ConfigLibraryTable => write!(
                f,
                ".iwe/config.toml: the [library] table is not allowed, use [workspace]"
            ),
            Violation::ConfigCommands => {
                write!(f, ".iwe/config.toml: [commands] is not allowed")
            }
            Violation::ConfigTransformAction { action } => write!(
                f,
                ".iwe/config.toml: action `{action}` is of type transform, which is not allowed"
            ),
            Violation::ConfigActionCommand { action } => write!(
                f,
                ".iwe/config.toml: action `{action}` carries a command, which is not allowed"
            ),
            Violation::ConfigWorkspacePath { path } => write!(
                f,
                ".iwe/config.toml: workspace.path `{path}` must be empty or a relative path, without `..`, to a folder the template contains"
            ),
            Violation::ManifestMissing => write!(f, "template.toml is required"),
            Violation::ManifestInvalid { message } => {
                write!(f, "template.toml does not parse: {message}")
            }
            Violation::ManifestFormat { found } => {
                write!(f, "template.toml: format is {found}, it must be {FORMAT}")
            }
            Violation::ManifestId { id } => write!(
                f,
                "template.toml: id `{id}` must be lowercase kebab-case, {MIN_ID_CHARS} to {MAX_ID_CHARS} characters"
            ),
            Violation::ManifestName { name } => write!(
                f,
                "template.toml: name `{name}` must have 1 to {MAX_NAME_CHARS} characters on one line"
            ),
            Violation::ManifestDescription { description } => write!(
                f,
                "template.toml: description `{description}` must be one plain-text sentence of at most {MAX_DESCRIPTION_CHARS} characters"
            ),
            Violation::ManifestVersion { version } => {
                write!(f, "template.toml: version `{version}` is not a semver version")
            }
            Violation::ManifestAuthors => {
                write!(f, "template.toml: authors must name at least one author")
            }
            Violation::ManifestLicense { license } => write!(
                f,
                "template.toml: license `{license}` is not one of {}",
                ACCEPTED_LICENSES.join(", ")
            ),
            Violation::ManifestExclude { pattern } => write!(
                f,
                "template.toml: exclude pattern `{pattern}` must be a valid relative glob that keeps .iwe/config.toml"
            ),
            Violation::RegistryIdMismatch { expected, found } => write!(
                f,
                "template.toml: id is `{found}`, the registry entry says `{expected}`"
            ),
            Violation::RegistryVersionMismatch { expected, found } => write!(
                f,
                "template.toml: version is `{found}`, the registry entry says `{expected}`"
            ),
            Violation::VersionNotIncreased { previous, found } => write!(
                f,
                "template.toml: version `{found}` must be greater than the published `{previous}`"
            ),
        }
    }
}
