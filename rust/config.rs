use crate::model::{Language, Settings};
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::{Path, PathBuf};
use toml::{Table, Value};

const SECTION: &str = "[tool.lint-my-headers]";

#[derive(Parser, Debug)]
#[command(
    name = "lmh",
    about = "Lint source copyright and license headers and conservatively refresh recognized years."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Check copyright and license headers without writing source files.
    Check(Options),
    /// Refresh stale years and insert missing headers with a declared creation year.
    Fix(Options),
}

impl Command {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Check(_) => "check",
            Self::Fix(_) => "fix",
        }
    }
    pub fn options(&self) -> &Options {
        match self {
            Self::Check(o) | Self::Fix(o) => o,
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq, Default)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
}

#[derive(Args, Debug, Default)]
pub struct Options {
    #[arg(
        long,
        help = "Configuration file; defaults to the nearest project configuration"
    )]
    pub config: Option<PathBuf>,
    #[arg(long, value_enum, value_delimiter = ',')]
    pub languages: Option<Vec<Language>>,
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long, allow_negative_numbers = true)]
    pub starting_year: Option<i32>,
    /// Declared first copyright year for missing headers; never inferred.
    #[arg(long, allow_negative_numbers = true)]
    pub creation_year: Option<i32>,
    #[arg(long)]
    pub license: Option<String>,
    #[arg(long)]
    pub license_notice: Option<PathBuf>,
    #[arg(long, hide = true)]
    pub folders: Option<String>,
    #[arg(long)]
    pub ignore_files: Option<String>,
    #[arg(long)]
    pub ignore_folders: Option<String>,
    #[arg(long, value_enum, default_value = "text")]
    pub output_format: OutputFormat,
    /// Explicit files or folders; overrides configured paths.
    pub paths: Vec<PathBuf>,
}

fn find_config(explicit: Option<&Path>, cwd: &Path) -> Result<(Option<PathBuf>, Table), String> {
    if let Some(path) = explicit {
        if !path.is_file() {
            return Err(format!("Invalid configuration path: {}", path.display()));
        }
        return Ok((
            Some(path.canonicalize().map_err(|e| e.to_string())?),
            config_table(path)?.unwrap_or_default(),
        ));
    }
    for parent in cwd.ancestors() {
        for name in [".lmh.toml", "pyproject.toml", "Cargo.toml", "package.json"] {
            let candidate = parent.join(name);
            if candidate.is_file()
                && let Some(table) = config_table(&candidate)?
            {
                return Ok((
                    Some(candidate.canonicalize().map_err(|e| e.to_string())?),
                    table,
                ));
            }
        }
    }
    Ok((None, Table::new()))
}

fn config_table(path: &Path) -> Result<Option<Table>, String> {
    let content = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let name = path.file_name().and_then(|name| name.to_str());
    let doc: Value = if name == Some("package.json") {
        let doc: serde_json::Value =
            serde_json::from_str(&content).map_err(|e| format!("{}: {e}", path.display()))?;
        let Some(config) = doc.get("lint-my-headers") else {
            return Ok(None);
        };
        Value::try_from(config).map_err(|e| format!("{}: {e}", path.display()))?
    } else {
        toml::from_str(&content).map_err(|e| format!("{}: {e}", path.display()))?
    };
    let (config, section) = match name {
        Some("pyproject.toml") => (
            doc.get("tool").and_then(|v| v.get("lint-my-headers")),
            SECTION,
        ),
        Some("Cargo.toml") => (
            doc.get("package")
                .and_then(|v| v.get("metadata"))
                .and_then(|v| v.get("lint-my-headers"))
                .or_else(|| {
                    doc.get("workspace")?
                        .get("metadata")?
                        .get("lint-my-headers")
                }),
            "metadata.lint-my-headers",
        ),
        Some("package.json") => (Some(&doc), "lint-my-headers"),
        Some(".lmh.toml") => (
            Some(
                doc.get("tool")
                    .and_then(|v| v.get("lint-my-headers"))
                    .unwrap_or(&doc),
            ),
            "lint-my-headers",
        ),
        // Explicit custom TOML paths retain the existing [tool.lint-my-headers] form.
        _ => (
            doc.get("tool").and_then(|v| v.get("lint-my-headers")),
            SECTION,
        ),
    };
    let Some(config) = config else {
        return Ok(None);
    };
    let config = config
        .as_table()
        .ok_or_else(|| format!("Invalid {section}: expected a table in {}", path.display()))?;
    for (key, value) in config {
        let reason = match key.as_str() {
            "owner" | "license" | "license-notice" => value
                .as_str()
                .is_none_or(str::is_empty)
                .then_some("expected a non-empty string"),
            "starting-year" | "creation-year" => value
                .as_integer()
                .is_none_or(|v| !(1000..=9999).contains(&v))
                .then_some("expected a four-digit integer"),
            "paths" | "languages" | "ignore-files" | "ignore-folders" => match value.as_array() {
                Some(items)
                    if items
                        .iter()
                        .all(|v| v.as_str().is_some_and(|s| !s.is_empty())) =>
                {
                    if matches!(key.as_str(), "paths" | "languages") && items.is_empty() {
                        Some("expected at least one value")
                    } else if key == "languages"
                        && items.iter().any(|v| {
                            Language::from_str(v.as_str().expect("validated string"), false)
                                .is_err()
                        })
                    {
                        Some(
                            "expected python, javascript, typescript, rust, go, swift, bash (alias shell), c, or cpp (alias c++)",
                        )
                    } else {
                        None
                    }
                }
                _ => Some("expected an array of non-empty strings"),
            },
            _ => Some("unknown key"),
        };
        if let Some(reason) = reason {
            return Err(format!(
                "Invalid {section}.{key}: {reason} in {}",
                path.display()
            ));
        }
    }
    Ok(Some(config.clone()))
}

