---
description: Configure explicit ownership and license policies, select languages and paths, and understand supported header layouts.
---

# Configuration

One explicit policy for your source files. Choose the languages, paths, and notice that belong to your project.

## Configuration files

Discovery searches upward from the invocation directory and selects the nearest
applicable policy. Within each directory, priority is:

| Priority | File | Policy location |
| --- | --- | --- |
| 1 | `.lmh.toml` | Top-level keys. |
| 2 | `pyproject.toml` | `[tool.lint-my-headers]`. |
| 3 | `Cargo.toml` | `[package.metadata.lint-my-headers]` or `[workspace.metadata.lint-my-headers]`. Package settings take precedence. |
| 4 | `package.json` | A `"lint-my-headers"` object. |

Manifests without LMH settings are skipped. Invalid configurations fail;
configurations are never merged. To select an exact file:

```shell
lmh check --config Cargo.toml
```

Other explicit TOML filenames use the `[tool.lint-my-headers]` section.
Configured paths are relative to the policy file. Positional CLI paths are
relative to the invocation directory. CLI policy options override file settings.

### Rust example

```toml
[package.metadata.lint-my-headers]
owner = "Example Organization"
starting-year = 2024
license = "Apache-2.0"
languages = ["rust"]
paths = ["src", "tests"]
ignore-folders = ["target"]
```

For a virtual workspace, use `[workspace.metadata.lint-my-headers]`. Go modules,
SwiftPM/Xcode projects, and shell or C/C++ projects can use `.lmh.toml`;
`go.mod`, `go.work`, and `Package.swift` do not supply header policy.

## Options

| Key / CLI option | Meaning and default |
| --- | --- |
| `owner` / `--owner` | Required exact, single-line copyright owner. |
| `starting-year` / `--starting-year` | Required earliest accepted file creation year. |
| `creation-year` / `--creation-year` | Unreleased source only. Optional declared first year for inserting missing headers; default unset. Applies to all selected missing headers, not existing ones. |
| `license` / `--license` | SPDX identifier selecting a prose notice; requires a local `LICENSE`. |
| `license-notice` / `--license-notice` | Custom notice file; configure exactly one license source. |
| `paths` / positional paths | Selected files or directories; default `.`. |
| `languages` / `--languages` | Non-empty language allowlist; default `["python"]`. |
| `ignore-files` / `--ignore-files` | Exact excluded basenames; default `["__init__.py"]`. |
| `ignore-folders` / `--ignore-folders` | Excluded subtrees; default `[".github"]`. |

Select files with the same actual creation year when inserting headers. Run
separately for each year; one shared `creation-year` does not describe mixed-age files.

Configuration lists are arrays. CLI language and ignore lists are comma-separated
and replace the corresponding configured list:

```shell
lmh check --languages typescript,rust --ignore-folders node_modules,target
```

Exclusions apply to explicit inputs too. Limit paths or exclude dependency/build
directories such as `node_modules`, `target`, `vendor`, and `.build`; excluded
subtrees are not traversed. The tool does not infer exclusions from `.gitignore`.

## Supported languages

Only enabled, supported extensions are checked, including for explicit files.
Unknown language names and empty language lists fail.

| Selector | Extensions | Header marker |
| --- | --- | --- |
| `python` | `.py` | `#` |
| `javascript` | `.js`, `.jsx`, `.mjs`, `.cjs` | `//` |
| `typescript` | `.ts`, `.tsx`, `.mts`, `.cts`, including declarations | `//` |
| `rust` | `.rs` | `//` |
| `go` | `.go`, including test and platform files | `//` |
| `swift` | `.swift`, including `Package.swift` | `//` |
| `bash` (alias `shell`) | `.sh`, `.bash` | `#` |
| `c` | `.c`, `.h` | `//` |
| `cpp` (alias `c++`) | `.cc`, `.cpp`, `.cxx`, `.c++`, `.C`, `.hh`, `.hpp`, `.hxx`, `.h++`, `.H`, `.ipp`, `.tpp`, `.inl`, `.h` | `//` |

Shared `.h` headers are checked with either C selector, using C first and falling
back to C++ when needed. Other extensions and extensionless shebang scripts are
skipped. Files are checked regardless of build tags or platform suffixes.

