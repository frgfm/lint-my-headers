pub mod analysis;
pub mod config;
pub mod filesystem;
pub mod model;

use chrono::Datelike;
use clap::{CommandFactory, FromArgMatches};
use config::{Cli, OutputFormat};
use filesystem::RepairError;
use model::{CommandResult, Settings};
use std::collections::HashSet;
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

pub fn version() -> String {
    env!("CARGO_PKG_VERSION").replace("-rc.", "rc")
}

pub fn display_path(path: &Path, root: &Path) -> String {
    path_formatter(root)(path)
}

fn path_formatter(root: &Path) -> impl Fn(&Path) -> String {
    let cwd = std::env::current_dir().unwrap_or_default();
    let canonical_cwd = cwd.canonicalize().unwrap_or_else(|_| cwd.clone());
    fn absolute(path: &Path, cwd: &Path, canonical_cwd: &Path) -> PathBuf {
        let path = path.strip_prefix(cwd).unwrap_or(path);
        let path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            canonical_cwd.join(path)
        };
        let mut normal = PathBuf::new();
        for part in path.components() {
            match part {
                #[cfg(windows)]
                Component::Prefix(prefix) => match prefix.kind() {
                    std::path::Prefix::VerbatimDisk(drive) => {
                        normal.push(format!("{}:", char::from(drive)));
                    }
                    std::path::Prefix::VerbatimUNC(server, share) => {
                        normal.push(r"\\");
                        normal.push(server);
                        normal.push(share);
                    }
                    _ => normal.push(prefix.as_os_str()),
                },
                Component::CurDir => {}
                Component::ParentDir => {
                    normal.pop();
                }
                _ => normal.push(part.as_os_str()),
            }
        }
        normal
    }
    let root = absolute(root, &cwd, &canonical_cwd);
    move |path| {
        let path = absolute(path, &cwd, &canonical_cwd);
        if let Ok(relative) = path.strip_prefix(&root) {
            return relative.to_string_lossy().replace('\\', "/");
        }
        pathdiff::diff_paths(&path, &root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/")
    }
}

fn analyze_bytes(
    raw: &[u8],
    policy: &model::HeaderPolicy,
    shown: &str,
    known_clean: Option<&[u8]>,
) -> analysis::Inspection {
    // Reuse validation only for exactly the same bytes, policy and display path.
    if known_clean == Some(raw) {
        analysis::Inspection::default()
    } else {
        analysis::inspect(raw, policy, shown)
    }
}

fn map_files<T: Send, R: Send>(items: &mut [T], action: impl Fn(&mut T) -> R + Sync) -> Vec<R> {
    let count = items.len();
    let workers = if count < 256 {
        1
    } else {
        std::thread::available_parallelism().map_or(1, |n| n.get().min(4))
    };
    if workers == 1 {
        return items.iter_mut().map(action).collect();
    }
    let size = count.div_ceil(workers);
    std::thread::scope(|scope| {
        let handles = items
            .chunks_mut(size)
            .map(|chunk| scope.spawn(|| chunk.iter_mut().map(&action).collect::<Vec<_>>()))
            .collect::<Vec<_>>();
        let mut output = Vec::with_capacity(count);
        for handle in handles {
            output.extend(handle.join().expect("file worker panicked"));
        }
        output
    })
}

