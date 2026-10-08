use chrono::{Datelike, Local};
use serde_json::{Value, json};
use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
};
use tempfile::{TempDir, tempdir};

const OWNER: &str = "Example Owner";
const NOTICE: &str = "# This program is licensed under the Apache License 2.0.\n# See LICENSE or go to <https://www.apache.org/licenses/LICENSE-2.0> for full license details.\n";
const CONFIG: &str = "[tool.lint-my-headers]\nowner = 'Example Owner'\nstarting-year = 2022\nlicense = 'Apache-2.0'\npaths = ['src']\nignore-files = []\nignore-folders = []\n";

fn workspace() -> TempDir {
    let dir = tempdir().unwrap();
    let root = dir.path();
    fs::write(root.join("LICENSE"), "Apache-2.0\n").unwrap();
    fs::write(root.join("pyproject.toml"), CONFIG).unwrap();
    fs::create_dir(root.join("src")).unwrap();
    dir
}

fn year() -> i32 {
    Local::now().year()
}

fn source(years: impl std::fmt::Display, owner: &str, notice: &str) -> String {
    format!("# Copyright (C) {years}, {owner}.\n\n{notice}\nvalue = 'café'\n")
}

fn header(years: impl std::fmt::Display) -> String {
    source(years, OWNER, NOTICE)
}

fn message(result: &Value) -> &str {
    result["error"]["message"].as_str().unwrap()
}

fn write(root: &Path, name: &str, contents: impl AsRef<[u8]>) {
    let path = root.join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

fn run(root: &Path, args: &[&str], exit: i32) -> std::process::Output {
    let output = Command::new(env!("CARGO_BIN_EXE_lmh"))
        .current_dir(root)
        .env("PATH", "")
        .env_remove("GITHUB_OUTPUT")
        .args(args)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(exit), "{args:?}: {output:?}");
    output
}

