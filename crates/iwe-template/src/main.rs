use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::SystemTime;

use clap::{Parser, Subcommand};
use iwe_template::{
    archive_file_name, assemble_index, compare_url, pack, report_dir, utc_date, utc_timestamp,
    validate_dir, EntryRecord, Expected, Origin, Registry, ReportOptions, DEFAULT_RELEASE_BASE_URL,
    ENTRY_RECORD_FILE, PREVIEW_FILE,
};

#[derive(Parser)]
#[command(
    name = "iwe-template",
    version,
    about = "Validate, review and package iWe workspace templates"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    #[command(about = "Validate a template folder against the structural rules")]
    Check {
        dir: PathBuf,
        #[arg(long, requires = "version", help = "The id the registry entry names")]
        id: Option<String>,
        #[arg(long, requires = "id", help = "The version the registry entry names")]
        version: Option<String>,
        #[arg(
            long,
            requires = "id",
            help = "The version published before, which this one must exceed"
        )]
        previous_version: Option<String>,
    },
    #[command(about = "Write the reviewer's summary of a template as markdown")]
    Report {
        dir: PathBuf,
        #[arg(long, help = "The previous revision's folder, for an update")]
        since: Option<PathBuf>,
        #[arg(long, requires_all = ["rev", "since_rev"], help = "The template repository, for the compare link")]
        repository: Option<String>,
        #[arg(long, help = "The revision under review, for the compare link")]
        rev: Option<String>,
        #[arg(long, help = "The previous revision, for the compare link")]
        since_rev: Option<String>,
    },
    #[command(about = "Write <id>-<version>.tar.gz, entry.json and preview.png")]
    Pack {
        dir: PathBuf,
        #[arg(long, help = "The folder the archive and its record are written to")]
        out: PathBuf,
        #[arg(
            long,
            default_value = "",
            help = "The template repository recorded in entry.json"
        )]
        repository: String,
        #[arg(long, default_value = "", help = "The revision recorded in entry.json")]
        rev: String,
        #[arg(
            long,
            help = "The publication date, YYYY-MM-DD; today in UTC by default"
        )]
        published: Option<String>,
    },
    #[command(about = "Assemble index.json from templates.toml and the published entry records")]
    Index {
        #[arg(long, default_value = "templates.toml", help = "The registry file")]
        registry: PathBuf,
        #[arg(
            long,
            help = "A folder holding the entry.json records, searched recursively"
        )]
        entries: PathBuf,
        #[arg(long, help = "A JSON object mapping template ids to star counts")]
        stars: Option<PathBuf>,
        #[arg(long, default_value = DEFAULT_RELEASE_BASE_URL, help = "The release download URL the archive and preview links start with")]
        base_url: String,
        #[arg(long, help = "The generation time, RFC 3339; now in UTC by default")]
        generated: Option<String>,
        #[arg(long, help = "The file to write; standard output by default")]
        out: Option<PathBuf>,
    },
    #[command(about = "Validate templates.toml and print its entries as JSON")]
    Entries {
        #[arg(default_value = "templates.toml")]
        registry: PathBuf,
        #[arg(
            long,
            help = "An earlier templates.toml; only entries added or changed since are printed"
        )]
        since: Option<PathBuf>,
    },
}

fn main() -> ExitCode {
    match run(Cli::parse().command) {
        Ok(code) => code,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::from(2)
        }
    }
}