pub fn run(settings: &Settings, command: &str, current_year: i32) -> CommandResult {
    let mut result = CommandResult::new(command);
    let display = path_formatter(&settings.project_root);
    result.config_path = settings.config_path.as_ref().map(|p| display(p));
    let outcome: Result<(), (String, Option<String>)> = (|| {
        let policy = analysis::build_policy(settings, current_year).map_err(|e| (e, None))?;
        result.expected_header = Some(analysis::render_header(
            &policy.expected_header,
            settings.languages[0],
        ));
        let mut paths: Vec<_> = filesystem::discover_unsorted(settings)
            .map_err(|e| (e, None))?
            .into_iter()
            .map(|path| {
                let shown = display(&path);
                (path, shown)
            })
            .collect();
        paths.sort_by(|a, b| a.1.cmp(&b.1));
        let read = |path: &Path, shown: &str| {
            let snapshot = filesystem::read(path, &settings.project_root)
                .map_err(|e| (e.to_string(), Some(shown.to_owned())))?;
            let mut content = analysis::inspect(&snapshot.bytes, &policy, shown);
            if let Some(diagnostic) = &mut content.diagnostic {
                diagnostic.fixable &= snapshot.unsafe_reason.is_none();
            }
            Ok::<_, (String, Option<String>)>((snapshot, content))
        };
        let inspect =
            |(path, shown): &(PathBuf, String), repair: bool, known_clean: Option<&[u8]>| {
                if !repair {
                    let bytes =
                        std::fs::read(path).map_err(|e| (e.to_string(), Some(shown.clone())))?;
                    let mut content = analyze_bytes(&bytes, &policy, shown, known_clean);
                    // Only repairable findings need a guarded snapshot to report fixability.
                    if content.diagnostic.as_ref().is_some_and(|d| d.fixable) {
                        let snapshot = filesystem::read(path, &settings.project_root)
                            .map_err(|e| (e.to_string(), Some(shown.clone())))?;
                        if snapshot.bytes != bytes {
                            content = analysis::inspect(&snapshot.bytes, &policy, shown);
                            content
                                .diagnostic
                                .iter_mut()
                                .for_each(|d| d.fixable = false);
                        } else {
                            content.diagnostic.iter_mut().for_each(|d| {
                                d.fixable &= snapshot.unsafe_reason.is_none();
                            });
                        }
                    }
                    return Ok((None, content));
                }
                read(path, shown).map(|(snapshot, content)| (Some(snapshot), content))
            };
        let inspected = map_files(&mut paths, |path| inspect(path, command == "fix", None));
        let mut files: Vec<(_, (filesystem::Snapshot, analysis::Inspection))> =
            Vec::with_capacity(if command == "fix" { paths.len() } else { 0 });
        let mut targets = HashSet::new();
        for (path, file) in paths.iter().zip(inspected) {
            let (mut snapshot, mut content) = match file {
                Ok(file) => file,
                Err(error) => {
                    result.diagnostics.extend(
                        files
                            .iter()
                            .filter_map(|(_, file)| file.1.diagnostic.clone()),
                    );
                    return Err(error);
                }
            };
            if content.edit.is_some()
                && let Some(snapshot) = &mut snapshot
            {
                snapshot.claim_target(&mut targets);
                content
                    .diagnostic
                    .iter_mut()
                    .for_each(|d| d.fixable &= snapshot.unsafe_reason.is_none());
            }
            result.checked += 1;
            if let Some(snapshot) = snapshot {
                files.push((path, (snapshot, content)));
            } else {
                result.diagnostics.extend(content.diagnostic);
            }
        }
        if command != "fix" {
            return Ok(());
        }
        let cancelled = AtomicBool::new(false);
        let repaired = map_files(&mut files, |((path, _), (snapshot, content))| {
            if cancelled.load(Ordering::Relaxed) {
                return Ok(None);
            }
            let Some(diagnostic) = &mut content.diagnostic else {
                return Ok(None);
            };
            if !matches!(diagnostic.code.as_str(), "LMH001" | "LMH004") {
                return Ok(None);
            }
            if diagnostic.code == "LMH001" && content.edit.is_none() {
                return Ok(None);
            }
            let repair = match (&snapshot.unsafe_reason, &content.edit) {
                (Some(reason), _) => Err(RepairError::Unsafe(reason.clone())),
                (None, Some(edit)) => {
                    let bytes = edit.apply(&snapshot.bytes);
                    filesystem::replace(path, snapshot, &bytes, &settings.project_root)
                        .map(|()| snapshot.bytes = bytes)
                }
                (None, None) => Err(RepairError::Unsafe(
                    "copyright bytes cannot be repaired safely".into(),
                )),
            };
            match repair {
                Ok(()) => {
                    let changed = diagnostic.path.clone();
                    content.diagnostic = None;
                    return Ok(Some(changed));
                }
                Err(RepairError::Unsafe(reason)) => {
                    diagnostic.code = "LMH008".into();
                    diagnostic.message = reason;
                    diagnostic.fixable = false;
                }
                Err(RepairError::Io(error)) => {
                    cancelled.store(true, Ordering::Relaxed);
                    return Err((error.to_string(), Some(diagnostic.path.clone())));
                }
            }
            Ok(None)
        });
        let mut error = None;
        // Join all workers before reporting an error: in-flight writes may have completed.
        for outcome in repaired {
            match outcome {
                Ok(Some(path)) => result.changed.push(path),
                Err(e) => {
                    error.get_or_insert(e);
                }
                _ => {}
            }
        }
        result.diagnostics = files
            .iter()
            .filter_map(|(_, file)| file.1.diagnostic.clone())
            .collect();
        if let Some(error) = error {
            return Err(error);
        }
        // Keep unsafe findings; otherwise report the files as they stand after repair.
        result.diagnostics.clear();
        let diagnostics = map_files(&mut files, |(path, (snapshot, content))| {
            let known_clean = content
                .diagnostic
                .is_none()
                .then_some(snapshot.bytes.as_slice());
            match &content.diagnostic {
                Some(d) if d.code == "LMH008" => Ok(Some(d.clone())),
                _ => inspect(path, false, known_clean).map(|(_, content)| content.diagnostic),
            }
        });
        for diagnostic in diagnostics {
            result.diagnostics.extend(diagnostic?);
        }
        Ok(())
    })();
    if let Err((message, path)) = outcome {
        result.fail(message, path);
    }
    result
}

