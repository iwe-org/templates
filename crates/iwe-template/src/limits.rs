pub const FORMAT: u32 = 1;

pub const MAX_FILES: usize = 1500;
pub const MAX_FILE_BYTES: u64 = 1024 * 1024;
pub const MAX_TOTAL_BYTES: u64 = 5 * 1024 * 1024;
pub const MAX_PREVIEW_BYTES: u64 = 500 * 1024;
pub const MAX_SCRIPT_LINE_CHARS: usize = 500;
pub const MAX_ARCHIVE_BYTES: u64 = 8 * 1024 * 1024;
pub const MAX_UNPACKED_STREAM_BYTES: u64 = 16 * 1024 * 1024;

pub const MIN_ID_CHARS: usize = 3;
pub const MAX_ID_CHARS: usize = 40;
pub const MAX_NAME_CHARS: usize = 40;
pub const MAX_DESCRIPTION_CHARS: usize = 140;
pub const MIN_CONFIG_VERSION: i64 = 4;
pub const REV_CHARS: usize = 40;

pub const ACCEPTED_LICENSES: &[&str] = &["MIT", "Apache-2.0", "CC0-1.0", "CC-BY-4.0"];

pub const ALLOWED_EXTENSIONS: &[&str] = &[
    "md", "html", "css", "js", "json", "jsonl", "csv", "tsv", "png", "jpg", "gif", "webp", "svg",
];

pub const ALLOWED_CONFIG_KEYS: &[&str] = &[
    "version",
    "format",
    "markdown",
    "djot",
    "workspace",
    "completion",
    "search",
    "templates",
    "actions",
    "schemas",
];

pub const MANIFEST_PATH: &str = "template.toml";
pub const PREVIEW_PATH: &str = "preview.png";
pub const CONFIG_PATH: &str = ".iwe/config.toml";
pub const GITIGNORE_PATH: &str = ".gitignore";
pub const SCHEMAS_PREFIX: &str = ".iwe/schemas/";
pub const SKILLS_PREFIX: &str = ".claude/skills/";
pub const GITHUB_PREFIX: &str = ".github/";
pub const AGENT_INSTRUCTION_FILES: &[&str] = &["AGENTS.md", "CLAUDE.md"];
pub const LICENSE_FILES: &[&str] = &["LICENSE", "LICENSE.md", "LICENSE.txt"];