## Header layouts

Use the [example header](getting-started.md#check-existing-headers) with the
appropriate comment marker and blank lines. A custom `license-notice` file can
contain plain text; existing Python-commented notice files remain accepted.
Custom notices must contain non-whitespace text after removing Python comment
markers. Blank notices fail with exit 2 before source files are checked or repaired.
Notice lines must match in full, including the final line when the notice file has
no trailing newline.

### Additional layouts (unreleased)

These layouts require the source checkout. Published **0.7.0** uses the prose
layout above. Existing-header repairs still change only the end year.

| Layout | Rules |
| --- | --- |
| Prose | `Copyright`, `Copyright (C)`, or `Copyright (c)`, followed by a four-digit year/range and the exact owner. Comma and final period are optional. Keep one blank or marker-only line before the matching notice. |
| Ordinary block | Languages using `//` also accept the whole header inside `/* ... */`. Leading `*` markers are optional; the closing delimiter may follow the final notice. |
| SPDX pair | One `SPDX-FileCopyrightText` and one `SPDX-License-Identifier`, in either order with optional blank lines. Owner and configured identifier must match exactly. Keep the local `LICENSE`. |

For example, with `owner = "Example Organization"` and `license = "Apache-2.0"`:

```c
/*
 * SPDX-FileCopyrightText: 2024-2026 Example Organization
 * SPDX-License-Identifier: Apache-2.0
 */
```

The pair also works in line comments: `#` for Python/Bash, `//` for other
languages. A prose copyright with an SPDX identifier is accepted, as is a full
prose notice followed by a matching identifier. Custom notices must match in
full, including text after any SPDX tag; compound expressions are not evaluated.

SPDX-only blocks contain only the pair and blank lines. Separate later
line-comment notes with a blank line. Conflicting legal text anywhere in the
leading comments, duplicate fields, nested legal blocks, and documentation
comment headers refuse repair. Multiple holders, omitted years, sidecars, and
`REUSE.toml` are unsupported. LMH checks declared policy, not legal compliance.

??? note "Python"
    UTF-8 BOM, shebang, and PEP 263 cookie are preserved. Verified encodings:
    UTF-8, ASCII, Latin-1, and Windows-1252. Other codecs fail without repair.

??? note "JavaScript / TypeScript"
    UTF-8 BOM, shebang, CRLF, and body are preserved. A shebang needs a blank
    separator before the header. Bare CR headers and Unicode line separators
    are refused.

??? note "Rust"
    UTF-8 BOM, shebang, and newline style are preserved. Use ordinary `//`
    headers; crate attributes follow the header. Shebangs need a blank separator;
    ambiguous comment-prefixed `#!` forms are refused.

??? note "Go"
    UTF-8 BOM and CRLF are preserved. Leading `//go:build` and `// +build`
    directives need a blank separator; constraints after the header are also accepted.

??? note "Swift"
    UTF-8 BOM, CRLF, and shebang are preserved. A leading
    `// swift-tools-version:` stays first, with a blank separator before the header.

??? note "Bash / shell"
    UTF-8 BOM, CRLF, shebang, and executable permissions are preserved.
    A shebang needs a blank separator; other shell dialects are skipped.

??? note "C / C++"
    UTF-8 BOM, CRLF, and body are preserved. Put include guards and
    `#pragma once` after the header.

Validation stops at the first code line after allowed preambles and leading
comments/blank lines. Later copyright notices and body syntax errors are ignored;
the entire file must still decode successfully. Duplicate notices and
copyright-bearing documentation comments in the leading region refuse repair.
Published 0.7.0 also refuses copyright-bearing ordinary blocks; the unreleased
layouts above support unambiguous ordinary blocks.
Directory discovery skips symlinks and reparse points. Explicit linked
files may be checked, but repairs refuse linked files/parents, multiple hard
links, and concurrently changed targets.

The bundled SPDX snapshot is **v3.28.0**, retaining previously accepted v3.17
names/URLs, including `KiCad-libraries-exception`. Compound SPDX expressions and
new exception semantics are unsupported. Selecting a notice does not establish
legal or SPDX compliance.