fn strings(table: &Table, key: &str, default: &[&str]) -> Vec<String> {
    table
        .get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_else(|| default.iter().map(|s| (*s).into()).collect())
}

fn split_values(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_owned)
        .collect()
}

pub fn resolve(options: &Options) -> Result<Settings, String> {
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    let (config_path, table) = find_config(options.config.as_deref(), &cwd)?;
    let root = config_path
        .as_ref()
        .and_then(|p| p.parent())
        .unwrap_or(&cwd)
        .to_path_buf();
    let string = |key| table.get(key).and_then(Value::as_str).map(str::to_owned);
    let missing = |key| {
        format!("Missing lint-my-headers.{key}; set it in project configuration or pass --{key}")
    };
    let owner = options
        .owner
        .clone()
        .or_else(|| string("owner"))
        .filter(|s| !s.is_empty())
        .ok_or_else(|| missing("owner"))?;
    let year = options
        .starting_year
        .or_else(|| {
            table
                .get("starting-year")
                .and_then(Value::as_integer)
                .and_then(|i| i32::try_from(i).ok())
        })
        .ok_or_else(|| missing("starting-year"))?;
    let (license, notice) = if options.license.is_some() || options.license_notice.is_some() {
        (options.license.clone(), options.license_notice.clone())
    } else {
        (
            string("license"),
            string("license-notice").map(|s| root.join(s)),
        )
    };
    let license = license.filter(|s| !s.is_empty());
    let notice = notice.filter(|p| !p.as_os_str().is_empty());
    if license.is_some() == notice.is_some() {
        return Err("Configure exactly one of lint-my-headers.license or lint-my-headers.license-notice, or pass one matching CLI option".into());
    }
    if !options.paths.is_empty() && options.folders.is_some() {
        return Err("Pass explicit paths or --folders, not both".into());
    }
    let paths = if !options.paths.is_empty() {
        options.paths.clone()
    } else if let Some(folders) = &options.folders {
        split_values(folders).iter().map(PathBuf::from).collect()
    } else {
        strings(&table, "paths", &["."])
            .iter()
            .map(|s| root.join(s))
            .collect()
    };
    if paths.is_empty() {
        return Err("Invalid lint-my-headers.paths: expected at least one path".into());
    }
    let ignore_files = options
        .ignore_files
        .as_deref()
        .map(split_values)
        .unwrap_or_else(|| strings(&table, "ignore-files", &["__init__.py"]));
    let ignore_folders = options
        .ignore_folders
        .as_deref()
        .map(|s| split_values(s).iter().map(PathBuf::from).collect())
        .unwrap_or_else(|| {
            strings(&table, "ignore-folders", &[".github"])
                .iter()
                .map(|s| root.join(s))
                .collect()
        });
    Ok(Settings {
        owner,
        year,
        creation_year: options.creation_year.or_else(|| {
            table
                .get("creation-year")
                .and_then(Value::as_integer)
                .and_then(|year| i32::try_from(year).ok())
        }),
        license,
        license_notice: notice,
        license_path: root.join("LICENSE"),
        paths,
        languages: options.languages.clone().unwrap_or_else(|| {
            strings(&table, "languages", &["python"])
                .iter()
                .map(|name| Language::from_str(name, false).expect("validated language"))
                .collect()
        }),
        ignore_files,
        ignore_folders,
        project_root: root,
        config_path,
    })
}