fn run(command: Command) -> Result<ExitCode, String> {
    match command {
        Command::Check {
            dir,
            id,
            version,
            previous_version,
        } => {
            let expected = id.zip(version).map(|(id, version)| Expected {
                id,
                version,
                previous_version,
            });
            let violations = validate_dir(&dir, expected.as_ref()).map_err(display)?;
            for violation in &violations {
                println!("{violation}");
            }
            if violations.is_empty() {
                println!("ok: {} passes every structural rule", dir.display());
                Ok(ExitCode::SUCCESS)
            } else {
                eprintln!("{} violation(s)", violations.len());
                Ok(ExitCode::FAILURE)
            }
        }
        Command::Report {
            dir,
            since,
            repository,
            rev,
            since_rev,
        } => {
            let options = ReportOptions {
                compare_url: match (repository, since_rev, rev) {
                    (Some(repository), Some(since_rev), Some(rev)) => {
                        Some(compare_url(&repository, &since_rev, &rev))
                    }
                    _ => None,
                },
            };
            print!(
                "{}",
                report_dir(&dir, since.as_deref(), &options).map_err(display)?
            );
            Ok(ExitCode::SUCCESS)
        }
        Command::Pack {
            dir,
            out,
            repository,
            rev,
            published,
        } => {
            let origin = Origin {
                repository,
                rev,
                published: published.unwrap_or_else(|| utc_date(SystemTime::now())),
            };
            let packed = match pack(&dir, &origin) {
                Ok(packed) => packed,
                Err(iwe_template::PackError::Invalid(violations)) => {
                    for violation in &violations {
                        println!("{violation}");
                    }
                    eprintln!("{} violation(s)", violations.len());
                    return Ok(ExitCode::FAILURE);
                }
                Err(error) => return Err(error.to_string()),
            };
            fs::create_dir_all(&out).map_err(display)?;
            let archive_path =
                out.join(archive_file_name(&packed.record.id, &packed.record.version));
            fs::write(&archive_path, &packed.archive).map_err(display)?;
            write_json(&out.join(ENTRY_RECORD_FILE), &packed.record)?;
            if let Some(preview) = &packed.preview {
                fs::write(out.join(PREVIEW_FILE), preview).map_err(display)?;
            }
            println!("{}", archive_path.display());
            Ok(ExitCode::SUCCESS)
        }
        Command::Index {
            registry,
            entries,
            stars,
            base_url,
            generated,
            out,
        } => {
            let registry = read_registry(&registry)?;
            let records = read_records(&entries)?;
            let stars: BTreeMap<String, u64> = match stars {
                Some(path) => serde_json::from_str(&read_text(&path)?)
                    .map_err(|error| format!("{}: {error}", path.display()))?,
                None => BTreeMap::new(),
            };
            let generated = generated.unwrap_or_else(|| utc_timestamp(SystemTime::now()));
            let index = assemble_index(&registry, &records, &stars, &base_url, &generated)
                .map_err(display)?;
            match out {
                Some(path) => write_json(&path, &index)?,
                None => println!("{}", to_json(&index)?),
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Entries { registry, since } => {
            let current = read_registry(&registry)?;
            let changes = match since {
                Some(path) => current.changes_since(&read_registry_unchecked(&path)?),
                None => current.all_entries(),
            };
            println!("{}", to_json(&changes)?);
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn display(error: impl std::fmt::Display) -> String {
    error.to_string()
}

fn read_text(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))
}

fn read_registry_unchecked(path: &Path) -> Result<Registry, String> {
    Registry::parse(&read_text(path)?).map_err(|error| format!("{}: {error}", path.display()))
}

fn read_registry(path: &Path) -> Result<Registry, String> {
    let registry = read_registry_unchecked(path)?;
    let problems = registry.problems();
    if problems.is_empty() {
        return Ok(registry);
    }
    let lines: Vec<String> = problems.iter().map(ToString::to_string).collect();
    Err(format!("{}:\n{}", path.display(), lines.join("\n")))
}

fn read_records(dir: &Path) -> Result<Vec<EntryRecord>, String> {
    let mut records = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(folder) = pending.pop() {
        let listing =
            fs::read_dir(&folder).map_err(|error| format!("{}: {error}", folder.display()))?;
        for item in listing {
            let path = item.map_err(display)?.path();
            if path.is_dir() {
                pending.push(path);
            } else if path
                .file_name()
                .is_some_and(|name| name == ENTRY_RECORD_FILE)
            {
                let record = serde_json::from_str(&read_text(&path)?)
                    .map_err(|error| format!("{}: {error}", path.display()))?;
                records.push(record);
            }
        }
    }
    Ok(records)
}

fn to_json(value: &impl serde::Serialize) -> Result<String, String> {
    serde_json::to_string_pretty(value).map_err(display)
}

fn write_json(path: &Path, value: &impl serde::Serialize) -> Result<(), String> {
    fs::write(path, to_json(value)? + "\n").map_err(|error| format!("{}: {error}", path.display()))
}