fn write_action_outputs(result: &CommandResult) -> io::Result<()> {
    let Some(path) = std::env::var_os("GITHUB_OUTPUT") else {
        return Ok(());
    };
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    let mut issues: Vec<&str> = if result.error.is_some() {
        Vec::new()
    } else {
        result.diagnostics.iter().map(|d| d.path.as_str()).collect()
    };
    issues.sort_unstable();
    issues.dedup();
    // Hook runners may launch several CLI processes sharing the same output file.
    file.write_all(
        format!(
            "issues={}\nchanged={}\n",
            serde_json::to_string(&issues)?,
            serde_json::to_string(&result.changed)?
        )
        .as_bytes(),
    )
}

fn render(result: &CommandResult, format: OutputFormat) -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    if format == OutputFormat::Json {
        serde_json::to_writer(&mut stdout, result)?;
        return writeln!(stdout);
    }
    if !result.changed.is_empty() {
        writeln!(stdout, "Updated headers:")?;
        for path in &result.changed {
            writeln!(stdout, "- {path}")?;
        }
    }
    let mut stderr = io::stderr().lock();
    if let Some(error) = &result.error {
        let path = error
            .path
            .as_ref()
            .map(|p| format!(" ({p})"))
            .unwrap_or_default();
        writeln!(stderr, "error: {}{path}", error.message)?;
    }
    for diagnostic in &result.diagnostics {
        let suffix = if diagnostic.fixable { " [fixable]" } else { "" };
        writeln!(
            stderr,
            "{}:{}:{}: {} {}{suffix}",
            diagnostic.path,
            diagnostic.line,
            diagnostic.column,
            diagnostic.code,
            diagnostic.message
        )?;
    }
    if !result.diagnostics.is_empty()
        && let Some(header) = &result.expected_header
    {
        writeln!(stderr, "\nExpected header:\n\n{header}")?;
    }
    Ok(())
}

pub fn main_entry() -> i32 {
    static VERSION: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    let command = Cli::command().version(VERSION.get_or_init(version).as_str());
    let args = Cli::from_arg_matches(&command.get_matches()).expect("clap-validated arguments");
    let options = args.command.options();
    let explicit_config = options
        .config
        .as_ref()
        .map(|p| display_path(p, &std::env::current_dir().unwrap_or_default()));
    let mut result = match config::resolve(options) {
        Ok(settings) => {
            let mut result = run(&settings, args.command.name(), chrono::Local::now().year());
            if result.expected_header.is_none() && result.error.is_some() {
                result.config_path = explicit_config.clone();
                if let Some(error) = &mut result.error
                    && error.message.starts_with("Unable to locate")
                {
                    error.path = explicit_config.clone();
                }
            }
            result
        }
        Err(error) => {
            let mut result = CommandResult::new(args.command.name());
            let path = if error.starts_with("Invalid configuration path:") {
                explicit_config.clone()
            } else {
                None
            };
            result.config_path = explicit_config;
            result.fail(error, path);
            result
        }
    };
    if let Err(error) = write_action_outputs(&result) {
        result.fail(
            format!("Unable to write GitHub Action outputs: {error}"),
            None,
        );
    }
    if let Err(error) = render(&result, options.output_format) {
        if error.kind() != io::ErrorKind::BrokenPipe {
            eprintln!("error: {error}");
        }
        return 2;
    }
    result.exit_code()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cached_paths_preserve_relative_absolute_and_parent_components() {
        let cwd = std::env::current_dir().unwrap();
        let root = cwd.join("src/nested/..");
        let display = path_formatter(&root);
        for path in [PathBuf::from("src/./file.rs"), cwd.join("src/file.rs")] {
            assert_eq!(display(&path), "file.rs");
        }
        assert_eq!(display(&cwd.join("other/../outside.rs")), "../outside.rs");
    }

    #[test]
    fn post_write_validation_rechecks_changed_bytes() {
        let policy = model::HeaderPolicy {
            owner: "Owner".into(),
            starting_year: 2024,
            current_year: 2030,
            creation_year: None,
            license_notices: vec!["Notice.\n".into()],
            expected_header: String::new(),
        };
        let clean = b"// Copyright (C) 2024-2030, Owner.\n\n// Notice.\n\nfn main() {}\n";
        for current in [
            clean.to_vec(),
            String::from_utf8_lossy(clean)
                .replace("2030", "2029")
                .into_bytes(),
            String::from_utf8_lossy(clean)
                .replace("Owner", "Other")
                .into_bytes(),
            [clean.as_slice(), b"// Copyright (C) 2024, Other.\n"].concat(),
            [clean.as_slice(), b"\xff"].concat(),
        ] {
            let cached = analyze_bytes(&current, &policy, "x.rs", Some(clean));
            let fresh = analysis::analyze(&current, &policy, "x.rs");
            assert_eq!(cached.diagnostic, fresh.diagnostic);
            assert_eq!(
                cached.edit.as_ref().map(|edit| edit.apply(&current)),
                fresh.replacement
            );
        }
    }
}