fn json_run(root: &Path, args: &[&str], exit: i32) -> Value {
    let mut args = args.to_vec();
    args.extend(["--output-format", "json"]);
    let output = run(root, &args, exit);
    assert!(output.stderr.is_empty(), "{output:?}");
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn missing_headers_use_explicit_policy_and_the_existing_guarded_writer() {
    let dir = workspace();
    let root = dir.path();
    write(
        root,
        "pyproject.toml",
        format!(
            "{CONFIG}languages = ['python', 'javascript', 'typescript', 'rust', 'go', 'swift', 'bash', 'c', 'cpp']\n"
        ),
    );
    let originals: Vec<_> = [
        (
            "py",
            "#!/usr/bin/python\r\n# coding: utf-8\r\nvalue = 'café'\r\n",
        ),
        (
            "js",
            "#!/usr/bin/env node\r\n'use strict';\r\nconst value = 1;\r\n",
        ),
        ("ts", "const value: number = 1;\r\n"),
        ("rs", "#![no_std]\r\nconst VALUE: i32 = 1;\r\n"),
        ("go", "//go:build linux\r\n\r\npackage example\r\n"),
        (
            "swift",
            "// swift-tools-version: 6.0\r\nimport PackageDescription\r\n",
        ),
        ("sh", "#!/bin/bash\r\necho café\r\n"),
        ("c", "#pragma once\r\nint value = 1;\r\n"),
        ("cpp", "#include <vector>\r\nint value = 1;\r\n"),
    ]
    .into_iter()
    .map(|(extension, body)| {
        let name = format!("src/missing.{extension}");
        let body = format!("\u{feff}{body}");
        write(root, &name, &body);
        (name, body)
    })
    .collect();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(
            root.join("src/missing.sh"),
            fs::Permissions::from_mode(0o755),
        )
        .unwrap();
    }
    let without_year = json_run(root, &["fix"], 1);
    assert_eq!(without_year["changed"], json!([]));
    assert!(
        without_year["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .all(|d| d["code"] == "LMH001" && d["fixable"] == false)
    );
    let declared = (year() - 2).to_string();
    let checked = json_run(root, &["check", "--creation-year", &declared], 1);
    assert_eq!(checked["schema_version"], 1);
    assert!(
        checked["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .all(|d| d["code"] == "LMH001" && d["fixable"] == true)
    );
    for (name, bytes) in &originals {
        assert_eq!(fs::read_to_string(root.join(name)).unwrap(), *bytes);
    }
    let fixed = json_run(root, &["fix", "--creation-year", &declared], 0);
    assert_eq!(fixed["changed"].as_array().unwrap().len(), originals.len());
    for (name, body) in originals {
        let bytes = fs::read(root.join(&name)).unwrap();
        assert!(bytes.starts_with(b"\xef\xbb\xbf"));
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains(&format!("Copyright (C) {declared}-{}, {OWNER}.", year())));
        assert!(text.contains("\r\n\r\n"));
        let code = body
            .rsplit_once("\r\n")
            .unwrap()
            .0
            .rsplit_once("\r\n")
            .map_or(
                body.trim_start_matches('\u{feff}').trim_end_matches("\r\n"),
                |(_, code)| code,
            );
        assert!(text.ends_with(&format!("{code}\r\n")), "{name}");
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(root.join("src/missing.sh"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o755
        );
    }
    assert_eq!(json_run(root, &["check"], 0)["diagnostics"], json!([]));
    assert_eq!(
        json_run(root, &["fix", "--creation-year", &declared], 0)["changed"],
        json!([])
    );
}

#[test]
fn creation_year_configuration_and_errors_are_resolved_before_writes() {
    let dir = workspace();
    let root = dir.path();
    let body = "value = 1\n";
    write(root, "src/missing.py", body);
    for bad in [
        "'2024'".to_string(),
        "999".to_string(),
        "2021".to_string(),
        (year() + 1).to_string(),
    ] {
        write(
            root,
            "pyproject.toml",
            format!("{CONFIG}creation-year = {bad}\n"),
        );
        let result = json_run(root, &["fix"], 2);
        assert_eq!(result["changed"], json!([]));
        assert_eq!(
            fs::read_to_string(root.join("src/missing.py")).unwrap(),
            body
        );
    }
    write(
        root,
        "pyproject.toml",
        format!("{CONFIG}creation-year = {}\n", year() - 2),
    );
    let current = year().to_string();
    json_run(root, &["fix", "--creation-year", &current], 0);
    let fixed = fs::read_to_string(root.join("src/missing.py")).unwrap();
    assert!(fixed.starts_with(&format!("# Copyright (C) {current}, {OWNER}.\n")));
    assert!(!fixed.contains(&format!("{}-{current}", year() - 2)));
}

#[cfg(unix)]
#[test]
fn missing_header_insertion_refuses_links_and_keeps_other_owners() {
    use std::os::unix::fs::symlink;
    let dir = workspace();
    let root = dir.path();
    write(
        root,
        "pyproject.toml",
        format!("{CONFIG}creation-year = {}\n", year()),
    );
    write(root, "src/linked.py", "value = 1\n");
    fs::hard_link(root.join("src/linked.py"), root.join("src/alias.py")).unwrap();
    symlink(root.join("src/linked.py"), root.join("src/symlink.py")).unwrap();
    let other = source(year(), "Other Owner", NOTICE);
    write(root, "src/other.py", &other);
    let before: Vec<_> = ["linked.py", "alias.py", "symlink.py", "other.py"]
        .into_iter()
        .map(|name| (name, fs::read(root.join("src").join(name)).unwrap()))
        .collect();
    let checked = json_run(root, &["check"], 1);
    assert!(
        checked["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .all(|d| d["fixable"] == false)
    );
    let fixed = json_run(root, &["fix"], 1);
    assert_eq!(fixed["changed"], json!([]));
    for (name, bytes) in before {
        assert_eq!(fs::read(root.join("src").join(name)).unwrap(), bytes);
    }
}

#[test]
fn missing_header_insertion_keeps_existing_legal_text_and_unsupported_docstrings_unchanged() {
    let dir = workspace();
    let root = dir.path();
    write(
        root,
        "pyproject.toml",
        format!("{CONFIG}creation-year = {}\n", year()),
    );
    let originals = [
        (
            "parenthesized.py",
            "(\"Copyright 2024 Other Owner\")\nvalue = 1\n",
        ),
        (
            "joined.py",
            "\"Copy\" \"right 2024 Other Owner\"\nvalue = 1\n",
        ),
        (
            "escaped.py",
            "\"Copy\\x72ight 2024 Other Owner\"\nvalue = 1\n",
        ),
        ("later.py", "value = 1\n# Copyright 2024 Other Owner\n"),
        ("symbol.py", "# © 2024 Other Owner\nvalue = 1\n"),
        ("license_variable.py", "license = 'example'\n"),
        ("attribution.py", "# (c) 2024 Other Owner\nvalue = 1\n"),
        ("rights.py", "# All rights reserved.\nvalue = 1\n"),
        (
            "bytes.py",
            "b\"Copy\\x72ight 2024 Other Owner\"\nvalue = 1\n",
        ),
        (
            "formatted.py",
            "f\"Copy\\x72ight 2024 Other Owner\"\nvalue = 1\n",
        ),
        (
            "joined_formatted.py",
            "\"Copy\" f\"\\x72ight 2024 Other Owner\"\nvalue = 1\n",
        ),
        (
            "joined_bytes.py",
            "b\"Copy\" b\"\\x72ight 2024 Other Owner\"\nvalue = 1\n",
        ),
        (
            "interpolated.py",
            "f\"{'Copy'}right 2024 Other Owner\"\nvalue = 1\n",
        ),
        (
            "attribute.rs",
            "#![no_std]\n// Copyright 2024 Other Owner\n",
        ),
        (
            "literal.py",
            "f\"\"\"Copyright 2024 Other Owner\"\"\"\nvalue = 1\n",
        ),
    ];
    for (name, body) in originals {
        write(root, &format!("src/{name}"), body);
    }
    let fixed = json_run(root, &["fix"], 1);
    assert_eq!(fixed["changed"], json!([]));
    assert!(
        fixed["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .all(|d| { d["code"] == "LMH001" && d["fixable"] == false })
    );
    assert!(fixed["diagnostics"].as_array().unwrap().iter().all(|d| {
        d["message"]
            .as_str()
            .unwrap()
            .contains("manual review required")
    }));
    for (name, body) in originals {
        assert_eq!(
            fs::read_to_string(root.join("src").join(name)).unwrap(),
            body
        );
    }
}

#[test]
fn common_header_layouts_use_the_same_read_only_checks_and_guarded_year_repairs() {
    let dir = workspace();
    let root = dir.path();
    write(
        root,
        "pyproject.toml",
        format!(
            "{CONFIG}languages = ['python', 'javascript', 'typescript', 'rust', 'go', 'swift', 'bash', 'c', 'cpp']\n"
        ),
    );
    let mut originals = Vec::new();
    for (extension, marker, body) in [
        ("py", "#", "value = 'café'\n"),
        ("js", "//", "const value = 'café';\n"),
        ("ts", "//", "const value: string = 'café';\n"),
        ("rs", "//", "const VALUE: &str = \"café\";\n"),
        ("go", "//", "package example\nconst Value = \"café\"\n"),
        ("swift", "//", "let value = \"café\"\n"),
        ("sh", "#", "value='café'\n"),
        ("c", "//", "const char *value = \"café\";\n"),
        ("cpp", "//", "const char *value = \"café\";\n"),
    ] {
        let text = format!(
            "SPDX-FileCopyrightText: {} {OWNER}\nSPDX-License-Identifier: Apache-2.0\n",
            year() - 2
        );
        let mut layouts = vec![
            text.lines()
                .map(|line| format!("{marker} {line}\n"))
                .collect::<String>(),
        ];
        if marker == "//" {
            layouts.push(format!(
                "/*\n{} */\n",
                text.lines()
                    .map(|line| format!(" * {line}\n"))
                    .collect::<String>()
            ));
        }
        for (index, layout) in layouts.into_iter().enumerate() {
            let name = format!("src/layout_{index}.{extension}");
            let contents = format!("\u{feff}{layout}\n{body}").replace('\n', "\r\n");
            write(root, &name, &contents);
            originals.push((name, contents));
        }
    }
    let prose = header(year() - 2)
        .replacen("Copyright (C)", "Copyright", 1)
        .replacen(&format!(", {OWNER}."), &format!(" {OWNER}"), 1);
    let block = format!(
        "/*\nCopyright {} {OWNER}\n\n{} */\nint value = 1;\n",
        year() - 2,
        NOTICE.replace("# ", "").trim_end_matches('\n')
    );
    for (name, contents) in [("src/prose.py", prose), ("src/prose_block.c", block)] {
        write(root, name, &contents);
        originals.push((name.to_string(), contents));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(
            root.join("src/layout_0.sh"),
            fs::Permissions::from_mode(0o755),
        )
        .unwrap();
    }
    let checked = json_run(root, &["check"], 1);
    assert_eq!(checked["schema_version"], 1);
    assert_eq!(checked["checked"], originals.len());
    assert!(
        checked["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .all(|d| d["code"] == "LMH004" && d["fixable"] == true)
    );
    for (name, contents) in &originals {
        assert_eq!(fs::read_to_string(root.join(name)).unwrap(), *contents);
    }
    let fixed = json_run(root, &["fix"], 0);
    assert_eq!(fixed["changed"].as_array().unwrap().len(), originals.len());
    for (name, contents) in originals {
        let repaired = contents.replacen(
            &(year() - 2).to_string(),
            &format!("{}-{}", year() - 2, year()),
            1,
        );
        assert_eq!(fs::read_to_string(root.join(name)).unwrap(), repaired);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(root.join("src/layout_0.sh"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o755
        );
    }
    assert_eq!(json_run(root, &["check"], 0)["diagnostics"], json!([]));
    assert_eq!(json_run(root, &["fix"], 0)["changed"], json!([]));

    for years in [year() - 2, year()] {
        let matching = format!(
            "{}# SPDX-License-Identifier: Apache-2.0\n\nvalue = 1\n",
            header(years).split("value =").next().unwrap()
        );
        write(root, "src/prose_with_id.py", &matching);
        let checked = json_run(
            root,
            &["check", "src/prose_with_id.py"],
            i32::from(years != year()),
        );
        assert_eq!(
            checked["diagnostics"].as_array().unwrap().len(),
            usize::from(years != year())
        );
        let fixed = json_run(root, &["fix", "src/prose_with_id.py"], 0);
        assert_eq!(
            fixed["changed"].as_array().unwrap().len(),
            usize::from(years != year())
        );
        assert_eq!(
            fs::read_to_string(root.join("src/prose_with_id.py")).unwrap(),
            if years == year() {
                matching.clone()
            } else {
                matching.replacen(&years.to_string(), &format!("{years}-{}", year()), 1)
            }
        );
        for (name, contents) in [
            (
                "src/mixed.py",
                format!(
                    "# SPDX-FileCopyrightText: {years} {OWNER}\n# SPDX-License-Identifier: Apache-2.0\n\n# This program is licensed under the MIT License.\nvalue = 1\n"
                ),
            ),
            (
                "src/mixed_file_notice.py",
                format!(
                    "# SPDX-FileCopyrightText: {years} {OWNER}\n# SPDX-License-Identifier: Apache-2.0\n\n# This file is licensed under the MIT License.\nvalue = 1\n"
                ),
            ),
            (
                "src/mixed_short_notice.c",
                format!(
                    "/*\nSPDX-FileCopyrightText: {years} {OWNER}\nSPDX-License-Identifier: Apache-2.0\n*/\n\n// License: MIT\nint value = 1;\n"
                ),
            ),
            (
                "src/mixed.c",
                format!(
                    "/*\n{}*/\n// SPDX-License-Identifier: MIT\nint value = 1;\n",
                    header(years)
                        .split("value =")
                        .next()
                        .unwrap()
                        .replace("# ", "")
                ),
            ),
            (
                "src/prose_conflict.c",
                format!(
                    "/*\n{}*/ /* Licensed under the MIT License. */\nint value = 1;\n",
                    header(years)
                        .split("value =")
                        .next()
                        .unwrap()
                        .replace("# ", "")
                ),
            ),
        ] {
            write(root, name, &contents);
            let checked = json_run(root, &["check", name], 1);
            assert_eq!(checked["diagnostics"][0]["code"], "LMH006");
            let fixed = json_run(root, &["fix", name], 1);
            assert_eq!(fixed["changed"], json!([]));
            assert_eq!(fixed["diagnostics"][0]["code"], "LMH006");
            assert_eq!(fs::read_to_string(root.join(name)).unwrap(), contents);
        }
    }

    let wrong_id = header(year() - 2)
        .split("value =")
        .next()
        .unwrap()
        .to_string()
        + "# SPDX-License-Identifier: MIT\n";
    write(root, "src/wrong_id.py", &wrong_id);
    let fixed = json_run(root, &["fix", "src/wrong_id.py"], 1);
    assert_eq!(fixed["diagnostics"][0]["code"], "LMH005");
    assert_eq!(fixed["changed"], json!([]));
    assert_eq!(
        fs::read_to_string(root.join("src/wrong_id.py")).unwrap(),
        wrong_id
    );

    let linked = format!(
        "# SPDX-FileCopyrightText: {} {OWNER}\n# SPDX-License-Identifier: Apache-2.0\n",
        year() - 2
    );
    write(root, "src/linked.py", &linked);
    fs::hard_link(root.join("src/linked.py"), root.join("src/alias.py")).unwrap();
    let checked = json_run(root, &["check", "src/linked.py"], 1);
    assert_eq!(checked["diagnostics"][0]["fixable"], false);
    let fixed = json_run(root, &["fix", "src/linked.py"], 1);
    assert_eq!(fixed["changed"], json!([]));
    assert_eq!(fixed["diagnostics"][0]["code"], "LMH008");
    assert_eq!(
        fs::read_to_string(root.join("src/linked.py")).unwrap(),
        linked
    );
}

#[test]
fn shell_selection_aliases_and_year_only_repairs_work_without_a_shell_runtime() {
    let dir = workspace();
    let root = dir.path();
    let original = header(year() - 2).replace("value = 'café'", "value='café'");
    let script = format!("\u{feff}#!/usr/bin/env bash\n\n{original}").replace('\n', "\r\n");
    for name in [
        "example.sh",
        "example.bash",
        "unsupported.zsh",
        "extensionless",
    ] {
        write(root, &format!("src/{name}"), &script);
    }
    write(root, "src/example.py", header(year()));
    assert_eq!(
        json_run(root, &["check", "src/example.sh"], 0)["checked"],
        0
    );
    for language in ["bash", "shell"] {
        write(
            root,
            "pyproject.toml",
            format!("{CONFIG}languages = ['{language}']\n"),
        );
        let checked = json_run(root, &["check"], 1);
        assert_eq!(checked["checked"], 2);
        assert_eq!(checked["diagnostics"][0]["code"], "LMH004");
        assert!(
            checked["expected_header"]
                .as_str()
                .unwrap()
                .starts_with("# Copyright")
        );
        assert_eq!(
            fs::read_to_string(root.join("src/example.sh")).unwrap(),
            script
        );
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(
            root.join("src/example.sh"),
            fs::Permissions::from_mode(0o755),
        )
        .unwrap();
    }
    let fixed = json_run(root, &["fix", "--languages", "bash"], 0);
    assert_eq!(
        fixed["changed"],
        json!(["src/example.bash", "src/example.sh"])
    );
    let repaired = script.replacen(
        &(year() - 2).to_string(),
        &format!("{}-{}", year() - 2, year()),
        1,
    );
    for name in ["example.sh", "example.bash"] {
        assert_eq!(
            fs::read_to_string(root.join(format!("src/{name}"))).unwrap(),
            repaired
        );
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(root.join("src/example.sh"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o755
        );
    }
    assert_eq!(
        json_run(root, &["fix", "--languages", "shell"], 0)["changed"],
        json!([])
    );
    assert_eq!(
        json_run(
            root,
            &["check", "--languages", "python", "src/example.sh"],
            0
        )["checked"],
        0
    );
    assert_eq!(
        json_run(
            root,
            &[
                "check",
                "--languages",
                "shell",
                "--ignore-files",
                "example.sh",
                "src/example.sh"
            ],
            0
        )["checked"],
        0
    );
    assert_eq!(
        fs::read_to_string(root.join("src/extensionless")).unwrap(),
        script
    );
}

#[test]
fn c_and_cpp_selection_includes_shared_headers_and_all_supported_extensions() {
    let dir = workspace();
    let root = dir.path();
    fs::remove_file(root.join("pyproject.toml")).unwrap();
    let config = CONFIG.strip_prefix("[tool.lint-my-headers]\n").unwrap();
    write(
        root,
        ".lmh.toml",
        format!("{config}languages = ['c', 'cpp']\n"),
    );
    let original = header(year() - 2)
        .replace("# ", "// ")
        .replace("value = 'café'", "const char *value = \"café\";");
    let cpp = original.replace(
        "const char *value = \"café\";",
        "template <typename T> T value(T x) { return x; }",
    );
    let cpp_extensions = [
        "cc", "cpp", "cxx", "c++", "C", "hh", "hpp", "hxx", "h++", "H", "ipp", "tpp", "inl",
    ];
    for extension in ["c", "h"] {
        write(root, &format!("src/example.{extension}"), &original);
    }
    write(root, "src/cpp.h", &cpp);
    // Use a distinct basename so .C/.H never overwrite the C fixtures on case-insensitive filesystems.
    for extension in cpp_extensions {
        write(root, &format!("src/cpp_example.{extension}"), &cpp);
    }
    write(root, "src/example.py", header(year()));
    write(root, "src/unsupported.m", &original);
    let checked = json_run(root, &["check"], 1);
    assert_eq!(checked["config_path"], ".lmh.toml");
    assert_eq!(checked["checked"], cpp_extensions.len() + 3);
    assert!(
        checked["expected_header"]
            .as_str()
            .unwrap()
            .starts_with("// Copyright")
    );
    assert!(
        checked["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .all(|d| d["code"] == "LMH004" && d["fixable"] == true)
    );
    assert_eq!(
        fs::read_to_string(root.join("src/example.c")).unwrap(),
        original
    );
    assert_eq!(
        json_run(root, &["check", "--languages", "c"], 1)["checked"],
        3
    );
    for language in ["cpp", "c++"] {
        write(
            root,
            ".lmh.toml",
            format!("{config}languages = ['{language}']\n"),
        );
        assert_eq!(
            json_run(root, &["check"], 1)["checked"],
            cpp_extensions.len() + 2
        );
        assert_eq!(
            json_run(
                root,
                &[
                    "check",
                    "--languages",
                    language,
                    "src/example.h",
                    "src/cpp.h"
                ],
                1
            )["checked"],
            2
        );
    }
    let fixed = json_run(root, &["fix", "--languages", "c,cpp"], 0);
    assert_eq!(
        fixed["changed"].as_array().unwrap().len(),
        cpp_extensions.len() + 3
    );
    for extension in ["c", "h"] {
        assert_eq!(
            fs::read_to_string(root.join(format!("src/example.{extension}"))).unwrap(),
            original.replacen(
                &(year() - 2).to_string(),
                &format!("{}-{}", year() - 2, year()),
                1,
            )
        );
    }
    for extension in cpp_extensions {
        assert_eq!(
            fs::read_to_string(root.join(format!("src/cpp_example.{extension}"))).unwrap(),
            cpp.replacen(
                &(year() - 2).to_string(),
                &format!("{}-{}", year() - 2, year()),
                1,
            )
        );
    }
    assert_eq!(
        json_run(root, &["fix", "--languages", "c,c++"], 0)["changed"],
        json!([])
    );
    assert_eq!(
        json_run(
            root,
            &[
                "check",
                "--languages",
                "python",
                "src/example.c",
                "src/cpp_example.cpp",
                "src/example.h"
            ],
            0
        )["checked"],
        0
    );
    assert_eq!(
        json_run(
            root,
            &["check", "--ignore-files", "example.h", "src/example.h"],
            0
        )["checked"],
        0
    );
}

#[test]
fn swift_sources_tests_and_package_manifest_use_standalone_policy() {
    let dir = workspace();
    let root = dir.path();
    fs::remove_file(root.join("pyproject.toml")).unwrap();
    let config = CONFIG
        .strip_prefix("[tool.lint-my-headers]\n")
        .unwrap()
        .replace("paths = ['src']", "paths = ['.']");
    write(
        root,
        ".lmh.toml",
        format!("{config}languages = ['swift']\n"),
    );
    let original = header(year() - 2)
        .replace("# ", "// ")
        .replace("value = 'café'", "let value = \"café\"");
    write(root, "Sources/Example/Example.swift", &original);
    write(root, "Tests/ExampleTests/ExampleTests.swift", &original);
    let manifest = format!(
        "// swift-tools-version: 5.9\n\n{}",
        original.replace(
            "let value = \"café\"",
            "import PackageDescription\nlet package = Package(name: \"Example\")"
        )
    );
    write(root, "Package.swift", &manifest);
    let checked = json_run(root, &["check"], 1);
    assert_eq!(checked["config_path"], ".lmh.toml");
    assert_eq!(checked["checked"], 3);
    assert_eq!(
        fs::read_to_string(root.join("Package.swift")).unwrap(),
        manifest
    );
    assert_eq!(
        json_run(root, &["fix"], 0)["changed"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert_eq!(
        fs::read_to_string(root.join("Package.swift")).unwrap(),
        manifest.replacen(
            &(year() - 2).to_string(),
            &format!("{}-{}", year() - 2, year()),
            1
        )
    );
    assert_eq!(json_run(root, &["fix"], 0)["changed"], json!([]));
    assert_eq!(
        json_run(
            root,
            &["check", "--languages", "python", "Package.swift"],
            0
        )["checked"],
        0
    );
}

#[test]
fn go_files_use_standalone_policy_and_include_tests_and_platform_variants() {
    let dir = workspace();
    let root = dir.path();
    fs::remove_file(root.join("pyproject.toml")).unwrap();
    write(root, "go.mod", "module example.com/project\n\ngo 1.22\n");
    let config = CONFIG.strip_prefix("[tool.lint-my-headers]\n").unwrap();
    write(root, ".lmh.toml", format!("{config}languages = ['go']\n"));
    let original = header(year() - 2)
        .replace("# ", "// ")
        .replace("value = 'café'", "package example\nconst value = \"café\"");
    for name in ["main.go", "main_test.go", "main_windows.go"] {
        write(root, &format!("src/{name}"), &original);
    }
    write(root, "src/example.py", header(year()));
    let checked = json_run(root, &["check"], 1);
    assert_eq!(checked["config_path"], ".lmh.toml");
    assert_eq!(checked["checked"], 3);
    assert_eq!(
        fs::read_to_string(root.join("src/main.go")).unwrap(),
        original
    );
    let fixed = json_run(root, &["fix"], 0);
    assert_eq!(fixed["changed"].as_array().unwrap().len(), 3);
    assert_eq!(
        fs::read_to_string(root.join("src/main.go")).unwrap(),
        original.replacen(
            &(year() - 2).to_string(),
            &format!("{}-{}", year() - 2, year()),
            1
        )
    );
    assert_eq!(json_run(root, &["fix"], 0)["changed"], json!([]));
    assert_eq!(
        json_run(root, &["check", "--languages", "python", "src/main.go"], 0)["checked"],
        0
    );
    assert_eq!(
        json_run(
            root,
            &["check", "--ignore-files", "main.go", "src/main.go"],
            0
        )["checked"],
        0
    );
}

#[test]
fn rust_check_and_fix_use_cargo_package_or_workspace_policy() {
    let dir = workspace();
    let root = dir.path();
    fs::remove_file(root.join("pyproject.toml")).unwrap();
    let original = header(year() - 2)
        .replace("# ", "// ")
        .replace("value = 'café'", "const VALUE: &str = \"café\";");
    write(root, "src/example.py", header(year()));
    let table = CONFIG.strip_prefix("[tool.lint-my-headers]\n").unwrap();
    for section in ["package.metadata", "workspace.metadata"] {
        let manifest = if section == "package.metadata" {
            "[package]\nname = 'example'\nversion = '0.1.0'\nedition = '2024'\n"
        } else {
            "[workspace]\nmembers = []\n"
        };
        write(
            root,
            "Cargo.toml",
            format!("{manifest}[{section}.lint-my-headers]\n{table}languages = ['rust']\n"),
        );
        write(root, "src/lib.rs", &original);
        let checked = json_run(root, &["check"], 1);
        assert_eq!(checked["config_path"], "Cargo.toml");
        assert_eq!(checked["checked"], 1);
        assert_eq!(checked["diagnostics"][0]["code"], "LMH004");
        assert!(
            checked["expected_header"]
                .as_str()
                .unwrap()
                .starts_with("// Copyright")
        );
        assert_eq!(
            fs::read_to_string(root.join("src/lib.rs")).unwrap(),
            original
        );
        assert_eq!(
            json_run(root, &["fix"], 0)["changed"],
            json!(["src/lib.rs"])
        );
        assert_eq!(
            fs::read_to_string(root.join("src/lib.rs")).unwrap(),
            original.replacen(
                &(year() - 2).to_string(),
                &format!("{}-{}", year() - 2, year()),
                1
            )
        );
        assert_eq!(json_run(root, &["fix"], 0)["changed"], json!([]));
        assert_eq!(
            json_run(root, &["check", "--languages", "python", "src/lib.rs"], 0)["checked"],
            0
        );
    }
    write(
        root,
        ".lmh.toml",
        format!("{table}languages = ['python']\n"),
    );
    assert_eq!(json_run(root, &["check"], 0)["config_path"], ".lmh.toml");
    assert_eq!(
        json_run(root, &["check", "--config", "Cargo.toml"], 0)["checked"],
        1
    );
}

#[test]
fn language_selection_and_shared_notices_work_for_every_extension() {
    let dir = workspace();
    let root = dir.path();
    write(root, "notice.txt", "Proprietary.\n\nAll rights reserved.\n");
    let extensions = [
        "js", "jsx", "mjs", "cjs", "ts", "tsx", "mts", "cts", "d.ts", "d.mts", "d.cts",
    ];
    let original = source(
        year() - 2,
        OWNER,
        "# Proprietary.\n\n# All rights reserved.\n",
    );
    write(root, "src/example.py", &original);
    for extension in extensions {
        write(
            root,
            &format!("src/example.{extension}"),
            original.replace("# ", "// "),
        );
    }
    write(root, "src/unsupported.rs", "unrecognized source\n");
    assert_eq!(
        json_run(root, &["check", "--license-notice", "notice.txt"], 1)["checked"],
        1
    );
    let args = [
        "--languages",
        "python,javascript,typescript",
        "--license-notice",
        "notice.txt",
    ];
    let checked = json_run(root, &[&["check"], &args[..]].concat(), 1);
    assert_eq!(checked["checked"], extensions.len() + 1);
    assert_eq!(
        fs::read_to_string(root.join("src/example.ts")).unwrap(),
        original.replace("# ", "// ")
    );
    let fixed = json_run(root, &[&["fix"], &args[..]].concat(), 0);
    assert_eq!(
        fixed["changed"].as_array().unwrap().len(),
        extensions.len() + 1
    );
    for extension in extensions {
        assert_eq!(
            fs::read_to_string(root.join(format!("src/example.{extension}"))).unwrap(),
            original.replace("# ", "// ").replacen(
                &(year() - 2).to_string(),
                &format!("{}-{}", year() - 2, year()),
                1
            )
        );
    }
    assert_eq!(
        json_run(root, &[&["fix"], &args[..]].concat(), 0)["changed"],
        json!([])
    );
    assert_eq!(
        json_run(root, &["check", "src/example.ts", "src/unsupported.rs"], 0)["checked"],
        0
    );
    let selected = json_run(
        root,
        &[
            "check",
            "--languages",
            "typescript",
            "--license-notice",
            "notice.txt",
            "src/example.ts",
        ],
        0,
    );
    assert!(
        selected["expected_header"]
            .as_str()
            .unwrap()
            .starts_with("// Copyright")
    );
    assert_eq!(
        json_run(
            root,
            &[
                "check",
                "--languages",
                "typescript",
                "--license-notice",
                "notice.txt",
                "--ignore-files",
                "example.ts",
                "src/example.ts"
            ],
            0
        )["checked"],
        0
    );
}

#[test]
fn project_configuration_fallbacks_precedence_and_explicit_selection() {
    let dir = workspace();
    let root = dir.path();
    write(root, "src/example.ts", header(year()).replace("# ", "// "));
    let table = CONFIG.strip_prefix("[tool.lint-my-headers]\n").unwrap();
    let configs = [
        (".lmh.toml", format!("{table}languages = ['typescript']\n")),
        (
            "pyproject.toml",
            format!("{CONFIG}languages = ['typescript']\n"),
        ),
        (
            "Cargo.toml",
            format!("[workspace.metadata.lint-my-headers]\n{table}languages = ['typescript']\n"),
        ),
        (
            "package.json",
            json!({"name": "example", "lint-my-headers": {
                "owner": OWNER, "starting-year": 2022, "license": "Apache-2.0",
                "paths": ["src"], "languages": ["typescript"]
            }})
            .to_string(),
        ),
    ];
    for (name, contents) in &configs {
        write(root, name, contents);
    }
    fs::create_dir(root.join("nested")).unwrap();
    assert_eq!(
        json_run(root, &["check", "--config", "package.json"], 0)["config_path"],
        "package.json"
    );
    for (name, _) in &configs {
        let result = json_run(&root.join("nested"), &["check"], 0);
        assert_eq!(result["config_path"], *name);
        assert_eq!(result["checked"], 1);
        fs::remove_file(root.join(name)).unwrap();
    }
    write(
        root,
        "Cargo.toml",
        configs[2]
            .1
            .replace("workspace.metadata", "package.metadata"),
    );
    write(root, "pyproject.toml", "[project]\nname = 'unrelated'\n");
    assert_eq!(json_run(root, &["check"], 0)["config_path"], "Cargo.toml");
    write(root, ".lmh.toml", &configs[0].1);
    write(root, "nested/LICENSE", "Apache-2.0\n");
    write(
        root,
        "nested/package.json",
        configs[3].1.replace("\"src\"", "\"../src\""),
    );
    assert_eq!(
        json_run(&root.join("nested"), &["check"], 0)["config_path"],
        "package.json"
    );
    write(root, "nested/.lmh.toml", "languages = []\n");
    assert_eq!(
        json_run(&root.join("nested"), &["fix"], 2)["changed"],
        json!([])
    );
    for invalid in [
        "lint-my-headers = null",
        "{\"lint-my-headers\": []}",
        "{\"lint-my-headers\": {\"languages\": [\"typo\"]}}",
    ] {
        write(root, "package.json", invalid);
        assert_eq!(
            json_run(root, &["fix", "--config", "package.json"], 2)["changed"],
            json!([])
        );
    }
    write(root, "custom.toml", "[tool.ruff]\nline-length = 100\n");
    let result = json_run(
        root,
        &[
            "check",
            "--config",
            "custom.toml",
            "--owner",
            OWNER,
            "--starting-year",
            "2022",
            "--license",
            "Apache-2.0",
            "--languages",
            "typescript",
            "src/example.ts",
        ],
        0,
    );
    assert_eq!(result["checked"], 1);
    write(root, "custom.toml", &configs[1].1);
    assert_eq!(
        json_run(root, &["check", "--config", "custom.toml"], 0)["checked"],
        1
    );
}

#[test]
fn native_commands_help_version_and_legacy_names() {
    let dir = workspace();
    let root = dir.path();
    let version = run(root, &["--version"], 0);
    assert!(
        String::from_utf8(version.stdout)
            .unwrap()
            .ends_with(&format!(
                " {}\n",
                env!("CARGO_PKG_VERSION").replace("-rc.", "rc")
            ))
    );
    for binary in [
        env!("CARGO_BIN_EXE_lmh"),
        env!("CARGO_BIN_EXE_lint-my-headers"),
    ] {
        let output = Command::new(binary)
            .env("PATH", "")
            .arg("--help")
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        let help = String::from_utf8(output.stdout).unwrap();
        assert!(help.contains("check") && help.contains("fix"));
    }
    for removed in ["vph", "validate-python-headers", "validate_headers"] {
        run(root, &[removed], 2);
    }
    write(
        root,
        "pyproject.toml",
        CONFIG.replace("lint-my-headers", "validate-python-headers"),
    );
    let result = json_run(root, &["check"], 2);
    assert!(message(&result).contains("owner"));
}

#[test]
fn check_json_is_ordered_complete_and_read_only() {
    let dir = workspace();
    let root = dir.path();
    write(root, "src/b.py", "value = 1\n");
    let stale = header(year() - 2);
    write(root, "src/a.py", &stale);
    let before = fs::metadata(root.join("src/a.py"))
        .unwrap()
        .modified()
        .unwrap();
    let output = run(root, &["check", "--output-format", "json"], 1);
    assert!(output.stderr.is_empty());
    let text = String::from_utf8(output.stdout).unwrap();
    let keys = [
        "schema_version",
        "tool_version",
        "command",
        "config_path",
        "checked",
        "changed",
        "diagnostics",
        "expected_header",
        "error",
    ];
    let positions: Vec<_> = keys
        .iter()
        .map(|key| text.find(&format!("\"{key}\":")).unwrap())
        .collect();
    assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
    let result: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(result["schema_version"], 1);
    assert_eq!(result["command"], "check");
    assert_eq!(result["config_path"], "pyproject.toml");
    assert_eq!(result["checked"], 2);
    assert_eq!(result["changed"], json!([]));
    assert_eq!(result["diagnostics"][0]["path"], "src/a.py");
    assert_eq!(result["diagnostics"][0]["code"], "LMH004");
    assert_eq!(result["diagnostics"][0]["fixable"], true);
    assert_eq!(result["diagnostics"][1]["path"], "src/b.py");
    assert_eq!(result["diagnostics"][1]["code"], "LMH001");
    assert!(result["error"].is_null());
    assert!(
        result["expected_header"]
            .as_str()
            .unwrap()
            .contains("<FILE_CREATION_YEAR>")
    );
    assert_eq!(fs::read(root.join("src/a.py")).unwrap(), stale.as_bytes());
    assert_eq!(
        fs::metadata(root.join("src/a.py"))
            .unwrap()
            .modified()
            .unwrap(),
        before
    );
}

#[test]
fn concurrent_hook_processes_append_complete_action_output_records() {
    let dir = workspace();
    let root = dir.path();
    write(root, "src/clean.py", header(year()));
    let output_path = root.join("github-output.txt");
    let children: Vec<_> = (0..32)
        .map(|_| {
            Command::new(env!("CARGO_BIN_EXE_lmh"))
                .current_dir(root)
                .env("PATH", "")
                .env("GITHUB_OUTPUT", &output_path)
                .args(["check", "--output-format", "json"])
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap()
        })
        .collect();
    for child in children {
        let result = child.wait_with_output().unwrap();
        assert!(result.status.success(), "{result:?}");
    }
    assert_eq!(
        fs::read_to_string(output_path).unwrap(),
        "issues=[]\nchanged=[]\n".repeat(32)
    );
}

#[test]
fn text_diagnostics_and_action_outputs_remain_compatible() {
    let dir = workspace();
    let root = dir.path();
    write(root, "src/stale.py", header(year() - 2));
    write(root, "src/missing.py", "value = 1\n");
    let output = run(root, &["check"], 1);
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("src/stale.py:1:1: LMH004"));
    assert!(stderr.contains("[fixable]") && stderr.contains("Expected header:"));
    let action_output = root.join("github-output.txt");
    let output = Command::new(env!("CARGO_BIN_EXE_lmh"))
        .current_dir(root)
        .env("PATH", "")
        .env("GITHUB_OUTPUT", &action_output)
        .arg("fix")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert_eq!(
        fs::read_to_string(&action_output).unwrap(),
        "issues=[\"src/missing.py\"]\nchanged=[\"src/stale.py\"]\n"
    );
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("Updated headers:\n- src/stale.py\n")
    );
    let output = Command::new(env!("CARGO_BIN_EXE_lmh"))
        .current_dir(root)
        .env("GITHUB_OUTPUT", &action_output)
        .args(["check", "--output-format", "json", "missing.py"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stderr.is_empty());
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["checked"], 0);
    assert_eq!(result["diagnostics"], json!([]));
    assert_eq!(result["error"]["code"], "LMH900");
    assert!(!result["expected_header"].is_null());
    assert!(
        fs::read_to_string(action_output)
            .unwrap()
            .ends_with("issues=[]\nchanged=[]\n")
    );
}

#[test]
fn diagnostic_precedence_and_unfixable_sources() {
    let dir = workspace();
    let root = dir.path();
    let invalid = [
        ("missing", "value = 1\n".to_owned(), "LMH001"),
        (
            "owner",
            source(year() + 1, "Another Owner", "# Wrong.\n"),
            "LMH002",
        ),
        ("future", source(year() + 1, OWNER, "# Wrong.\n"), "LMH003"),
        (
            "reversed",
            source(format!("{}-{}", year() - 1, year() - 2), OWNER, NOTICE),
            "LMH003",
        ),
        ("old", source("2021", OWNER, NOTICE), "LMH003"),
        ("license", source(year() - 2, OWNER, "# Wrong.\n"), "LMH005"),
        ("layout", source("20x4", "Another Owner", NOTICE), "LMH006"),
        (
            "duplicate",
            format!(
                "{}# Copyright (C) {}, {OWNER}.\n",
                header(year() - 2).split("value =").next().unwrap(),
                year()
            ),
            "LMH006",
        ),
        (
            "decode",
            "# coding: made-up-codec\nvalue = 1\n".to_owned(),
            "LMH007",
        ),
        (
            "late_cookie",
            format!("value = 0\n# coding: utf-8\n\n{}", header(year())),
            "LMH001",
        ),
    ];
    for (name, contents, _) in &invalid {
        write(root, &format!("src/{name}.py"), contents);
    }
    write(root, "src/stale.py", header(year() - 2));
    let result = json_run(root, &["fix"], 1);
    assert_eq!(result["changed"], json!(["src/stale.py"]));
    let diagnostics = result["diagnostics"].as_array().unwrap();
    assert_eq!(diagnostics.len(), invalid.len());
    for (name, contents, code) in invalid {
        let path = format!("src/{name}.py");
        let diagnostic = diagnostics.iter().find(|d| d["path"] == path).unwrap();
        assert_eq!(diagnostic["code"], code, "{name}");
        assert_eq!(diagnostic["fixable"], false, "{name}");
        assert_eq!(fs::read_to_string(root.join(path)).unwrap(), contents);
    }
}

#[test]
fn python_preambles_and_string_examples_are_accepted() {
    let dir = workspace();
    let root = dir.path();
    for (name, preamble) in [
        ("single", ""),
        ("shebang", "#!/usr/bin/env python3\n\n"),
        ("cookie", "# coding: utf-8\n\n"),
        (
            "both",
            "#!/usr/bin/env python3\n# -*- coding: utf-8 -*-\n\n",
        ),
        ("second_cookie", "# A comment\n# coding=utf-8\n\n"),
    ] {
        write(
            root,
            &format!("src/{name}.py"),
            format!("{preamble}{}", header(year())),
        );
    }
    write(root, "src/range.py", header(format!("2024-{}", year())));
    write(
        root,
        "src/example.py",
        format!(
            "{}EXAMPLE = '''\n# Copyright (C) 2024, Someone Else.\n'''\n",
            header(year())
        ),
    );
    let result = json_run(root, &["check"], 0);
    assert_eq!(result["checked"], 7);
    assert_eq!(result["diagnostics"], json!([]));
}

#[test]
fn fix_preserves_bytes_mode_and_is_idempotent() {
    let dir = workspace();
    let root = dir.path();
    write(
        root,
        "notice.txt",
        "# Proprietary.\n# All rights reserved.\n",
    );
    let notice = "# Proprietary.\n# All rights reserved.\n";
    let original = format!(
        "\u{feff}#!/usr/bin/env python3\n# coding: utf-8\n\n{}",
        source(year() - 2, OWNER, notice)
    )
    .replace('\n', "\r\n");
    let expected = original.replacen(
        &format!("Copyright (C) {},", year() - 2),
        &format!("Copyright (C) {}-{},", year() - 2, year()),
        1,
    );
    write(root, "src/stale.py", &original);
    let path = root.join("src/stale.py");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o754)).unwrap();
    }
    let result = json_run(root, &["fix", "--license-notice", "notice.txt"], 0);
    assert_eq!(result["changed"], json!(["src/stale.py"]));
    assert_eq!(fs::read(&path).unwrap(), expected.as_bytes());
    let modified = fs::metadata(&path).unwrap().modified().unwrap();
    let result = json_run(root, &["fix", "--license-notice", "notice.txt"], 0);
    assert_eq!(result["changed"], json!([]));
    assert_eq!(fs::read(&path).unwrap(), expected.as_bytes());
    assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), modified);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(path).unwrap().permissions().mode() & 0o777,
            0o754
        );
    }
}

#[test]
fn non_utf8_sources_keep_their_original_encoding() {
    let dir = workspace();
    let root = dir.path();
    let raw = format!(
        "# coding: latin-1\n\n{}",
        source(year() - 2, "André", NOTICE)
    );
    let bytes: Vec<u8> = raw.chars().map(|ch| ch as u8).collect();
    write(root, "src/latin.py", &bytes);
    let result = json_run(root, &["fix", "--owner", "André"], 0);
    assert_eq!(result["changed"], json!(["src/latin.py"]));
    let expected: Vec<u8> = raw
        .replacen(
            &format!("Copyright (C) {},", year() - 2),
            &format!("Copyright (C) {}-{},", year() - 2, year()),
            1,
        )
        .chars()
        .map(|ch| ch as u8)
        .collect();
    assert_eq!(fs::read(root.join("src/latin.py")).unwrap(), expected);
}

#[test]
fn nearest_configuration_cli_precedence_and_root_relative_paths() {
    let dir = workspace();
    let root = dir.path();
    write(root, "src/clean.py", header(year()));
    fs::create_dir_all(root.join("nested/deep")).unwrap();
    let nested = root.join("nested/deep");
    let configured = json_run(&nested, &["check"], 0);
    assert_eq!(configured["checked"], 1);
    assert_eq!(configured["config_path"], "pyproject.toml");
    write(root, "notice.txt", "# Proprietary.\n");
    write(
        root,
        "nested/deep/selected.py",
        source(year() - 2, "CLI Owner", "# Proprietary.\n"),
    );
    let result = json_run(
        &nested,
        &[
            "check",
            "--owner",
            "CLI Owner",
            "--starting-year",
            "2024",
            "--license-notice",
            "../../notice.txt",
            "selected.py",
        ],
        1,
    );
    assert_eq!(result["checked"], 1);
    assert_eq!(result["diagnostics"][0]["path"], "nested/deep/selected.py");
    assert_eq!(result["diagnostics"][0]["code"], "LMH004");
    write(
        root,
        "nested/pyproject.toml",
        CONFIG
            .replace("Example Owner", "Nested Owner")
            .replace("paths = ['src']", "paths = ['deep']"),
    );
    write(root, "nested/LICENSE", "Apache-2.0\n");
    let result = json_run(&nested, &["check"], 1);
    assert_eq!(result["diagnostics"][0]["path"], "deep/selected.py");
    assert_eq!(result["diagnostics"][0]["code"], "LMH002");
    let result = json_run(&nested, &["check", "--config", "../../pyproject.toml"], 0);
    assert_eq!(result["checked"], 1);
}

#[test]
fn configured_custom_notice_is_relative_to_the_config() {
    let dir = workspace();
    let root = dir.path();
    write(root, "notice.txt", "# Proprietary.\r\n");
    write(
        root,
        "pyproject.toml",
        CONFIG.replace("license = 'Apache-2.0'", "license-notice = 'notice.txt'"),
    );
    write(
        root,
        "src/clean.py",
        source(year(), OWNER, "# Proprietary.\n"),
    );
    fs::create_dir(root.join("nested")).unwrap();
    fs::remove_file(root.join("LICENSE")).unwrap();
    assert_eq!(json_run(&root.join("nested"), &["check"], 0)["checked"], 1);
}

#[test]
fn custom_notices_require_text_and_match_complete_lines() {
    let dir = workspace();
    let root = dir.path();
    let target = root.join("src/stale.py");
    let old_year = year() - 2;
    write(
        root,
        "pyproject.toml",
        CONFIG.replace("license = 'Apache-2.0'", "license-notice = 'notice.txt'"),
    );
    let stale = source(old_year, OWNER, "");
    fs::write(&target, &stale).unwrap();
    for notice in ["", " \t\r\n", "#\n# \r\n\n"] {
        write(root, "notice.txt", notice);
        for command in ["check", "fix"] {
            let result = json_run(root, &[command], 2);
            assert_eq!(result["error"]["code"], "LMH900");
            assert!(message(&result).contains("non-empty"));
            assert_eq!(result["checked"], 0);
            assert_eq!(result["changed"], json!([]));
            assert_eq!(fs::read_to_string(&target).unwrap(), stale);
        }
    }
    for (notice, accepted) in [
        ("Proprietary.", "# Proprietary.\n"),
        ("# Terms.\n# Proprietary.", "# Terms.\n# Proprietary."),
        ("Proprietary.\r\n", "# Proprietary.\r\n"),
    ] {
        write(root, "notice.txt", notice);
        let mismatch = accepted.replace("Proprietary.", "Proprietary. Different terms.");
        let mismatched = source(old_year, OWNER, &mismatch);
        fs::write(&target, &mismatched).unwrap();
        for command in ["check", "fix"] {
            let result = json_run(root, &[command], 1);
            assert_eq!(result["diagnostics"][0]["code"], "LMH005");
            assert_eq!(result["diagnostics"][0]["fixable"], false);
            assert_eq!(result["changed"], json!([]));
            assert_eq!(fs::read_to_string(&target).unwrap(), mismatched);
        }
        let valid = format!("# Copyright (C) {old_year}, {OWNER}.\n\n{accepted}");
        fs::write(&target, &valid).unwrap();
        assert_eq!(
            json_run(root, &["check"], 1)["diagnostics"][0]["code"],
            "LMH004"
        );
        assert_eq!(
            json_run(root, &["fix"], 0)["changed"],
            json!(["src/stale.py"])
        );
        assert_eq!(
            fs::read_to_string(&target).unwrap(),
            valid.replacen(&old_year.to_string(), &format!("{old_year}-{}", year()), 1)
        );
    }
}

#[test]
fn invalid_config_is_rejected_before_any_write() {
    let dir = workspace();
    let root = dir.path();
    let stale = header(year() - 2);
    write(root, "src/stale.py", &stale);
    for (key, value) in [
        ("owner", "true"),
        ("owner", "''"),
        ("starting-year", "true"),
        ("starting-year", "2022.0"),
        ("starting-year", "999"),
        ("license", "4"),
        ("license-notice", "false"),
        ("paths", "[]"),
        ("paths", "['']"),
        ("paths", "'src'"),
        ("languages", "[]"),
        ("languages", "['typo']"),
        ("languages", "'typescript'"),
        ("ignore-files", "[42]"),
        ("ignore-folders", "true"),
        ("unknown-key", "true"),
    ] {
        let filtered: Vec<_> = CONFIG
            .lines()
            .filter(|line| !line.starts_with(&format!("{key} =")))
            .collect();
        write(
            root,
            "pyproject.toml",
            format!("{}\n{key} = {value}\n", filtered.join("\n")),
        );
        let result = json_run(root, &["fix"], 2);
        assert_eq!(result["error"]["code"], "LMH900", "{key}={value}");
        assert!(
            message(&result).contains(&format!("[tool.lint-my-headers].{key}")),
            "{result}"
        );
        assert_eq!(result["changed"], json!([]));
        assert_eq!(
            fs::read_to_string(root.join("src/stale.py")).unwrap(),
            stale
        );
    }
    write(
        root,
        "pyproject.toml",
        format!("{CONFIG}license-notice = 'notice.txt'\n"),
    );
    assert!(
        json_run(root, &["fix"], 2)["error"]["message"]
            .as_str()
            .unwrap()
            .contains("exactly one")
    );
    write(root, "pyproject.toml", CONFIG);
    for args in [
        vec!["fix", "--folders", "src", "src/stale.py"],
        vec!["fix", "--license", "made-up-license"],
        vec!["fix", "--config", "missing.toml"],
        vec!["fix", "--owner", "bad\nowner"],
    ] {
        assert_eq!(json_run(root, &args, 2)["changed"], json!([]));
    }
    let missing = json_run(root, &["check", "--config", "missing.toml"], 2);
    json_run(
        root,
        &["fix", "--starting-year", &(year() + 1).to_string()],
        2,
    );
    json_run(root, &["fix", "--license", ""], 2);
    assert_eq!(missing["config_path"], "missing.toml");
    assert_eq!(missing["error"]["path"], "missing.toml");
    assert_eq!(
        fs::read_to_string(root.join("src/stale.py")).unwrap(),
        stale
    );
}

#[test]
fn discovery_deduplicates_ignores_and_sorts_external_paths() {
    let outer = tempdir().unwrap();
    let root = outer.path().join("project");
    fs::create_dir(&root).unwrap();
    write(&root, "LICENSE", "Apache-2.0\n");
    write(&root, "pyproject.toml", CONFIG);
    for file in ["src/a.py", "src/z.py", "src/ignored.py", "src/skip/bad.py"] {
        write(&root, file, "value = 1\n");
    }
    write(outer.path(), "external.py", "value = 1\n");
    let result = json_run(
        &root,
        &[
            "check",
            "--ignore-files",
            "ignored.py",
            "--ignore-folders",
            "src/skip",
            "src",
            "src/a.py",
            "../external.py",
        ],
        1,
    );
    assert_eq!(result["checked"], 3);
    let paths: Vec<_> = result["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["path"].as_str().unwrap())
        .collect();
    assert_eq!(paths, ["../external.py", "src/a.py", "src/z.py"]);
}

#[cfg(windows)]
#[test]
fn display_paths_match_verbatim_and_regular_prefixes() {
    for (root, path, expected) in [
        (r"\\?\C:\repo", r"C:\repo\src\a.py", "src/a.py"),
        (r"C:\repo", r"\\?\C:\external.py", "../external.py"),
        (
            r"\\?\UNC\server\share\repo",
            r"\\server\share\repo\src\a.py",
            "src/a.py",
        ),
    ] {
        assert_eq!(
            lint_my_headers::display_path(Path::new(path), Path::new(root)),
            expected
        );
    }
}

#[test]
fn hardlinked_targets_are_readable_but_never_fixed() {
    let dir = workspace();
    let root = dir.path();
    let stale = header(year() - 2);
    write(root, "src/first.py", &stale);
    fs::hard_link(root.join("src/first.py"), root.join("src/second.py")).unwrap();
    let check = json_run(root, &["check", "src/first.py"], 1);
    assert_eq!(check["diagnostics"][0]["code"], "LMH004");
    assert_eq!(check["diagnostics"][0]["fixable"], false);
    let fixed = json_run(root, &["fix"], 1);
    assert_eq!(fixed["changed"], json!([]));
    for diagnostic in fixed["diagnostics"].as_array().unwrap() {
        assert_eq!(diagnostic["code"], "LMH008");
    }
    for path in ["src/first.py", "src/second.py"] {
        assert_eq!(fs::read_to_string(root.join(path)).unwrap(), stale);
    }
}

#[test]
fn large_trees_keep_ordered_findings_and_safe_year_only_repairs() {
    let dir = workspace();
    let root = dir.path();
    let originals: Vec<_> = (0..300)
        .map(|i| {
            let bytes = match i % 4 {
                0 => header(year()).into_bytes(),
                1 => header(year() - 2).into_bytes(),
                2 => source(year(), "Other", NOTICE).into_bytes(),
                _ => [header(year()).as_bytes(), b"\xff"].concat(),
            };
            let path = format!("src/{i:03}.py");
            write(root, &path, &bytes);
            (path, bytes)
        })
        .collect();
    fs::hard_link(root.join("src/001.py"), root.join("src/linked.py")).unwrap();
    let check = json_run(root, &["check"], 1);
    assert_eq!(check["checked"], 301);
    assert_eq!(check, json_run(root, &["check"], 1));
    let diagnostics = check["diagnostics"].as_array().unwrap();
    assert!(
        diagnostics
            .windows(2)
            .all(|pair| pair[0]["path"].as_str() < pair[1]["path"].as_str())
    );
    for (path, bytes) in &originals {
        assert_eq!(fs::read(root.join(path)).unwrap(), *bytes);
    }
    for path in ["src/001.py", "src/linked.py"] {
        assert!(
            !diagnostics.iter().find(|d| d["path"] == path).unwrap()["fixable"]
                .as_bool()
                .unwrap()
        );
    }
    let fixed = json_run(root, &["fix"], 1);
    let changed: Vec<_> = originals
        .iter()
        .enumerate()
        .filter(|(i, _)| i % 4 == 1 && *i != 1)
        .map(|(_, (path, _))| path)
        .collect();
    assert_eq!(fixed["changed"], json!(changed));
    assert_eq!(
        fixed["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|d| d["code"] == "LMH008")
            .count(),
        2
    );
    for (i, (path, bytes)) in originals.iter().enumerate() {
        let expected = if i % 4 == 1 && i != 1 {
            header(format!("{}-{}", year() - 2, year())).into_bytes()
        } else {
            bytes.clone()
        };
        assert_eq!(fs::read(root.join(path)).unwrap(), expected);
    }
}

#[test]
fn parallel_repairs_refuse_duplicate_file_identities() {
    let dir = workspace();
    let root = dir.path();
    fs::create_dir(root.join("src/deep")).unwrap();
    for i in 0..300 {
        write(root, &format!("src/{i:03}.py"), header(year() - 2));
    }
    // Windows resolves `..` in absolute discovery keys before identity reservation.
    let windows = cfg!(windows);
    let result = json_run(
        root,
        &["fix", "src", "src/deep/../075.py"],
        i32::from(!windows),
    );
    assert_eq!(result["checked"], if windows { 300 } else { 301 });
    assert_eq!(result["changed"].as_array().unwrap().len(), 300);
    assert_eq!(
        result["diagnostics"].as_array().unwrap().len(),
        usize::from(!windows)
    );
    if !windows {
        assert_eq!(result["diagnostics"][0]["path"], "src/075.py");
        assert_eq!(result["diagnostics"][0]["code"], "LMH008");
        assert_eq!(result["diagnostics"][0]["fixable"], false);
    }
    for i in 0..300 {
        assert_eq!(
            fs::read(root.join(format!("src/{i:03}.py"))).unwrap(),
            header(format!("{}-{}", year() - 2, year())).as_bytes()
        );
    }
}

#[cfg(unix)]
#[test]
fn parallel_repair_errors_report_every_completed_write() {
    use std::os::unix::{
        fs::{MetadataExt, PermissionsExt},
        process::CommandExt,
    };
    let dir = workspace();
    let root = dir.path();
    let stale = header(year() - 2);
    let clean = header(format!("{}-{}", year() - 2, year()));
    for i in 0..300 {
        write(root, &format!("src/{i:03}.py"), &stale);
    }
    write(root, "src/000/blocked.py", &stale);
    let outputs = root.join("outputs");
    fs::write(&outputs, "").unwrap();
    for (path, mode) in [
        (root.to_path_buf(), 0o755),
        (root.join("src"), 0o777),
        (root.join("src/000"), 0o555),
        (outputs.clone(), 0o666),
    ] {
        fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
    }
    let mut command = Command::new(env!("CARGO_BIN_EXE_lmh"));
    command
        .current_dir(root)
        .env("GITHUB_OUTPUT", &outputs)
        .args(["fix", "--output-format", "json"]);
    if fs::metadata(root).unwrap().uid() == 0 {
        command.uid(65534).gid(65534);
    }
    let output = command.output().unwrap();
    fs::set_permissions(root.join("src/000"), fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["checked"], 301);
    assert_eq!(result["error"]["path"], "src/000/blocked.py");
    let mut changed = Vec::new();
    for i in 0..300 {
        let path = format!("src/{i:03}.py");
        let bytes = fs::read(root.join(&path)).unwrap();
        if bytes == clean.as_bytes() {
            changed.push(path);
        } else {
            assert_eq!(bytes, stale.as_bytes());
        }
    }
    assert!(!changed.is_empty());
    assert_eq!(result["changed"], json!(changed));
    assert_eq!(
        fs::read(root.join("src/000/blocked.py")).unwrap(),
        stale.as_bytes()
    );
    let outputs = fs::read_to_string(outputs).unwrap();
    assert!(outputs.lines().any(|line| line == "issues=[]"));
    let changed: Value = serde_json::from_str(
        outputs
            .lines()
            .find_map(|line| line.strip_prefix("changed="))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(changed, result["changed"]);
}

#[cfg(any(unix, windows))]
#[test]
fn symlinks_are_skipped_during_discovery_and_refused_during_fix() {
    let dir = workspace();
    let root = dir.path();
    let stale = header(year() - 2);
    write(root, "outside/target.py", &stale);
    let target = root.join("outside/target.py");
    let link = root.join("src/link.py");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&target, &link).unwrap();
    #[cfg(windows)]
    if let Err(error) = std::os::windows::fs::symlink_file(&target, &link) {
        if error.raw_os_error() == Some(1314) {
            return;
        }
        panic!("{error}");
    }
    assert_eq!(json_run(root, &["check"], 0)["checked"], 0);
    let check = json_run(root, &["check", "src/link.py"], 1);
    assert_eq!(check["diagnostics"][0]["code"], "LMH004");
    assert_eq!(check["diagnostics"][0]["fixable"], false);
    let fixed = json_run(root, &["fix", "src/link.py"], 1);
    assert_eq!(fixed["diagnostics"][0]["code"], "LMH008");
    assert_eq!(fs::read_to_string(&target).unwrap(), stale);
    assert!(
        fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    let linked_root = root.join("linked-root");
    #[cfg(unix)]
    std::os::unix::fs::symlink(root.join("outside"), &linked_root).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(root.join("outside"), &linked_root).unwrap();
    assert_eq!(json_run(root, &["check", "linked-root"], 2)["checked"], 0);
    let result = json_run(root, &["fix", "linked-root/target.py"], 1);
    assert_eq!(result["diagnostics"][0]["code"], "LMH008");
    assert_eq!(fs::read_to_string(target).unwrap(), stale);
}

#[test]
fn pinned_spdx_data_accepts_every_legacy_notice_and_new_identifiers() {
    use sha2::{Digest, Sha256};
    let snapshot = include_bytes!("../src/lint_my_headers/supported-licenses.json");
    assert_eq!(
        format!("{:x}", Sha256::digest(snapshot)),
        "f728c534d8bd1044fc515a2ddb2292be99559021d830bfa3281be0bcd36302ee"
    );
    let current: Value = serde_json::from_slice(snapshot).unwrap();
    assert_eq!(current["licenseListVersion"], "3.28.0");
    assert_eq!(current["licenses"].as_array().unwrap().len(), 727);
    let legacy: Value = serde_json::from_str(include_str!(
        "../src/lint_my_headers/legacy-license-notices.json"
    ))
    .unwrap();
    assert_eq!(legacy["baseline_version"], "3.17");
    let licenses = legacy["licenses"].as_object().unwrap();
    assert_eq!(licenses.len(), 29);
    let dir = workspace();
    let root = dir.path();
    for (identifier, entry) in licenses {
        for url in entry["urls"].as_array().unwrap() {
            let notice = format!(
                "# This program is licensed under the {}.\n# See LICENSE or go to <{}> for full license details.\n",
                entry["name"].as_str().unwrap(),
                url.as_str().unwrap()
            );
            write(root, "src/valid.py", source(year(), OWNER, &notice));
            let result = json_run(root, &["check", "--license", identifier], 0);
            assert_eq!(result["checked"], 1, "{identifier}");
        }
    }
    let new = current["licenses"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["licenseId"] == "3D-Slicer-1.0")
        .unwrap();
    let notice = format!(
        "# This program is licensed under the {}.\n# See LICENSE or go to <{}> for full license details.\n",
        new["name"].as_str().unwrap(),
        new["seeAlso"][0].as_str().unwrap()
    );
    write(root, "src/valid.py", source(year(), OWNER, &notice));
    let result = json_run(
        root,
        &[
            "check",
            "--license",
            "3D-Slicer-1.0",
            "--starting-year",
            &year().to_string(),
        ],
        0,
    );
    assert_eq!(result["checked"], 1);
    assert!(
        result["expected_header"]
            .as_str()
            .unwrap()
            .starts_with(&format!("# Copyright (C) {}, {OWNER}.", year()))
    );
}
