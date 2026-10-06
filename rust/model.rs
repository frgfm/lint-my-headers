use serde::Serialize;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum Language {
    Python,
    Javascript,
    Typescript,
    Rust,
    Go,
    Swift,
    #[value(alias = "shell")]
    Bash,
    C,
    #[value(alias = "c++")]
    Cpp,
}

impl Language {
    pub fn from_path(path: &std::path::Path) -> Option<Self> {
        let name = path.file_name()?.to_string_lossy();
        match name.rsplit_once('.')?.1 {
            "py" => Some(Self::Python),
            "js" | "jsx" | "mjs" | "cjs" => Some(Self::Javascript),
            "ts" | "tsx" | "mts" | "cts" => Some(Self::Typescript),
            "rs" => Some(Self::Rust),
            "go" => Some(Self::Go),
            "swift" => Some(Self::Swift),
            "sh" | "bash" => Some(Self::Bash),
            "c" | "h" => Some(Self::C),
            "cc" | "cpp" | "cxx" | "c++" | "C" | "hh" | "hpp" | "hxx" | "h++" | "H" | "ipp"
            | "tpp" | "inl" => Some(Self::Cpp),
            _ => None,
        }
    }

    pub fn is_shared_header(path: &std::path::Path) -> bool {
        path.extension().is_some_and(|extension| extension == "h")
    }

    pub fn enabled_for_path(path: &std::path::Path, languages: &[Self]) -> bool {
        Self::from_path(path).is_some_and(|language| {
            languages.contains(&language)
                || (Self::is_shared_header(path) && languages.contains(&Self::Cpp))
        })
    }

    pub fn comment(self) -> &'static str {
        match self {
            Self::Python | Self::Bash => "#",
            Self::Javascript
            | Self::Typescript
            | Self::Rust
            | Self::Go
            | Self::Swift
            | Self::C
            | Self::Cpp => "//",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Settings {
    pub owner: String,
    pub year: i32,
    pub creation_year: Option<i32>,
    pub license: Option<String>,
    pub license_notice: Option<PathBuf>,
    pub license_path: PathBuf,
    pub paths: Vec<PathBuf>,
    pub languages: Vec<Language>,
    pub ignore_files: Vec<String>,
    pub ignore_folders: Vec<PathBuf>,
    pub project_root: PathBuf,
    pub config_path: Option<PathBuf>,
}

#[derive(Clone, Debug)]
pub struct HeaderPolicy {
    pub owner: String,
    pub starting_year: i32,
    pub current_year: i32,
    pub creation_year: Option<i32>,
    pub license_notices: Vec<String>,
    pub expected_header: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Diagnostic {
    pub path: String,
    pub line: usize,
    pub column: usize,
    pub code: String,
    pub message: String,
    pub fixable: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct CommandError {
    pub code: String,
    pub message: String,
    pub path: Option<String>,
}

#[derive(Debug, Default, Serialize)]
pub struct CommandResult {
    pub schema_version: u8,
    pub tool_version: String,
    pub command: String,
    pub config_path: Option<String>,
    pub checked: usize,
    pub changed: Vec<String>,
    pub diagnostics: Vec<Diagnostic>,
    pub expected_header: Option<String>,
    pub error: Option<CommandError>,
}

impl CommandResult {
    pub fn new(command: &str) -> Self {
        Self {
            schema_version: 1,
            tool_version: crate::version(),
            command: command.into(),
            ..Self::default()
        }
    }

    pub fn exit_code(&self) -> i32 {
        if self.error.is_some() {
            2
        } else if self.diagnostics.is_empty() {
            0
        } else {
            1
        }
    }

    pub fn fail(&mut self, message: impl ToString, path: Option<String>) {
        self.error = Some(CommandError {
            code: "LMH900".into(),
            message: message.to_string(),
            path,
        });
    }
}
